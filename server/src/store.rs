use crate::models::{CachedPreview, GoogleTokens, HealthStatus, ReminderSettings, SystemConfig};
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{postgres::PgPoolOptions, Row, PgPool};
use std::sync::Arc;
use thiserror::Error;

const KEY_SETTINGS: &str = "settings";
const KEY_HEALTH: &str = "health";
const KEY_TOKENS: &str = "tokens";
const KEY_CONFIG: &str = "config";
const KEY_PREVIEW: &str = "preview";

#[derive(Clone)]
pub struct DataStore {
    pool: PgPool,
    cache_settings: Arc<RwLock<ReminderSettings>>,
    cache_health: Arc<RwLock<HealthStatus>>,
    cache_tokens: Arc<RwLock<Option<GoogleTokens>>>,
    cache_config: Arc<RwLock<SystemConfig>>,
    cache_preview: Arc<RwLock<Option<CachedPreview>>>,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl DataStore {
    pub async fn new(db_url: &str) -> Result<Self, anyhow::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS reminders_kv (
                key TEXT PRIMARY KEY,
                value JSONB NOT NULL,
                updated_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT)
            )",
        )
        .execute(&pool)
        .await?;

        let settings: ReminderSettings = read_or_default(&pool, KEY_SETTINGS).await?;
        let mut health: HealthStatus = read_or_default(&pool, KEY_HEALTH).await?;
        let tokens: Option<GoogleTokens> = read_optional(&pool, KEY_TOKENS).await?;
        let config: SystemConfig = read_or_default(&pool, KEY_CONFIG).await?;
        let preview: Option<CachedPreview> = read_optional(&pool, KEY_PREVIEW).await.unwrap_or(None);

        if tokens.is_some() && !health.google_connected {
            health.google_connected = true;
            write_json(&pool, KEY_HEALTH, &health).await?;
        }

        Ok(Self {
            pool,
            cache_settings: Arc::new(RwLock::new(settings)),
            cache_health: Arc::new(RwLock::new(health)),
            cache_tokens: Arc::new(RwLock::new(tokens)),
            cache_config: Arc::new(RwLock::new(config)),
            cache_preview: Arc::new(RwLock::new(preview)),
        })
    }

    pub fn read_settings(&self) -> ReminderSettings {
        self.cache_settings.read().clone()
    }

    pub async fn write_settings(&self, data: &ReminderSettings) -> Result<(), StoreError> {
        {
            let mut guard = self.cache_settings.write();
            *guard = data.clone();
        }
        write_json(&self.pool, KEY_SETTINGS, data).await
    }

    pub fn read_health(&self) -> HealthStatus {
        self.cache_health.read().clone()
    }

    pub async fn write_health(&self, data: &HealthStatus) -> Result<(), StoreError> {
        {
            let mut guard = self.cache_health.write();
            *guard = data.clone();
        }
        write_json(&self.pool, KEY_HEALTH, data).await
    }

    pub fn read_google_tokens(&self) -> Option<GoogleTokens> {
        self.cache_tokens.read().clone()
    }

    pub async fn write_google_tokens(&self, data: Option<GoogleTokens>) -> Result<(), StoreError> {
        {
            let mut guard = self.cache_tokens.write();
            *guard = data.clone();
        }
        match data {
            Some(ref tokens) => write_json(&self.pool, KEY_TOKENS, tokens).await,
            None => {
                sqlx::query("DELETE FROM reminders_kv WHERE key = $1")
                    .bind(KEY_TOKENS)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }
        }
    }

    pub fn read_config(&self) -> SystemConfig {
        self.cache_config.read().clone()
    }

    pub async fn write_config(&self, data: &SystemConfig) -> Result<(), StoreError> {
        {
            let mut guard = self.cache_config.write();
            *guard = data.clone();
        }
        write_json(&self.pool, KEY_CONFIG, data).await
    }

    pub fn read_preview(&self) -> Option<CachedPreview> {
        self.cache_preview.read().clone()
    }

    pub async fn write_preview(&self, data: &CachedPreview) -> Result<(), StoreError> {
        {
            let mut guard = self.cache_preview.write();
            *guard = Some(data.clone());
        }
        write_json(&self.pool, KEY_PREVIEW, data).await
    }
}

async fn read_or_default<T>(pool: &PgPool, key: &str) -> Result<T, StoreError>
where
    T: Default + DeserializeOwned,
{
    let row = sqlx::query("SELECT value FROM reminders_kv WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(r) => {
            let val: serde_json::Value = r.try_get(0)?;
            Ok(serde_json::from_value(val)?)
        }
        None => Ok(T::default()),
    }
}

async fn read_optional<T>(pool: &PgPool, key: &str) -> Result<Option<T>, StoreError>
where
    T: DeserializeOwned,
{
    let row = sqlx::query("SELECT value FROM reminders_kv WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(r) => {
            let val: serde_json::Value = r.try_get(0)?;
            Ok(Some(serde_json::from_value(val)?))
        }
        None => Ok(None),
    }
}

async fn write_json<T: Serialize>(pool: &PgPool, key: &str, data: &T) -> Result<(), StoreError> {
    let json_val = serde_json::to_value(data)?;
    sqlx::query(
        "INSERT INTO reminders_kv (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXTRACT(EPOCH FROM NOW())::BIGINT"
    )
    .bind(key)
    .bind(json_val)
    .execute(pool)
    .await?;
    Ok(())
}
