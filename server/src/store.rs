use crate::models::{GoogleTokens, HealthStatus, ReminderSettings};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::{PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone)]
pub struct DataStore {
    pool: PgPool,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreviewCache {
    pub subject: String,
    pub html: String,
    pub text: String,
    pub weather_advisory: Option<String>,
    pub image_url: Option<String>,
    pub generated_language: String,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogEntry {
    pub id: String,
    pub executed_at: String,
    pub events_count: i32,
    pub email_sent: bool,
    pub error_message: Option<String>,
}

impl DataStore {
    pub async fn new(db_url: &str) -> Result<Self, anyhow::Error> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .idle_timeout(std::time::Duration::from_secs(300))
            .connect(db_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;
        Self::migrate(&pool).await?;
        Ok(Self { pool })
    }

    async fn migrate(pool: &PgPool) -> Result<(), anyhow::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_settings (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_email TEXT NOT NULL UNIQUE,
                settings JSONB NOT NULL DEFAULT '{}',
                google_tokens JSONB,
                last_preview JSONB,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS reminder_execution_log (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_email TEXT NOT NULL,
                executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                events_count INTEGER DEFAULT 0,
                email_sent BOOLEAN DEFAULT false,
                error_message TEXT
            )",
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS system_health (
                id INTEGER PRIMARY KEY DEFAULT 1,
                scheduler_state TEXT NOT NULL DEFAULT 'idle',
                last_dispatch TIMESTAMPTZ,
                next_run TIMESTAMPTZ,
                google_connected BOOLEAN DEFAULT false
            )",
        )
        .execute(pool)
        .await?;

        sqlx::query("INSERT INTO system_health (id) VALUES (1) ON CONFLICT (id) DO NOTHING")
            .execute(pool)
            .await?;

        tracing::info!("Database migration complete");
        Ok(())
    }

    pub async fn ensure_user(&self, email: &str) -> Result<Uuid, StoreError> {
        let row = sqlx::query("SELECT id FROM user_settings WHERE user_email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        if let Some(r) = row {
            return Ok(r.try_get("id")?);
        }
        let default = ReminderSettings {
            user_email: email.to_string(),
            ..Default::default()
        };
        let settings_json = serde_json::to_value(&default)?;
        let row = sqlx::query(
            "INSERT INTO user_settings (user_email, settings) VALUES ($1, $2) RETURNING id",
        )
        .bind(email)
        .bind(settings_json)
        .fetch_one(&self.pool)
        .await?;
        tracing::info!(email, "Created new user settings row");
        Ok(row.try_get("id")?)
    }

    pub async fn read_settings(&self, email: &str) -> Result<ReminderSettings, StoreError> {
        let row = sqlx::query("SELECT settings FROM user_settings WHERE user_email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => {
                let val: serde_json::Value = r.try_get("settings")?;
                let settings: ReminderSettings = serde_json::from_value(val)?;
                Ok(settings)
            }
            None => {
                let default = ReminderSettings {
                    user_email: email.to_string(),
                    ..Default::default()
                };
                self.write_settings(email, &default).await?;
                Ok(default)
            }
        }
    }

    pub async fn write_settings(
        &self,
        email: &str,
        settings: &ReminderSettings,
    ) -> Result<(), StoreError> {
        let json = serde_json::to_value(settings)?;
        sqlx::query(
            "INSERT INTO user_settings (user_email, settings) VALUES ($1, $2)
             ON CONFLICT (user_email) DO UPDATE SET settings = EXCLUDED.settings, updated_at = NOW()",
        )
        .bind(email)
        .bind(json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn read_google_tokens(
        &self,
        email: &str,
    ) -> Result<Option<GoogleTokens>, StoreError> {
        let row = sqlx::query("SELECT google_tokens FROM user_settings WHERE user_email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => {
                let val: Option<serde_json::Value> = r.try_get("google_tokens")?;
                if let Some(v) = val {
                    return Ok(Some(serde_json::from_value(v)?));
                }
            }
            None => {}
        }
        Ok(None)
    }

    pub async fn write_google_tokens(
        &self,
        email: &str,
        tokens: Option<&GoogleTokens>,
    ) -> Result<(), StoreError> {
        let json = tokens.map(|t| serde_json::to_value(t)).transpose()?;
        sqlx::query(
            "UPDATE user_settings SET google_tokens = $2, updated_at = NOW() WHERE user_email = $1",
        )
        .bind(email)
        .bind(json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn read_preview(&self, email: &str) -> Result<Option<UserPreviewCache>, StoreError> {
        let row = sqlx::query("SELECT last_preview FROM user_settings WHERE user_email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => {
                let val: Option<serde_json::Value> = r.try_get("last_preview")?;
                if let Some(v) = val {
                    return Ok(Some(serde_json::from_value(v)?));
                }
            }
            None => {}
        }
        Ok(None)
    }

    pub async fn write_preview(
        &self,
        email: &str,
        preview: &UserPreviewCache,
    ) -> Result<(), StoreError> {
        let json = serde_json::to_value(preview)?;
        sqlx::query(
            "UPDATE user_settings SET last_preview = $2, updated_at = NOW() WHERE user_email = $1",
        )
        .bind(email)
        .bind(json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn log_execution(
        &self,
        email: &str,
        events_count: i32,
        sent: bool,
        error: Option<&str>,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO reminder_execution_log (user_email, events_count, email_sent, error_message) VALUES ($1, $2, $3, $4)",
        )
        .bind(email)
        .bind(events_count)
        .bind(sent)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_execution_log(
        &self,
        email: &str,
        limit: i64,
    ) -> Result<Vec<ExecutionLogEntry>, StoreError> {
        let rows = sqlx::query(
            "SELECT id::text, executed_at::text, events_count, email_sent, error_message
             FROM reminder_execution_log WHERE user_email = $1 ORDER BY executed_at DESC LIMIT $2",
        )
        .bind(email)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ExecutionLogEntry {
                id: r.try_get("id").unwrap_or_default(),
                executed_at: r.try_get("executed_at").unwrap_or_default(),
                events_count: r.try_get("events_count").unwrap_or(0),
                email_sent: r.try_get("email_sent").unwrap_or(false),
                error_message: r.try_get("error_message").ok().flatten(),
            })
            .collect())
    }

    pub async fn read_health(&self) -> Result<HealthStatus, StoreError> {
        let row = sqlx::query(
            "SELECT scheduler_state, last_dispatch, next_run, google_connected FROM system_health WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(r) => {
                let state: String = r.try_get("scheduler_state")?;
                Ok(HealthStatus {
                    scheduler: match state.as_str() {
                        "waiting" => crate::models::SchedulerState::Waiting,
                        "sending" => crate::models::SchedulerState::Sending,
                        _ => crate::models::SchedulerState::Idle,
                    },
                    last_dispatch: r.try_get("last_dispatch").ok().flatten(),
                    next_run: r.try_get("next_run").ok().flatten(),
                    google_connected: r
                        .try_get("google_connected")
                        .ok()
                        .flatten()
                        .unwrap_or(false),
                })
            }
            None => Ok(HealthStatus::default()),
        }
    }

    pub async fn write_health(&self, data: &HealthStatus) -> Result<(), StoreError> {
        let scheduler_state = match data.scheduler {
            crate::models::SchedulerState::Idle => "idle",
            crate::models::SchedulerState::Waiting => "waiting",
            crate::models::SchedulerState::Sending => "sending",
        };
        sqlx::query(
            "UPDATE system_health SET scheduler_state = $1, last_dispatch = $2, next_run = $3, google_connected = $4 WHERE id = 1",
        )
        .bind(scheduler_state)
        .bind(data.last_dispatch)
        .bind(data.next_run)
        .bind(data.google_connected)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_all_users(&self) -> Result<Vec<(String, DateTime<Utc>, bool)>, StoreError> {
        let rows = sqlx::query(
            "SELECT user_email, updated_at, (google_tokens IS NOT NULL) as has_google FROM user_settings ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let email: String = r.try_get("user_email").unwrap_or_default();
                let updated: DateTime<Utc> = r.try_get("updated_at").unwrap_or(Utc::now());
                let has_google: Option<bool> = r.try_get("has_google").ok();
                (email, updated, has_google.unwrap_or(false))
            })
            .collect())
    }
}
