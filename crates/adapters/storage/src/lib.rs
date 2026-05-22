use async_trait::async_trait;
use mira_core::{
    CoreError, PersistenceProvider,
    models::persistence::{Host, Subscription, SubscriptionRestore},
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

    async fn get_subscription(&self, subscription_id: i64) -> Result<Subscription, CoreError> {
        let subscription = sqlx::query_as!(
            Subscription,
            r#"
                SELECT id, key, host_guild_id, channel_id, message_id, playing
                FROM subscription
                WHERE id = ?
            "#,
            subscription_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(subscription)
    }

    async fn mark_subscription_online(
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

    async fn mark_subscription_offline(&self, subscription_id: i64) -> Result<(), CoreError> {
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

    async fn get_all_subscriptions(&self) -> Result<Vec<SubscriptionRestore>, CoreError> {
        let results = sqlx::query!(
            r#"
                SELECT h.id AS host_id, h.url, h.auth_header, hg.id AS host_guild_id, s.key, s.id AS subscription_id
                FROM host h
                INNER JOIN host_guild hg ON hg.host_id = h.id
                INNER JOIN subscription s ON s.host_guild_id = hg.id
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        let subscriptions = results
            .into_iter()
            .map(|row| {
                let host = Host {
                    id: row.host_id,
                    url: row.url,
                    auth_header: row.auth_header,
                    host_guild_id: row.host_guild_id,
                };
                SubscriptionRestore {
                    host,
                    key: row.key,
                    subscription_id: row.subscription_id,
                }
            })
            .collect();

        Ok(subscriptions)
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
        // client
        //     .add_subscription(1, "test-key".to_string(), 123, 1)
        //     .await
        //     .unwrap();
    }
}
