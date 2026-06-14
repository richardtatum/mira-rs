use async_trait::async_trait;
use mira_core::{
    CoreError, PersistenceProvider,
    models::persistence::{Host, HostSubscription, StreamState, Subscription},
};
use sqlx::{migrate::MigrateError, sqlite::{SqliteConnectOptions, SqlitePool}};
use std::str::FromStr;
use uuid::Uuid;

pub struct SqliteClient {
    pool: SqlitePool,
}

impl SqliteClient {
    pub async fn new(database_url: String) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        Ok(Self { pool })
    }

    /// Run migrations
    pub async fn migrate(&self) -> Result<(), MigrateError> {
        sqlx::migrate!().run(&self.pool).await
    }
}

#[async_trait]
impl PersistenceProvider for SqliteClient {
    async fn add_host(&self, url: String, created_by: i64) -> Result<i64, CoreError> {
        let host_id = sqlx::query_scalar!(
            r#"
                INSERT INTO host (url, created_by)
                VALUES (?, ?)
                ON CONFLICT (url) DO UPDATE SET url = excluded.url
                RETURNING id
            "#,
            url,
            created_by
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(host_id)
    }

    async fn link_host(
        &self,
        host_id: i64,
        guild_id: i64,
        auth_header: Option<String>,
        created_by: i64,
    ) -> Result<i64, CoreError> {
        let host_guild_id = sqlx::query!(
            r#"
               INSERT INTO host_guild (host_id, guild_id, auth_header, created_by)
               VALUES (?, ?, ?, ?)
               ON CONFLICT DO NOTHING
            "#,
            host_id,
            guild_id,
            auth_header,
            created_by
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?
        .last_insert_rowid();

        Ok(host_guild_id)
    }

    async fn unlink_host(&self, host_id: i64, guild_id: i64) -> Result<(), CoreError> {
        sqlx::query!(
            r#"
                DELETE FROM host_guild
                WHERE host_id = ?
                AND guild_id = ?
            "#,
            host_id,
            guild_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(())
    }

    async fn get_hosts(&self, guild_id: i64) -> Result<Vec<Host>, CoreError> {
        let hosts = sqlx::query_as!(
            Host,
            r#"
                SELECT h.id, h.url, hg.auth_header, hg.id AS host_guild_id
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

    async fn get_stream_state(&self, subscription_id: i64) -> Result<StreamState, CoreError> {
        let row = sqlx::query!(
            r#"
                SELECT message_id, playing
                FROM subscription
                WHERE id = ?
            "#,
            subscription_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(StreamState::new(row.message_id, row.playing))
    }

    async fn mark_subscription_online(&self, subscription_id: i64, message_id: i64) -> Result<(), CoreError> {
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

    async fn get_subscriptions(&self, guild_id: i64) -> Result<Vec<HostSubscription>, CoreError> {
        let results = sqlx::query!(
            r#"
                SELECT h.id AS host_id, h.url, hg.auth_header, hg.id AS host_guild_id,
                       s.key, s.id AS subscription_id, s.channel_id, s.subscription_token, s.message_id
                FROM subscription s
                INNER JOIN host_guild hg ON hg.id = s.host_guild_id
                INNER JOIN host h ON h.id = hg.host_id
                WHERE hg.guild_id = ?
            "#,
            guild_id
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

                let token = row.subscription_token.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                let subscription = Subscription {
                    id: row.subscription_id,
                    key: row.key,
                    channel_id: row.channel_id,
                    token,
                    message_id: row.message_id,
                };

                HostSubscription { host, subscription }
            })
            .collect();

        Ok(subscriptions)
    }

    async fn get_all_subscriptions(&self) -> Result<Vec<HostSubscription>, CoreError> {
        let results = sqlx::query!(
            r#"
                SELECT h.id AS host_id, h.url, hg.auth_header, hg.id AS host_guild_id,
                       s.key, s.id AS subscription_id, s.channel_id, s.subscription_token, s.message_id
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

                let token = row.subscription_token.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                let subscription = Subscription {
                    id: row.subscription_id,
                    key: row.key,
                    channel_id: row.channel_id,
                    token,
                    message_id: row.message_id,
                };

                HostSubscription { host, subscription }
            })
            .collect();

        Ok(subscriptions)
    }

    async fn delete_subscription(&self, subscription_id: i64) -> Result<(), CoreError> {
        sqlx::query!(
            r#"
                DELETE FROM subscription
                WHERE id = ?
            "#,
            subscription_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(())
    }

    async fn update_subscription_token(&self, subscription_id: i64, token: Uuid) -> Result<(), CoreError> {
        let token_str = token.to_string();
        sqlx::query!(
            r#"
                UPDATE subscription
                SET subscription_token = ?
                WHERE id = ?
            "#,
            token_str,
            subscription_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        Ok(())
    }

    async fn set_playing(&self, subscription_id: i64, playing: String) -> Result<(), CoreError> {
        sqlx::query!(
            r#"
                UPDATE subscription
                SET playing = ?
                WHERE id = ?
            "#,
            playing,
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
    use uuid::Uuid;

    async fn setup() -> SqliteClient {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let client = SqliteClient { pool };
        client.migrate().await.unwrap();
        client
    }

    #[tokio::test]
    async fn test_update_subscription_token() {
        let client = setup().await;

        let host_id =
            sqlx::query_scalar!("INSERT INTO host (url, created_by) VALUES ('http://test.host', 1) RETURNING id")
                .fetch_one(&client.pool)
                .await
                .unwrap();

        let guild_id: i64 = 999;
        let host_guild_id = sqlx::query_scalar!(
            "INSERT INTO host_guild (host_id, guild_id, created_by) VALUES (?, ?, 1) RETURNING id",
            host_id,
            guild_id
        )
        .fetch_one(&client.pool)
        .await
        .unwrap();

        let sub_id = sqlx::query_scalar!(
            "INSERT INTO subscription (key, host_guild_id, channel_id, created_by) VALUES ('stream1', ?, 42, 1) RETURNING id",
            host_guild_id
        )
        .fetch_one(&client.pool)
        .await
        .unwrap();

        let token = Uuid::new_v4();
        client.update_subscription_token(sub_id, token).await.unwrap();

        let stored: Option<String> =
            sqlx::query_scalar!("SELECT subscription_token FROM subscription WHERE id = ?", sub_id)
                .fetch_one(&client.pool)
                .await
                .unwrap();

        assert_eq!(stored, Some(token.to_string()));
    }

    #[tokio::test]
    async fn test_get_subscriptions_to_restore_includes_subscription_token() {
        let client = setup().await;

        let host_id =
            sqlx::query_scalar!("INSERT INTO host (url, created_by) VALUES ('http://test.host', 1) RETURNING id")
                .fetch_one(&client.pool)
                .await
                .unwrap();

        let guild_id: i64 = 999;
        let host_guild_id = sqlx::query_scalar!(
            "INSERT INTO host_guild (host_id, guild_id, created_by) VALUES (?, ?, 1) RETURNING id",
            host_id,
            guild_id
        )
        .fetch_one(&client.pool)
        .await
        .unwrap();

        let sub_id = sqlx::query_scalar!(
            "INSERT INTO subscription (key, host_guild_id, channel_id, created_by) VALUES ('stream1', ?, 42, 1) RETURNING id",
            host_guild_id
        )
        .fetch_one(&client.pool)
        .await
        .unwrap();

        let token = Uuid::new_v4();
        client.update_subscription_token(sub_id, token).await.unwrap();

        let subscriptions = client.get_all_subscriptions().await.unwrap();
        assert_eq!(subscriptions.len(), 1);
        assert_eq!(subscriptions[0].subscription.token, Some(token));
    }
}
