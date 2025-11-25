use std::time::Duration;

use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use chrono::Utc;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use thiserror::Error;

static CLOUDFLARE_MODELS: &[&str] = &[
  "@cf/black-forest-labs/flux-2-dev",
  "@cf/black-forest-labs/flux-1-schnell",
  "@cf/bytedance/stable-diffusion-xl-lightning",
];

#[derive(Debug, Error)]
pub enum CloudflareAiError {
  #[error("cloudflare account id missing")]
  MissingAccountId,
  #[error("cloudflare api token missing")]
  MissingApiToken,
  #[error("http error: {0}")]
  Request(#[from] reqwest::Error),
  #[error("serde error: {0}")]
  Serde(#[from] serde_json::Error),
  #[error("api error: {0}")]
  Api(String),
  #[error("image payload missing in response")]
  MissingImage,
}

#[derive(Clone)]
pub struct CloudflareAiClient {
  http: reqwest::Client,
  account_id: String,
  api_token: String,
  api_base: String,
}

impl CloudflareAiClient {
  pub fn new() -> Result<Self, CloudflareAiError> {
    let account_id = std::env::var("CLOUDFLARE_ACCOUNT_ID").map_err(|_| CloudflareAiError::MissingAccountId)?;
    let api_token = std::env::var("CLOUDFLARE_API_TOKEN").map_err(|_| CloudflareAiError::MissingApiToken)?;
    let api_base = std::env::var("CLOUDFLARE_AI_BASE").unwrap_or_else(|_| "https://api.cloudflare.com/client/v4".into());

    let http = reqwest::Client::builder()
      .timeout(Duration::from_secs(60))
      .build()?;

    Ok(Self {
      http,
      account_id,
      api_token,
      api_base,
    })
  }

  pub fn supported_models() -> Vec<String> {
    CLOUDFLARE_MODELS.iter().map(|m| m.to_string()).collect()
  }

  pub async fn generate(&self, prompt: &str, model: Option<&str>) -> Result<String, CloudflareAiError> {
    let model_name = model.filter(|m| !m.is_empty()).unwrap_or(CLOUDFLARE_MODELS[0]);
    let url = format!(
      "{}/accounts/{}/ai/run/{}",
      self.api_base.trim_end_matches('/'),
      self.account_id,
      model_name
    );

    let body = build_payload(model_name, prompt);

    let mut request = self.http.post(&url).json(&body);
    let mut token_header = HeaderValue::from_str(&format!("Bearer {}", self.api_token)).map_err(|err| CloudflareAiError::Api(err.to_string()))?;
    token_header.set_sensitive(true);
    request = request.header(AUTHORIZATION, token_header);

    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
      let body_text = response.text().await.unwrap_or_default();
      return Err(CloudflareAiError::Api(format!("HTTP {}: {}", status, body_text)));
    }

    let content_type = response
      .headers()
      .get(CONTENT_TYPE)
      .and_then(|value| value.to_str().ok())
      .unwrap_or("")
      .to_string();

    if content_type.contains("application/json") {
      let text = response.text().await?;
      let parsed: CloudflareJsonResponse = serde_json::from_str(&text)?;
      if let Some(result) = parsed.result {
        if let Some(image) = result.image {
          return Ok(format!("data:image/jpeg;base64,{}", image));
        }
      }
      return Err(CloudflareAiError::MissingImage);
    }

    let bytes = response.bytes().await?;
    let mime = if content_type.is_empty() { "image/jpeg" } else { content_type.as_str() };
    let encoded = Base64.encode(bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
  }
}

#[derive(Debug, Deserialize)]
struct CloudflareJsonResponse {
  result: Option<CloudflareResult>,
}

#[derive(Debug, Deserialize)]
struct CloudflareResult {
  image: Option<String>,
}

fn build_payload(model: &str, prompt: &str) -> serde_json::Value {
  let seed = Utc::now().timestamp();
  match model {
    "@cf/bytedance/stable-diffusion-xl-lightning" => serde_json::json!({
      "prompt": prompt,
      "width": 1024,
      "height": 256,
      "num_steps": 8,
      "guidance": 5,
      "seed": seed,
    }),
    "@cf/black-forest-labs/flux-2-dev" | "@cf/black-forest-labs/flux-1-schnell" => serde_json::json!({
      "prompt": prompt,
      "width": 1024,
      "height": 256,
      "seed": seed,
    }),
    _ => serde_json::json!({ "prompt": prompt }),
  }
}

