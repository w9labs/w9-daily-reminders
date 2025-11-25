use chrono::Utc;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize)]
struct GenerateRequest {
  prompt: String,
  width: u32,
  height: u32,
  seed: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
  image_url: Option<String>,
  url: Option<String>,
  error: Option<String>,
}

#[derive(Clone)]
pub struct PollinationsClient {
  http: reqwest::Client,
  api_key: Option<String>,
  api_base: String,
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
    })
  }

  // Fallback constructor for when initialization fails
  pub(crate) fn fallback() -> Self {
    Self {
      http: reqwest::Client::new(),
      api_key: None,
      api_base: "https://api.pollinations.ai".into(),
    }
  }

  pub async fn generate(&self, prompt: &str) -> Result<String, PollinationsError> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
      return Err(PollinationsError::MissingPrompt);
    }

    // If API key is set, use API endpoint
    if let Some(api_key) = &self.api_key {
      return self.generate_via_api(trimmed, api_key).await;
    }

    // Fallback to direct URL generation (no auth required)
    let encoded = urlencoding::encode(trimmed);
    let seed = Utc::now().timestamp();
    Ok(format!(
      "https://image.pollinations.ai/prompt/{}?width=1024&height=1024&seed={}",
      encoded, seed
    ))
  }

  async fn generate_via_api(&self, prompt: &str, api_key: &str) -> Result<String, PollinationsError> {
    let url = format!("{}/api/generate", self.api_base.trim_end_matches('/'));
    let seed = Utc::now().timestamp();
    
    let req = GenerateRequest {
      prompt: prompt.to_string(),
      width: 1024,
      height: 1024,
      seed: Some(seed),
    };

    let resp = self
      .http
      .post(&url)
      .bearer_auth(api_key)
      .json(&req)
      .send()
      .await?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
      return Err(PollinationsError::Api(format!("HTTP {}: {}", status, body_text)));
    }

    let json: GenerateResponse = serde_json::from_str(&body_text)
      .map_err(|_| PollinationsError::Api(format!("Invalid response: {}", body_text)))?;

    if let Some(error) = json.error {
      return Err(PollinationsError::Api(error));
    }

    // Try image_url first, then url, then fallback to direct URL
    if let Some(image_url) = json.image_url {
      return Ok(image_url);
    }
    
    if let Some(url) = json.url {
      return Ok(url);
    }

    // Fallback to direct URL if API doesn't return one
    let encoded = urlencoding::encode(prompt);
    Ok(format!(
      "https://image.pollinations.ai/prompt/{}?width=1024&height=1024&seed={}",
      encoded, seed
    ))
  }
}

