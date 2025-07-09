use axum::{Router, routing::get};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

mod config;
mod error;
mod models;
mod services;
mod routes;
mod services;
mod routes;
mod logger;

#[tokio::main]
async fn main() {
    let cfg = config::Config::from_env().unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(100) 
        .connect(&cfg.database_url)
        .await
        .unwrap();

    let redis = redis::Client::open(cfg.redis_url).unwrap();

    // 服务
    let shortener = Arc::new(services::ShortenerService::new(pool, redis));

    // 路由
    let app = Router::new()
        .route("/:id", get(routes::redirect::redirect))
        .route("/api/shorten", post(routes::api::shorten))
        .with_state(shortener);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
