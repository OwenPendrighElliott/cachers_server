mod errors;
mod handlers;
mod request_types;
mod state;

use actix_web::{web, App, HttpServer};
use state::AppState;
use std::collections::HashMap;
use std::sync::Mutex;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        caches: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/cache/create", web::post().to(handlers::create_cache))
            .route("/cache/delete", web::post().to(handlers::delete_cache))
            .route("/cache/{cache_name}/stats", web::get().to(handlers::stats))
            .route(
                "/cache/{cache_name}/{key}",
                web::get().to(handlers::get_value),
            )
            .route(
                "/cache/{cache_name}/{key}",
                web::put().to(handlers::set_value),
            )
            .route(
                "/cache/{cache_name}/{key}",
                web::delete().to(handlers::delete_value),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
