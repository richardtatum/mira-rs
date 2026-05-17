use async_trait::async_trait;
use mira_core::{CoreError, PersistenceProvider, models::persistence::{Host, Subscription}};
use sqlx::sqlite::SqlitePool;
use sqlx::Sqlite;

pub struct SqliteClient {
    pool: SqlitePool,
}

impl SqliteClient {
    pub async fn new(database_url: String) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(&database_url).await?;
        Ok(Self { pool })
    }

    /// Run migrations
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query_file::<Sqlite, _>(
            "migrations/0001_create_tables.sql",
            &self.pool,
        )
        .await?;
        Ok(())
    }

    /// Create a new subscription
    pub async fn create_subscription(
        &self,
        guild_id: u64,
        host_id: u64,
        key: String,
        channel_id: u64,
        created_by: u64,
    ) -> Result<Subscription, CoreError> {
        let result = sqlx::query_as!(
            Subscription,
            r#"
            INSERT INTO subscription (guild_id, host_id, `key`, channel_id, created_by)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
            guild_id, host_id, key, channel_id, created_by
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(result)
    }

    /// Get subscriptions by guild ID
    pub async fn get_subscriptions_by_guild(
        &self,
        guild_id: u64,
    ) -> Result<Vec<Subscription>, CoreError> {
        let results = sqlx::query_as!(
            Subscription,
            r#"SELECT * FROM subscription WHERE guild_id = ? ORDER BY key"#, guild_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(results)
    }

    /// Get subscriptions by channel ID
    pub async fn get_subscriptions_by_channel(
        &self,
        channel_id: u64,
    ) -> Result<Vec<Subscription>, CoreError> {
        let results = sqlx::query_as!(
            Subscription,
            r#"SELECT * FROM subscription WHERE channel_id = ? ORDER BY key"#, channel_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(results)
    }

    /// Get a subscription by key
    pub async fn get_subscription_by_key(
        &self,
        key: &str,
    ) -> Result<Subscription, CoreError> {
        let result = sqlx::query_as!(
            Subscription,
            r#"SELECT * FROM subscription WHERE key = ?"#, key
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(result)
    }

    /// Delete a subscription by key
    pub async fn delete_subscription(
        &self,
        key: &str,
    ) -> Result<u64, CoreError> {
        let result = sqlx::query!(
            r#"DELETE FROM subscription WHERE key = ?"#, key
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Update subscription (e.g., change channel)
    pub async fn update_subscription(
        &self,
        guild_id: u64,
        key: String,
        channel_id: u64,
    ) -> Result<u64, CoreError> {
        let result = sqlx::query!(
            r#"UPDATE subscription SET channel_id = ? WHERE guild_id = ? AND key = ?"#, channel_id, guild_id, key
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(result.rows_affected())
    }
}

#[async_trait]
impl PersistenceProvider for SqliteClient {
    async fn get_hosts(&self) -> Result<Vec<Host>, CoreError> {
        let results = sqlx::query!(r#"SELECT id, url, auth_header, guild_id, created_by FROM host ORDER BY url"#)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        let hosts = results
            .into_iter()
            .map(|row| Host {
                id: row.id,
                url: row.url,
                auth_header: row.auth_header,
                guild_id: row.guild_id,
                created_by: row.created_by,
            })
            .collect();

        Ok(hosts)
    }

    async fn get_subscriptions(
        &self,
        host_url: String,
    ) -> Result<Vec<Subscription>, CoreError> {
        let results = sqlx::query!(r#"SELECT id, guild_id, host_id, `key`, channel_id, created_by FROM subscription WHERE host_id = ?"#, host_url)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        let subscriptions = results
            .into_iter()
            .map(|row| Subscription {
                id: row.id,
                guild_id: row.guild_id,
                host_id: row.host_id,
                key: row.key,
                channel_id: row.channel_id,
                created_by: row.created_by,
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
        let sub = client
            .create_subscription(1, 1, "test-key".to_string(), 123, 1)
            .await
            .unwrap();

        assert_eq!(sub.key, "test-key");
        assert_eq!(sub.channel_id, 123);

        // Get by guild
        let subs = client.get_subscriptions_by_guild(1).await.unwrap();
        assert_eq!(subs.len(), 1);

        // Delete
        client.delete_subscription("test-key").await.unwrap();

        let subs = client.get_subscriptions_by_guild(1).await.unwrap();
        assert_eq!(subs.len(), 0);
    }
}

