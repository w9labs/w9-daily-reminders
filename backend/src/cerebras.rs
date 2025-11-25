use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

use crate::models::{CalendarEvent, ReminderSettings, SummaryStyle};
use chrono_tz::Tz;

#[derive(Debug, Error)]
pub enum CerebrasError {
  #[error("missing api key")]
  MissingKey,
  #[error("request failed: {0}")]
  Request(#[from] reqwest::Error),
  #[error("response missing data: {0}")]
  Invalid(String),
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
  #[serde(default)]
  content: Option<String>,
  #[serde(default)]
  reasoning: Option<String>,
  role: String,
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
          "content": "You are W9 Reminders AI. Output ONLY valid JSON with keys subject, preview, html_body, text_body, image_prompt. The html_body must contain ONLY the event list content as simple HTML (use <p>, <ul>, <li>, <strong>, <br> tags only). Do NOT include: headers, titles, section dividers, h1-h6 tags, divs with classes, or any structural layout elements. Just the event content. The image_prompt must describe a wide cinematic film or painted image with a nostalgic, contemplative mood, referencing the current day's or week's schedule. Emphasize muted palettes, natural light, film grain, and atmospheric storytelling similar to: \"A wide cinematic film or painted image with a nostalgic, contemplative mood. All images share a muted color palette, natural light, film grain, and a quiet, peaceful atmosphere.\" Tailor the prompt to the actual events. No explanations, no markdown, just the JSON object."
        },
        {
          "role": "user",
          "content": instructions
        }
      ],
      "temperature": 1.0,
      "max_tokens": 4000,
      "response_format": {
        "type": "json_schema",
        "json_schema": {
          "name": "w9_daily_reminder",
          "strict": true,
          "schema": schema_definition()
        }
      },
      "disable_reasoning": true
    });

    let mut last_error = CerebrasError::Invalid("unknown error".into());

    for attempt in 1..=3 {
      let response = self
        .http
        .post("https://api.cerebras.ai/v1/chat/completions")
        .bearer_auth(&self.api_key)
        .json(&req)
        .send()
        .await;

      let response = match response {
        Ok(r) => r,
        Err(e) => {
          tracing::warn!(?e, attempt, "cerebras request failed");
          last_error = CerebrasError::Request(e);
          continue;
        }
      };

      let status = response.status();
      let resp_text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
          tracing::warn!(?e, attempt, "failed to read response text");
          last_error = CerebrasError::Request(e);
          continue;
        }
      };

      if !status.is_success() {
        tracing::error!(%status, body = %resp_text, attempt, "cerebras api error");
        last_error = CerebrasError::Api(format!("HTTP {}: {}", status, resp_text));
        continue;
      }

      let resp: ChatResponse = match serde_json::from_str(&resp_text) {
        Ok(r) => r,
        Err(err) => {
          tracing::error!(body = %resp_text, error = ?err, attempt, "failed to parse Cerebras response wrapper");
          last_error = CerebrasError::Invalid("failed to parse Cerebras response".into());
          continue;
        }
      };

      if let Some(err) = resp.error.as_ref() {
        last_error = CerebrasError::Api(err.message.clone().unwrap_or_else(|| "unknown Cerebras error".into()));
        continue;
      }

      let content = match resp.choices.first().and_then(|choice| {
        choice.message.content.as_ref().or_else(|| choice.message.reasoning.as_ref())
      }).filter(|text| !text.trim().is_empty()) {
        Some(c) => c,
        None => {
          tracing::error!(body = %resp_text, attempt, "Cerebras response missing textual content");
          last_error = CerebrasError::Invalid("missing textual content in response".into());
          continue;
        }
      };

      let sanitized_content = sanitize_control_chars(content);
      match extract_json_from_text(&sanitized_content) {
        Ok(extracted) => return Ok(extracted),
        Err(e) => {
          tracing::warn!(error = ?e, body = %sanitized_content, attempt, "failed to extract JSON from response");
          // Attempt to repair if it's the last attempt
          if attempt == 3 {
             if let Ok(repaired) = repair_truncated_json(&sanitized_content) {
                 tracing::info!("successfully repaired truncated JSON");
                 return Ok(repaired);
             }
          }
          last_error = e;
          continue;
        }
      }
    }

    Err(last_error)
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
  prompt.push_str("IMPORTANT: Output the JSON keys in this EXACT order: subject, preview, text_body, image_prompt, html_body. This is critical.\n");
  prompt.push_str("The html_body field must contain ONLY the event content as HTML. Use simple HTML tags like <p>, <ul>, <li>, <strong>. Do NOT include headers, titles, section dividers, or any structural elements. Just the event list content.\n");
  prompt.push_str("Image prompt guidelines: describe a wide cinematic film or painted image that mirrors the emotional tone of the upcoming schedule. Use muted colors, natural light, film grain, and contemplative mood. Blend motifs from the provided example (urban night desk, minimalist sky, hillside hut, person in tall grass, classical hands, coastal train) with the actual events to keep it fresh.\n");
  prompt.push_str("Example image prompt to emulate: \"A wide cinematic film or painted image with a nostalgic, contemplative mood. The image includes a moody urban scene with a laptop by a window at night, minimalist blue sky with clouds over an industrial structure, a lone wooden hut on rolling green hills with dramatic shadows, a person lying face down in tall grass, a close-up fragment of a classical painting showing two hands reaching for each other, and a coastal train passing by a turquoise ocean. All images share a muted color palette, natural light, film grain, and a quiet, peaceful atmosphere.\"\n");
  prompt.push_str("Events (Local Time):\n");
  let tz: Tz = settings.timezone.parse().unwrap_or(chrono_tz::UTC);
  for event in events {
    let start = event.start.with_timezone(&tz);
    let end = event.end.with_timezone(&tz);
    prompt.push_str(&format!(
      "- {} from {} to {} at {}\n",
      event.summary,
      start.format("%Y-%m-%d %H:%M"),
      end.format("%Y-%m-%d %H:%M"),
      event.location.as_deref().unwrap_or("N/A"),
    ));
  }
  if let Some(weather) = weather {
    prompt.push_str("Weather note: ");
    prompt.push_str(weather);
    prompt.push('\n');
    prompt.push_str("Note: Weather information will be displayed separately in the email template. Do NOT include weather in html_body.\n");
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

fn extract_json_from_text(text: &str) -> Result<String, CerebrasError> {
  // Try to find JSON object in the text
  // Look for { ... } pattern
  if let Some(start) = text.find('{') {
    let mut depth = 0;
    for (i, ch) in text[start..].char_indices() {
      match ch {
        '{' => depth += 1,
        '}' => {
          depth -= 1;
          if depth == 0 {
            return Ok(text[start..=start + i].to_string());
          }
        }
        _ => {}
      }
    }
    // Found start but no end
    return Err(CerebrasError::Invalid("incomplete JSON response".into()));
  }
  // No JSON object found
  Err(CerebrasError::Invalid("no JSON object found in response".into()))
}

fn repair_truncated_json(text: &str) -> Result<String, CerebrasError> {
  // Simple repair: assume it's a JSON object that got cut off.
  // Find the start
  let start = text.find('{').ok_or_else(|| CerebrasError::Invalid("no JSON start found".into()))?;
  let mut working = text[start..].to_string();
  
  // Try closing it with various suffixes
  let suffixes = ["}", "\"}", "\"]}", "\"]\"}"];
  
  for suffix in suffixes {
      let candidate = format!("{}{}", working, suffix);
      if serde_json::from_str::<serde_json::Value>(&candidate).is_ok() {
          return Ok(candidate);
      }
  }
  
  // If simple suffixes fail, try to backtrack to the last valid comma or key?
  // That's too complex. Let's just try to close the last open string if possible.
  // If the last non-whitespace char is not '"' or '}' or ']', it might be inside a string or number.
  
  // Fallback: just return error if we can't easily fix it.
  Err(CerebrasError::Invalid("could not repair truncated JSON".into()))
}

fn sanitize_control_chars(input: &str) -> String {
  let mut sanitized = String::with_capacity(input.len());
  let mut in_string = false;
  let mut escape_next = false;

  for ch in input.chars() {
    if in_string {
      if escape_next {
        sanitized.push(ch);
        escape_next = false;
        continue;
      }
      match ch {
        '\\' => {
          sanitized.push(ch);
          escape_next = true;
        }
        '"' => {
          sanitized.push(ch);
          in_string = false;
        }
        '\n' => sanitized.push_str("\\n"),
        '\r' => sanitized.push_str("\\r"),
        '\t' => sanitized.push_str("\\t"),
        c if c.is_control() => {
          // Skip other control characters (like \u0010)
        }
        _ => sanitized.push(ch),
      }
    } else {
      sanitized.push(ch);
      if ch == '"' {
        in_string = true;
      }
    }
  }

  sanitized
}

fn schema_definition() -> serde_json::Value {
  json!({
    "type": "object",
    "additionalProperties": false,
    "properties": {
      "subject": { "type": "string" },
      "preview": { "type": "string" },
      "html_body": { "type": "string" },
      "text_body": { "type": "string" },
      "image_prompt": { "type": "string" }
    },
    "required": ["subject", "preview", "html_body", "text_body", "image_prompt"]
  })
}
