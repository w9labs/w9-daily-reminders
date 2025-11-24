use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::{CalendarEvent, ReminderSettings, SummaryStyle};

#[derive(Debug, Error)]
pub enum CerebrasError {
  #[error("missing api key")]
  MissingKey,
  #[error("request failed: {0}")]
  Request(#[from] reqwest::Error),
  #[error("response missing data")]
  Invalid,
  #[error("cerebras error: {0}")]
  Api(String),
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
  model: &'a str,
  messages: Vec<ChatMessage<'a>>,
  temperature: f32,
  max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
  role: &'a str,
  content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
  choices: Vec<Choice>,
  #[serde(default)]
  error: Option<ApiErrorPayload>,
}

#[derive(Debug, Deserialize)]
struct Choice {
  message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
  content: Option<Vec<ContentBlock>>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
  #[serde(rename = "type")]
  kind: String,
  text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorPayload {
  #[serde(rename = "type")]
  kind: Option<String>,
  message: Option<String>,
}

#[derive(Clone)]
pub struct CerebrasClient {
  http: reqwest::Client,
  api_key: String,
  model: String,
}

impl CerebrasClient {
  pub fn new() -> Result<Self, CerebrasError> {
    let api_key = std::env::var("CEREBRAS_API_KEY").map_err(|_| CerebrasError::MissingKey)?;
    let model = std::env::var("CEREBRAS_MODEL").unwrap_or_else(|_| "zai-glm-4.6".into());
    Ok(Self {
      http: reqwest::Client::new(),
      api_key,
      model,
    })
  }

  pub async fn generate_email(
    &self,
    settings: &ReminderSettings,
    events: &[CalendarEvent],
    weather: Option<&str>,
  ) -> Result<String, CerebrasError> {
    let instructions = build_prompt(settings, events, weather);
    let req = serde_json::json!({
      "model": self.model,
      "messages": [
        {
          "role": "system",
          "content": [
            {
              "type": "text",
              "text": "You are W9 Reminders AI. Output JSON with keys subject, preview, html_body, text_body, image_prompt."
            }
          ]
        },
        {
          "role": "user",
          "content": [
            {
              "type": "text",
              "text": instructions
            }
          ]
        }
      ],
      "temperature": 0.2,
      "max_tokens": 1200
    });

    let resp_text = self
      .http
      .post("https://api.cerebras.ai/v1/chat/completions")
      .bearer_auth(&self.api_key)
      .json(&req)
      .send()
      .await?
      .error_for_status()? 
      .text()
      .await?;

    let resp: ChatResponse = serde_json::from_str(&resp_text).map_err(|_| CerebrasError::Invalid)?;

    if let Some(err) = resp.error.as_ref() {
      return Err(CerebrasError::Api(err.message.clone().unwrap_or_else(|| "unknown Cerebras error".into())));
    }

    resp.choices.first()
      .and_then(|choice| {
        choice.message.content.as_ref().and_then(|blocks| {
          blocks.iter()
            .find_map(|block| block.text.as_deref())
            .map(|s| s.to_string())
        })
      })
      .filter(|content| !content.trim().is_empty())
      .ok_or(CerebrasError::Invalid)
  }
}

fn build_prompt(settings: &ReminderSettings, events: &[CalendarEvent], weather: Option<&str>) -> String {
  let mut prompt = String::new();
  prompt.push_str("Generate AI reminder email copy for W9 brand. JSON only.\n");
  prompt.push_str(&format!("Language: {}\n", resolve_language(settings)));
  prompt.push_str(&format!(
    "Summary style: {}\n",
    summary_style_label(&settings.summary_style)
  ));
  prompt.push_str("Events (ISO8601 in timezone, include location if any):\n");
  for event in events {
    prompt.push_str(&format!(
      "- {} from {} to {} at {}\n",
      event.summary,
      event.start,
      event.end,
      event.location.as_deref().unwrap_or("N/A"),
    ));
  }
  if let Some(weather) = weather {
    prompt.push_str("Weather note: ");
    prompt.push_str(weather);
    prompt.push('\n');
  }
  prompt.push_str("Return stringified JSON.");
  prompt
}

fn resolve_language(settings: &ReminderSettings) -> String {
  match (&settings.language[..], &settings.custom_language) {
    ("custom", Some(custom)) => custom.clone(),
    _ => settings.language.clone(),
  }
}

fn summary_style_label(style: &SummaryStyle) -> &'static str {
  match style {
    SummaryStyle::Concise => "concise",
    SummaryStyle::Detailed => "detailed",
    SummaryStyle::Bullet => "bullet",
  }
}
