use crate::routes::{health_check, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgConnection;
use std::net::TcpListener;

/// Implement the main logic of the program
pub fn run(listener: TcpListener, conn: PgConnection) -> Result<Server, std::io::Error> {
    let conn = web::Data::new(conn);
    Ok(HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(conn.clone())
    })
    .listen(listener)?
    .run())
}
