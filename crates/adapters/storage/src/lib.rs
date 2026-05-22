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
                SELECT h.id, h.url, h.auth_header, hg.id AS host_guild_id 
                FROM host h
                INNER JOIN host_guild hg ON hg.host_id = h.id 
                WHERE hg.guild_id = ?
                ORDER BY h.url
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
        key: String,
        host_guild_id: i64,
        channel_id: i64,
        created_by: i64,
    ) -> Result<i64, CoreError> {
        let subscription_id = sqlx::query!(
            r#"
                INSERT INTO subscription (`key`, host_guild_id, channel_id, created_by)
                VALUES (?, ?, ?, ?)
            "#,
            key,
            host_guild_id,
            channel_id,
            created_by
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?
        .last_insert_rowid();

        Ok(subscription_id)
    }

    async fn get_subscription(
        &self,
        key: String,
        host_guild_id: i64,
        channel_id: i64,
    ) -> Result<Subscription, CoreError> {
        let subscription = sqlx::query_as!(
            Subscription,
            r#"
                SELECT id, key, host_guild_id, channel_id, message_id, playing
                FROM subscription
                WHERE host_guild_id = ?
                AND key = ?
                AND channel_id = ?
            "#,
            host_guild_id,
            key,
            channel_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(subscription)
    }

    async fn set_subscription_message_id(
        &self,
        subscription_id: i64,
        message_id: i64,
    ) -> Result<(), CoreError> {
        sqlx::query!(
            r#"
                UPDATE subscription
                SET message_id = ?
                WHERE id = ?
            "#,
            message_id,
            subscription_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(())
    }

    async fn clear_subscription_message_id(&self, subscription_id: i64) -> Result<(), CoreError> {
        sqlx::query!(
            r#"
                UPDATE subscription
                SET message_id = NULL, playing = NULL
                WHERE id = ?
            "#,
            subscription_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(())
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
