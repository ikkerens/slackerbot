use actix_files::Files;
use actix_web::{web::Data, App, HttpServer};
use anyhow::Result;
use chrono::Local;
use handlebars::{handlebars_helper, html_escape, Handlebars};
use sea_orm::{prelude::DateTimeWithTimeZone, DatabaseConnection};

mod image;
mod index;

pub(crate) fn start(db: DatabaseConnection) -> Result<()> {
    let mut handlebars = Handlebars::new();
    #[cfg(debug_assertions)]
    handlebars.set_dev_mode(true);
    handlebars.set_strict_mode(true);
    handlebars.register_helper("dateformat", Box::new(dateformat));
    handlebars.register_helper("htmlescape", Box::new(htmlescape));
    handlebars.register_templates_directory(".hbs", "web/templates/")?;

    let actix = HttpServer::new(move || {
        App::new()
            .app_data(Data::new(db.clone()))
            .app_data(Data::new(handlebars.clone()))
            .service(index::page)
            .service(image::page)
            .service(Files::new("/", "./web/static"))
    })
    .bind(("0.0.0.0", 8080))?;

    tokio::spawn(actix.run());
    Ok(())
}

handlebars_helper!(dateformat: |v: DateTimeWithTimeZone| format!("{}", v.with_timezone(&Local).format("%d-%m-%Y %H:%M")));
handlebars_helper!(htmlescape: |v: String| html_escape(&v));
