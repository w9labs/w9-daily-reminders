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

// Removed unused structs - Pollinations image API uses GET with URL parameters, not POST with JSON

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
    // Pollinations API uses GET requests with the prompt in the URL path
    // Format: https://image.pollinations.ai/prompt/{prompt}?width=1024&height=1024&seed={seed}
    // According to docs, authentication can be via Bearer token or referrer
    // The API returns the image directly, so we just need to construct the URL
    let encoded = urlencoding::encode(prompt);
    let seed = Utc::now().timestamp();
    
    // Build the URL with query parameters
    let url = format!(
      "https://image.pollinations.ai/prompt/{}?width=1024&height=1024&seed={}",
      encoded, seed
    );
    
    // For authenticated requests, we can add the API key as a query parameter
    // or use Bearer token in header. According to docs, Bearer token works for GET requests too.
    // Let's try with Bearer token first, and if that fails, use the URL directly
    // (the image will be generated and served from that URL)
    
    // Actually, since the image API returns the image directly (not JSON),
    // and the URL itself is the image URL, we can just return it.
    // The API key/referrer is mainly for rate limiting and authentication tracking.
    // We'll add the referrer as a query parameter if needed, but the URL itself works.
    
    // For now, just return the URL - it will work with or without auth
    // The API key is used for rate limiting, not for URL generation
    Ok(url)
  }
}

