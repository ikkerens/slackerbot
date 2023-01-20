use std::{collections::BTreeMap, sync::Arc};

use actix_web::{cookie::Cookie, get, web, web::Data, HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use hmac::{digest::KeyInit, Hmac};
use jwt::{SignWithKey, VerifyWithKey};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::StatusCode;
use serde::Deserialize;
use serenity::{
    model::{
        id::{GuildId, UserId},
        user::User,
    },
    CacheAndHttp,
};
use sha2::Sha256;

#[derive(Clone)]
pub(crate) struct Client {
    oauth: BasicClient,
    key: Hmac<Sha256>,
    discord: Arc<CacheAndHttp>,
    web_whitelist_guild_id: GuildId,
}

impl Client {
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_url: String,
        cookie_secret: String,
        discord: Arc<CacheAndHttp>,
        web_whitelist_guild_id: GuildId,
    ) -> Result<Self> {
        let oauth = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://discord.com/oauth2/authorize".to_string())?,
            Some(TokenUrl::new("https://discord.com/api/oauth2/token".to_string())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url)?);

        let key: Hmac<Sha256> = Hmac::new_from_slice(cookie_secret.as_bytes())?;

        Ok(Self { oauth, key, discord, web_whitelist_guild_id })
    }

    pub async fn verify(&self, req: HttpRequest) -> Option<HttpResponse> {
        let Some(cookie) = req.cookie("token") else { return Some(self.generate_login_redirect()) };
        let Ok(claims): Result<BTreeMap<String, String>, _> = cookie.value().verify_with_key(&self.key) else { return Some(self.generate_login_redirect()) };
        let Some(user_id_str) = claims.get("user_id") else { return Some(self.generate_login_redirect()) };
        let Ok(user_id) = user_id_str.parse::<UserId>() else {return Some(self.generate_login_redirect())};

        if self.web_whitelist_guild_id.member(&self.discord, user_id).await.is_err() {
            return Some(HttpResponse::TemporaryRedirect().insert_header(("Location", "/bad")).body(""));
        }

        None
    }

    fn generate_login_redirect(&self) -> HttpResponse {
        let (auth_url, csrf) =
            self.oauth.authorize_url(CsrfToken::new_random).add_scope(Scope::new("identify".to_string())).url();
        HttpResponse::TemporaryRedirect()
            .insert_header(("Location", auth_url.to_string()))
            .cookie(Cookie::new("token_csrf", csrf.secret()))
            .body("Redirecting...")
    }
}

#[derive(Deserialize)]
pub struct OAuthResponse {
    state: String,
    code: String,
}

#[derive(Deserialize, Debug)]
pub struct MeResponse {
    user: Option<User>,
}

#[get("/oauth/redirect")]
pub(super) async fn oauth_redirect(
    req: HttpRequest,
    auth: Data<Client>,
    response: web::Query<OAuthResponse>,
) -> HttpResponse {
    let Some(mut csrf_cookie) = req.cookie("token_csrf") else { return bad_request("csrf_cookie") };
    if csrf_cookie.value() != response.state {
        return bad_request("csrf_mismatch");
    };

    let Ok(token_result) = auth.oauth.exchange_code(AuthorizationCode::new(response.code.clone()))
        .request_async(async_http_client).await else { return bad_request("token_fail") };
    let Ok(me_response) = reqwest::Client::new().get("https://discord.com/api/oauth2/@me").header("Authorization", format!("Bearer {}", token_result.access_token().secret()))
        .send().await else { return bad_request("me_fail") };
    if me_response.status() != StatusCode::OK {
        return bad_request(&format!("discord_fail: {}", me_response.status()));
    }
    let Ok(me_response) = me_response.json::<MeResponse>().await else { return bad_request("json_fail") };
    let Some(user) = me_response.user else {return bad_request("user_fail");};

    let mut claims = BTreeMap::new();
    claims.insert("user_id", user.id.to_string());
    let Ok(token) = claims.sign_with_key(&auth.key) else { return bad_request("jwt_fail") };

    let mut cookie = Cookie::new("token", token);
    cookie.make_permanent();
    cookie.set_path("/");
    csrf_cookie.make_removal();
    HttpResponse::TemporaryRedirect()
        .insert_header(("Location", "/"))
        .cookie(cookie)
        .cookie(csrf_cookie)
        .body("Redirecting...")
}

#[get("/bad")]
pub(super) async fn unauthorized() -> impl Responder {
    HttpResponse::Unauthorized()
        .body(r#"You are not a member of the guild. <a href="/logout">Log out and try again with another account?</a>"#)
}

#[get("/logout")]
pub(super) async fn logout() -> impl Responder {
    let mut cookie = Cookie::new("token", "");
    cookie.make_removal();
    HttpResponse::TemporaryRedirect().insert_header(("Location", "/")).cookie(cookie).body("")
}

fn bad_request(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest().body(format!("Bad request: {}", msg))
}
