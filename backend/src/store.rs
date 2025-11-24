use crate::models::{GoogleTokens, HealthStatus, ReminderSettings, SystemConfig};
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Serialize};
use std::{path::PathBuf, sync::Arc};
use thiserror::Error;

const SETTINGS_FILE: &str = "settings.json";
const HEALTH_FILE: &str = "health.json";
const GOOGLE_TOKENS_FILE: &str = "google_tokens.json";
const CONFIG_FILE: &str = "config.json";

#[derive(Clone)]
pub struct DataStore {
  root: PathBuf,
  cache_settings: Arc<RwLock<ReminderSettings>>, 
  cache_health: Arc<RwLock<HealthStatus>>, 
  cache_tokens: Arc<RwLock<Option<GoogleTokens>>>, 
  cache_config: Arc<RwLock<SystemConfig>>, 
}

#[derive(Debug, Error)]
pub enum StoreError {
  #[error("io error: {0}")]
  Io(#[from] std::io::Error),
  #[error("serde error: {0}")]
  Serde(#[from] serde_json::Error),
}

impl DataStore {
  pub async fn new<P: Into<PathBuf>>(root: P) -> Result<Self, StoreError> {
    let root = root.into();
    tokio::fs::create_dir_all(&root).await?;

    let settings: ReminderSettings = read_or_default(root.join(SETTINGS_FILE)).await?;
    let mut health: HealthStatus = read_or_default(root.join(HEALTH_FILE)).await?;
    let tokens: Option<GoogleTokens> = read_optional(root.join(GOOGLE_TOKENS_FILE)).await?;
    let config: SystemConfig = read_or_default(root.join(CONFIG_FILE)).await?;
    if tokens.is_some() && !health.google_connected {
      health.google_connected = true;
      write_json(root.join(HEALTH_FILE), &health).await?;
    }

    Ok(Self {
      root,
      cache_settings: Arc::new(RwLock::new(settings)),
      cache_health: Arc::new(RwLock::new(health)),
      cache_tokens: Arc::new(RwLock::new(tokens)),
      cache_config: Arc::new(RwLock::new(config)),
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
    write_json(self.root.join(SETTINGS_FILE), data).await
  }

  pub fn read_health(&self) -> HealthStatus {
    self.cache_health.read().clone()
  }

  pub async fn write_health(&self, data: &HealthStatus) -> Result<(), StoreError> {
    {
      let mut guard = self.cache_health.write();
      *guard = data.clone();
    }
    write_json(self.root.join(HEALTH_FILE), data).await
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
      Some(ref tokens) => write_json(self.root.join(GOOGLE_TOKENS_FILE), tokens).await,
      None => {
        match tokio::fs::remove_file(self.root.join(GOOGLE_TOKENS_FILE)).await {
          Ok(_) | Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
          Err(err) => Err(err.into()),
        }
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
    write_json(self.root.join(CONFIG_FILE), data).await
  }
}

async fn read_or_default<T>(path: PathBuf) -> Result<T, StoreError>
where
  T: Default + DeserializeOwned,
{
  match tokio::fs::read(&path).await {
    Ok(bytes) => {
      if bytes.is_empty() {
        return Ok(T::default())
      }
      Ok(serde_json::from_slice(&bytes)?)
    }
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
    Err(err) => Err(err.into()),
  }
}

async fn read_optional<T>(path: PathBuf) -> Result<Option<T>, StoreError>
where
  T: DeserializeOwned,
{
  match tokio::fs::read(&path).await {
    Ok(bytes) => {
      if bytes.is_empty() {
        return Ok(None)
      }
      Ok(Some(serde_json::from_slice(&bytes)?))
    }
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(err) => Err(err.into()),
  }
}

async fn write_json<T: Serialize>(path: PathBuf, data: &T) -> Result<(), StoreError> {
  let json = serde_json::to_vec_pretty(data)?;
  tokio::fs::write(path, json).await?;
  Ok(())
}
