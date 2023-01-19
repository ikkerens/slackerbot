use actix_web::{get, web::Data, HttpRequest, HttpResponse};
use sea_orm::{prelude::DateTimeWithTimeZone, DatabaseConnection, EntityTrait, FromQueryResult, QuerySelect};
use serenity::json::json;

use entity::{prelude::Quote, quote};

use crate::web::auth;

#[derive(serde::Serialize, FromQueryResult)]
struct ListQuote {
    pub id: i64,
    pub channel_name: String,
    pub author: String,
    pub timestamp: DateTimeWithTimeZone,
    pub text: String,
    pub attachment_name: Option<String>,
}

#[get("/")]
pub(super) async fn page(
    req: HttpRequest,
    auth: Data<auth::Client>,
    handlebars: Data<handlebars::Handlebars<'_>>,
    db: Data<DatabaseConnection>,
) -> HttpResponse {
    if let Some(response) = auth.verify(req).await {
        return response;
    }

    let quotes = Quote::find()
        .select_only()
        .column(quote::Column::Id)
        .column(quote::Column::Author)
        .column(quote::Column::Text)
        .column(quote::Column::ChannelName)
        .column(quote::Column::Timestamp)
        .column(quote::Column::AttachmentName)
        .into_model::<ListQuote>()
        .all(db.get_ref())
        .await
        .unwrap();
    let rendered = handlebars.render("index", &json!({ "quotes": quotes })).unwrap();
    HttpResponse::Ok().body(rendered)
}
