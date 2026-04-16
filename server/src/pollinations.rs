use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use chrono::Utc;
use parking_lot::RwLock;
use reqwest::header::{HeaderValue, CONTENT_TYPE, REFERER, USER_AGENT};
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
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

#[derive(Clone)]
struct CachedModels {
    models: Vec<String>,
    fetched_at: SystemTime,
}

#[derive(Debug, Deserialize)]
struct PollinationsModel {
    name: String,
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
            .unwrap_or_else(|_| "https://enter.pollinations.ai".into());
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            http,
            api_key,
            api_base,
            cached_models: Arc::new(RwLock::new(None)),
        })
    }

    pub fn fallback() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            api_key: None,
            api_base: "https://enter.pollinations.ai".into(),
            cached_models: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_available_models(&self) -> Result<Vec<String>, PollinationsError> {
        const CACHE_DURATION_SECS: u64 = 5 * 60;

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

        let models = self.fetch_models().await?;

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
        let url = format!(
            "{}/api/generate/image/models",
            self.api_base.trim_end_matches('/')
        );

        let mut request = self.http.get(url);
        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key).header(
                REFERER,
                HeaderValue::from_str(&self.resolve_referer())
                    .unwrap_or_else(|_| HeaderValue::from_static("https://reminder.w9.nu/")),
            );
        }

        let resp = request.send().await?;

        if !resp.status().is_success() {
            return Err(PollinationsError::Api(format!(
                "Failed to fetch models: HTTP {}",
                resp.status()
            )));
        }

        let models: Vec<PollinationsModel> = resp.json().await?;
        let names = models.into_iter().map(|m| m.name).collect();
        Ok(names)
    }

    pub async fn generate(
        &self,
        prompt: &str,
        model: Option<&str>,
    ) -> Result<String, PollinationsError> {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return Err(PollinationsError::MissingPrompt);
        }

        let api_key = self
            .api_key
            .as_deref()
            .ok_or(PollinationsError::MissingKey)?;
        self.generate_via_api(trimmed, api_key, model).await
    }

    async fn generate_via_api(
        &self,
        prompt: &str,
        api_key: &str,
        model: Option<&str>,
    ) -> Result<String, PollinationsError> {
        let encoded_pr = urlencoding::encode(prompt);
        let seed = Utc::now().timestamp();
        let model_name = model.unwrap_or("flux");

        let url = format!(
            "{}/api/generate/image/{}",
            self.api_base.trim_end_matches('/'),
            encoded_pr
        );

        let referer = self.resolve_referer();

        let (width, height) = if model_name.eq_ignore_ascii_case("gptimage") {
            ("1536", "1024")
        } else {
            ("600", "150")
        };

        let response = self
            .http
            .get(&url)
            .query(&[
                ("model", model_name),
                ("width", width),
                ("height", height),
                ("seed", &seed.to_string()),
                ("quality", "medium"),
                ("safe", "false"),
                ("nologo", "true"),
                ("transparent", "false"),
            ])
            .bearer_auth(api_key)
            .header(
                REFERER,
                HeaderValue::from_str(&referer)
                    .unwrap_or_else(|_| HeaderValue::from_static("https://reminder.w9.nu/")),
            )
            .header(
                USER_AGENT,
                HeaderValue::from_static("w9-daily-reminders/1.0"),
            )
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(PollinationsError::Api(format!(
                "Pollinations request failed ({}): {}",
                status, body
            )));
        }

        let mime = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();
        let bytes = response.bytes().await?;
        let encoded = Base64.encode(bytes);
        Ok(format!("data:{};base64,{}", mime, encoded))
    }

    fn resolve_referer(&self) -> String {
        std::env::var("POLLINATIONS_REFERRER")
            .unwrap_or_else(|_| "https://reminder.w9.nu/".to_string())
    }
}
