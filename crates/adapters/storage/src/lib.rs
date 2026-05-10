use async_trait::async_trait;
use mira_core::{CoreError, PersistenceProvider, models::persistence::Host};
use sqlx::sqlite::SqlitePool;

pub struct SqliteClient {
    pool: SqlitePool,
}

impl SqliteClient {
    pub async fn new(database_url: String) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(&database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl PersistenceProvider for SqliteClient {
    async fn get_hosts(&self) -> Result<Vec<Host>, CoreError> {
        let results = sqlx::query!(r#"SELECT id, url, auth_header FROM host"#)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::PersistenceError(e.to_string()))?;

        let hosts = results
            .into_iter()
            .map(|row| Host {
                id: row.id,
                url: row.url,
                auth_header: row.auth_header,
                guild_id: 1,   // TEMP
                created_by: 1, // TEMP
            })
            .collect();

        Ok(hosts)
    }
}
