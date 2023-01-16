use actix_web::{get, web::Data, HttpResponse, Responder};
use entity::{prelude::Quote, quote};
use sea_orm::{prelude::DateTimeWithTimeZone, DatabaseConnection, EntityTrait, FromQueryResult, QuerySelect};
use tera::{Context, Tera};

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
pub(super) async fn page(tera: Data<Tera>, db: Data<DatabaseConnection>) -> impl Responder {
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
    let mut ctx = Context::new();
    ctx.insert("quotes", &quotes);
    let rendered = tera.render("index.html", &ctx).unwrap();
    HttpResponse::Ok().body(rendered)
}
