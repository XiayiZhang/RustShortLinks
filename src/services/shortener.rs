use nanoid::nanoid;
use sqlx::PgPool;
use redis::Client;
pub struct ShortenerService {
    pool: PgPool,
    cache: Client,
}

impl ShortenerService {
    pub fn new(pool: PgPool, cache: redis::Client) -> Self {
        Self { pool, cache }
    }

    // 生成短链（6位 ID）
    pub async fn shorten(&self, original_url: String) -> Result<String, crate::error::AppError> {
        let id = nanoid!(6); // 例如 "aBcD12"
        let mut conn = self.pool.acquire().await?;

        sqlx::query!(
            "INSERT INTO urls (id, original_url) VALUES ($1, $2)",
            id,
            original_url
        )
            .execute(&mut conn)
            .await?;

        // 写入 Redis 缓存（TTL 30天）
        let mut cache_conn = self.cache.get_async_connection().await?;
        redis::cmd("SET")
            .arg(&id)
            .arg(&original_url)
            .arg("EX")
            .arg(30 * 24 * 60 * 60) // 30天过期
            .query_async(&mut cache_conn)
            .await?;

        Ok(id)
    }

    // 从缓存或数据库查询原始 URL
    pub async fn resolve(&self, id: String) -> Result<String, crate::error::AppError> {
        // 优先查 Redis
        let mut cache_conn = self.cache.get_async_connection().await?;
        let cached_url: Option<String> = redis::cmd("GET")
            .arg(&id)
            .query_async(&mut cache_conn)
            .await?;

        if let Some(url) = cached_url {
            return Ok(url);
        }

        // 查数据库
        let mut conn = self.pool.acquire().await?;
        let url = sqlx::query!(
            "SELECT original_url FROM urls WHERE id = $1",
            id
        )
            .fetch_one(&mut conn)
            .await?;

        Ok(url.original_url)
    }
}