use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use chrono::Utc;
use parking_lot::RwLock;
use reqwest::header::{HeaderValue, CONTENT_TYPE, REFERER, USER_AGENT};
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PollinationsError {
  #[error("image prompt missing from Cerebras payload")]
  MissingPrompt,
  #[error("api key missing")]
  MissingKey,
  #[error("request failed: {0}")]
  Request(#[from] reqwest::Error),
  #[error("api error: {0}")]
  Api(String),
}

// Removed unused structs - Pollinations image API uses GET with URL parameters, not POST with JSON

#[derive(Clone)]
struct CachedModels {
  models: Vec<String>,
  fetched_at: SystemTime,
}

#[derive(Clone)]
pub struct PollinationsClient {
  http: reqwest::Client,
  api_key: Option<String>,
  api_base: String,
  cached_models: Arc<RwLock<Option<CachedModels>>>,
}

impl PollinationsClient {
  pub fn new() -> Result<Self, PollinationsError> {
    let api_key = std::env::var("POLLINATIONS_API_KEY").ok();
    let api_base = std::env::var("POLLINATIONS_API_BASE")
      .unwrap_or_else(|_| "https://api.pollinations.ai".into());
    
    Ok(Self {
      http: reqwest::Client::new(),
      api_key,
      api_base,
      cached_models: Arc::new(RwLock::new(None)),
    })
  }

  // Fallback constructor for when initialization fails
  pub(crate) fn fallback() -> Self {
    Self {
      http: reqwest::Client::new(),
      api_key: None,
      api_base: "https://api.pollinations.ai".into(),
      cached_models: Arc::new(RwLock::new(None)),
    }
  }

  pub async fn get_available_models(&self) -> Result<Vec<String>, PollinationsError> {
    // Check cache first (4 hours = 14400 seconds)
    const CACHE_DURATION_SECS: u64 = 4 * 60 * 60;
    
    {
      let cache = self.cached_models.read();
      if let Some(cached) = cache.as_ref() {
        if let Ok(elapsed) = cached.fetched_at.elapsed() {
          if elapsed.as_secs() < CACHE_DURATION_SECS {
            return Ok(cached.models.clone());
          }
        }
      }
    }

    // Cache expired or missing, fetch fresh models
    let models = self.fetch_models().await?;
    
    // Update cache
    {
      let mut cache = self.cached_models.write();
      *cache = Some(CachedModels {
        models: models.clone(),
        fetched_at: SystemTime::now(),
      });
    }

    Ok(models)
  }

  async fn fetch_models(&self) -> Result<Vec<String>, PollinationsError> {
    let url = "https://image.pollinations.ai/models";
    
    // If API key is available, add it as Bearer token for authenticated requests
    let mut request = self.http.get(url);
    if let Some(api_key) = &self.api_key {
      request = request
        .bearer_auth(api_key)
        .header(
          REFERER,
          HeaderValue::from_str(&self.resolve_referer()).unwrap_or_else(|_| HeaderValue::from_static("https://reminder.w9.nu/")),
        );
    }
    
    let resp = request.send().await?;
    
    if !resp.status().is_success() {
      return Err(PollinationsError::Api(format!(
        "Failed to fetch models: HTTP {}",
        resp.status()
      )));
    }

    let models: Vec<String> = resp.json().await?;
    Ok(models)
  }

  pub async fn generate(&self, prompt: &str, model: Option<&str>) -> Result<String, PollinationsError> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
      return Err(PollinationsError::MissingPrompt);
    }

    // If API key is set, use API endpoint
    if let Some(api_key) = &self.api_key {
      return self.generate_via_api(trimmed, api_key, model).await;
    }

    // Fallback to direct URL generation (no auth required)
    // Banner ratio: 1024x341 (3:1 horizontal banner, max 1024)
    let encoded = urlencoding::encode(trimmed);
    let seed = Utc::now().timestamp();
    let model_param = model
      .map(|m| format!("&model={}", urlencoding::encode(m)))
      .unwrap_or_default();
    Ok(format!(
      "https://image.pollinations.ai/prompt/{}?width=1024&height=341&seed={}{}",
      encoded, seed, model_param
    ))
  }

  async fn generate_via_api(&self, prompt: &str, api_key: &str, model: Option<&str>) -> Result<String, PollinationsError> {
    // Pollinations API uses GET requests with the prompt in the URL path
    // Format: https://image.pollinations.ai/prompt/{prompt}?width=1024&height=341&seed={seed}&model={model}
    // Banner ratio: 1024x341 (3:1 horizontal banner, max 1024)
    let encoded = urlencoding::encode(prompt);
    let seed = Utc::now().timestamp();
    let model_param = model
      .map(|m| format!("&model={}", urlencoding::encode(m)))
      .unwrap_or_default();
    
    let url = format!(
      "https://image.pollinations.ai/prompt/{}?width=1024&height=341&seed={}{}",
      encoded, seed, model_param
    );

    let referer = self.resolve_referer();

    let mut response = self
      .http
      .get(&url)
      .bearer_auth(api_key)
      .header(
        REFERER,
        HeaderValue::from_str(&referer).unwrap_or_else(|_| HeaderValue::from_static("https://reminder.w9.nu/")),
      )
      .header(USER_AGENT, HeaderValue::from_static("w9-daily-reminders/1.0"))
      .send()
      .await?;

    let status = response.status();

    if !status.is_success() {
      let body = response.text().await.unwrap_or_default();
      return Err(PollinationsError::Api(format!(
        "Pollinations request failed ({}): {}",
        status,
        body
      )));
    }

    let mime = response
      .headers()
      .get(CONTENT_TYPE)
      .and_then(|value| value.to_str().ok())
      .unwrap_or("image/jpeg");
    let bytes = response.bytes().await?;
    let encoded = Base64.encode(bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
  }

  fn resolve_referer(&self) -> String {
    std::env::var("POLLINATIONS_REFERRER").unwrap_or_else(|_| "https://reminder.w9.nu/".to_string())
  }
}

