mod shortener;
mod cache;

pub use shortener::ShortenerService;

#[derive(Debug)]
pub struct ServiceError {
    pub message: String,
}

pub async fn init_services() -> (ShortenerService, CacheService) {
    let redis_url = "redis://127.0.0.1:6379"; 
    let db_url = "postgres://user:pass@localhost/db"; 

    let cache = CacheService::new(redis_url)
        .expect("Failed to create cache service");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .expect("Failed to create DB pool");

    (
        ShortenerService::new(pool, cache.clone()),
        cache
    )
}
