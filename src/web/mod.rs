use actix_files::Files;
use actix_web::{web::Data, App, HttpServer};
use anyhow::Result;
use sea_orm::DatabaseConnection;
use tera::Tera;

mod image;
mod index;

pub(crate) fn start(db: DatabaseConnection) -> Result<()> {
    let tera = Tera::new("web/templates/**/*.html")?;
    let actix = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(db.clone()))
            .app_data(Data::new(tera.clone()))
            .service(index::page)
            .service(image::page)
            .service(Files::new("/", "./web/static"))
    })
    .bind(("0.0.0.0", 8080))?;
    tokio::spawn(actix.run());
    Ok(())
}
