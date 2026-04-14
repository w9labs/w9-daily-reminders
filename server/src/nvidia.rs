use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NvidiaError {
    #[error("missing api key")]
    MissingKey,
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("api error: {status} {body}")]
    Api { status: u16, body: String },
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum NvidiaModel {
    #[default]
    MiniMaxM27,
    GLM47,
}

impl NvidiaModel {
    pub fn id(&self) -> &'static str {
        match self {
            NvidiaModel::MiniMaxM27 => "minimaxai/minimax-m2.7",
            NvidiaModel::GLM47 => "z-ai/glm4.7",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "glm4.7" | "z-ai/glm4.7" => NvidiaModel::GLM47,
            _ => NvidiaModel::MiniMaxM27,
        }
    }

    pub fn all() -> &'static [(&'static str, &'static str)] {
        &[
            ("minimaxai/minimax-m2.7", "MiniMax M2.7 (Recommended)"),
            ("z-ai/glm4.7", "GLM 4.7"),
        ]
    }
}

#[derive(Debug, Deserialize)]
struct NvidiaChoice {
    message: NvidiaMessage,
}

#[derive(Debug, Deserialize)]
struct NvidiaMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NvidiaResponse {
    choices: Vec<NvidiaChoice>,
    #[serde(default)]
    error: Option<NvidiaErrorPayload>,
}

#[derive(Debug, Deserialize)]
struct NvidiaErrorPayload {
    #[serde(default)]
    message: Option<String>,
}

#[derive(Clone)]
pub struct NvidiaClient {
    http: reqwest::Client,
    api_key: String,
}

impl NvidiaClient {
    pub fn new() -> Result<Self, NvidiaError> {
        let api_key = std::env::var("NVIDIA_API_KEY").map_err(|_| NvidiaError::MissingKey)?;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;
        Ok(Self { http, api_key })
    }

    pub fn supported_models() -> Vec<String> {
        NvidiaModel::all().iter().map(|(id, _)| id.to_string()).collect()
    }

    /// Send a chat completion request to NVIDIA NIM.
    /// Returns the raw text content from the first choice.
    pub async fn chat(
        &self,
        model: NvidiaModel,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, NvidiaError> {
        let url = "https://integrate.api.nvidia.com/v1/chat/completions";
        tracing::debug!(model = model.id(), "NVIDIA chat request");

        let response = self
            .http
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model.id(),
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}
                ],
                "temperature": 1.0,
                "top_p": 0.95,
                "max_tokens": 4000,
                "stream": false,
            }))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!(%status, body = %body, "NVIDIA API error");
            return Err(NvidiaError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let text = response.text().await?;
        let resp: NvidiaResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(body = %text, error = ?e, "Failed to parse NVIDIA response");
            NvidiaError::InvalidResponse(e.to_string())
        })?;

        if let Some(err) = resp.error {
            return Err(NvidiaError::Api {
                status: 500,
                body: err.message.unwrap_or_else(|| "Unknown NVIDIA error".into()),
            });
        }

        let choice = resp.choices.into_iter().next().ok_or_else(|| {
            NvidiaError::InvalidResponse("No choices in NVIDIA response".into())
        })?;

        let content = choice
            .message
            .content
            .or(choice.message.reasoning)
            .ok_or_else(|| NvidiaError::InvalidResponse("No content in NVIDIA response".into()))?;

        Ok(content)
    }
}
