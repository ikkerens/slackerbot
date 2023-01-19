use actix_web::{
    get,
    web::{Data, Path},
    HttpRequest, HttpResponse, Responder,
};
use sea_orm::{DatabaseConnection, EntityTrait, FromQueryResult, QuerySelect};

use entity::{prelude::Quote, quote};

use crate::web::auth;

#[derive(FromQueryResult)]
struct ImageRow {
    pub attachment: Vec<u8>,
}

#[get("/image/{id}/{name}")]
pub(super) async fn page(
    req: HttpRequest,
    auth: Data<auth::Client>,
    id: Path<(u64, String)>,
    db: Data<DatabaseConnection>,
) -> impl Responder {
    if let Some(response) = auth.verify(req).await {
        return response;
    }

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
