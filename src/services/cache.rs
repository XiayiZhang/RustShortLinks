use redis::{Client, AsyncCommands, RedisError};
use std::time::Duration;
use tracing::{info, warn};

/// Redis 缓存服务
#[derive(Clone)]
pub struct CacheService {
    client: Client,
}

impl CacheService {
    /// 创建新的缓存服务实例
    pub fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = Client::open(redis_url)?;
        info!("Redis client created for URL: {}", redis_url);
        Ok(Self { client })
    }

    /// 获取异步连接（带连接池管理）
    async fn get_conn(&self) -> Result<redis::aio::Connection, RedisError> {
        self.client.get_async_connection().await
    }

    /// 设置键值对（带过期时间）
    pub async fn set(
        &self,
        key: &str,
        value: &str,
        ttl_seconds: usize,
    ) -> Result<(), RedisError> {
        let mut conn = self.get_conn().await?;
        conn.set_ex(key, value, ttl_seconds).await?;
        Ok(())
    }

    /// 获取键值
    pub async fn get(&self, key: &str) -> Result<Option<String>, RedisError> {
        let mut conn = self.get_conn().await?;
        conn.get(key).await
    }

    /// 批量获取键值（管道化操作提升性能）
    pub async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<String>>, RedisError> {
        let mut conn = self.get_conn().await?;
        redis::cmd("MGET").arg(keys).query_async(&mut conn).await
    }

    /// 删除键
    pub async fn del(&self, key: &str) -> Result<(), RedisError> {
        let mut conn = self.get_conn().await?;
        conn.del(key).await?;
        Ok(())
    }

    /// 健康检查
    pub async fn health_check(&self) -> bool {
        match self.get_conn().await {
            Ok(mut conn) => match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                Ok(pong) => pong == "PONG",
                Err(e) => {
                    warn!("Redis health check failed: {}", e);
                    false
                }
            },
            Err(e) => {
                warn!("Redis connection failed: {}", e);
                false
            }
        }
    }

    /// 带重试的获取操作（提高可用性）
    pub async fn get_with_retry(
        &self,
        key: &str,
        max_retries: u8,
        retry_delay: Duration,
    ) -> Result<Option<String>, RedisError> {
        let mut retries = 0;
        loop {
            match self.get(key).await {
                Ok(val) => return Ok(val),
                Err(e) if retries < max_retries => {
                    retries += 1;
                    tokio::time::sleep(retry_delay).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::RedisResult;

    async fn create_test_client() -> RedisResult<CacheService> {
        let redis_url = "redis://127.0.0.1:6379";
        CacheService::new(redis_url)
    }

    #[tokio::test]
    async fn test_set_get() {
        let cache = create_test_client().await.unwrap();
        cache.set("test_key", "test_value", 60).await.unwrap();
        let val = cache.get("test_key").await.unwrap();
        assert_eq!(val, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_health_check() {
        let cache = create_test_client().await.unwrap();
        assert!(cache.health_check().await);
    }
}