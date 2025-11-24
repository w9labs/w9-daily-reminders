use chrono::Utc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PollinationsError {
  #[error("image prompt missing from Cerebras payload")]
  MissingPrompt,
}

#[derive(Clone, Default)]
pub struct PollinationsClient;

impl PollinationsClient {
  pub fn new() -> Self {
    Self
  }

  pub async fn generate(&self, prompt: &str) -> Result<String, PollinationsError> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
      return Err(PollinationsError::MissingPrompt);
    }
    let encoded = urlencoding::encode(trimmed);
    let seed = Utc::now().timestamp();
    // Pollinations serves images directly from this URL without authentication.
    Ok(format!(
      "https://image.pollinations.ai/prompt/{}?width=1024&height=1024&seed={}",
      encoded, seed
    ))
  }
}

