use async_trait::async_trait;
use mira_core::{
    CoreError, PersistenceProvider, StreamStatus,
    models::persistence::{Host, Subscription},
};
use sqlx::{migrate::MigrateError, sqlite::SqlitePool};

pub struct SqliteClient {
    pool: SqlitePool,
}

impl SqliteClient {
    pub async fn new(database_url: String) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(&database_url).await?;
        Ok(Self { pool })
    }

    /// Run migrations
    pub async fn migrate(&self) -> Result<(), MigrateError> {
        sqlx::migrate!().run(&self.pool).await
    }
}

#[async_trait]
impl PersistenceProvider for SqliteClient {
    async fn get_hosts(&self, guild_id: i64) -> Result<Vec<Host>, CoreError> {
        let hosts = sqlx::query_as!(
            Host,
            r#"
                SELECT id, url, auth_header, guild_id, created_by
                FROM host
                WHERE guild_id = ?
                ORDER BY url
            "#,
            guild_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(hosts)
    }

    async fn add_subscription(
        &self,
        host_id: i64,
        key: String,
        channel_id: i64,
        created_by: i64,
    ) -> Result<i64, CoreError> {
        let subscription_id = sqlx::query!(
            r#"
                INSERT INTO subscription (host_id, `key`, channel_id, created_by)
                VALUES (?, ?, ?, ?)
            "#,
            host_id,
            key,
            channel_id,
            created_by
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?
        .last_insert_rowid();

        Ok(subscription_id)
    }

    async fn add_stream(
        &self,
        subscription_id: i64,
        status: StreamStatus,
        viewer_count: i64,
        message_id: i64,
        start_time: String,
    ) -> Result<i64, CoreError> {
        let db_status = status.to_db_string();
        let stream_id = sqlx::query!(
            r#"
                INSERT INTO stream (subscription_id, status, viewer_count, message_id, start_time)
                VALUES (?, ?, ?, ?, ?)
            "#,
            subscription_id,
            db_status,
            viewer_count,
            message_id,
            start_time
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?
        .last_insert_rowid();

        Ok(stream_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscription_crud() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let client = SqliteClient { pool };

        // Run migrations
        client.migrate().await.unwrap();

        // Create subscription
        client
            .add_subscription(1, "test-key".to_string(), 123, 1)
            .await
            .unwrap();
    }
}
