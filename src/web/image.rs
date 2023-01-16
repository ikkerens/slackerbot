use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse, Responder,
};
use entity::{prelude::Quote, quote};
use sea_orm::{DatabaseConnection, EntityTrait, FromQueryResult, QuerySelect};

#[derive(FromQueryResult)]
struct ImageRow {
    pub attachment: Vec<u8>,
}

#[get("/image/{id}/{name}")]
pub(super) async fn page(id: Path<(u64, String)>, db: Data<DatabaseConnection>) -> impl Responder {
    let quote = Quote::find_by_id(id.into_inner().0 as i64)
        .select_only()
        .column(quote::Column::Attachment)
        .into_model::<ImageRow>()
        .one(db.as_ref())
        .await
        .unwrap();
    match quote {
        Some(image) => HttpResponse::Ok().body(image.attachment),
        None => HttpResponse::NotFound().body("Image not found"),
    }
}
