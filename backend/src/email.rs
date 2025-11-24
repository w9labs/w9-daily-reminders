use crate::models::{ReminderPreview, ReminderSettings};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailBuildError {
  #[error("invalid cerebras payload: {0}")]
  InvalidPayload(String),
  #[error("serde error: {0}")]
  Serde(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct CerebrasPayload {
  subject: String,
  preview: String,
  html_body: String,
  text_body: String,
  image_prompt: Option<String>,
}

pub fn build_preview(
  settings: &ReminderSettings,
  cerebras_payload: &str,
  weather_advisory: Option<String>,
  image_url: Option<String>,
) -> Result<ReminderPreview, EmailBuildError> {
  let parsed: CerebrasPayload = serde_json::from_str(cerebras_payload).map_err(|err| {
    EmailBuildError::InvalidPayload(format!("{} · raw: {}", err, cerebras_payload))
  })?;

  let html = wrap_html(&parsed.html_body, weather_advisory.as_deref(), image_url.as_deref());
  let text = wrap_text(&parsed.text_body, weather_advisory.as_deref());

  Ok(ReminderPreview {
    subject: parsed.subject,
    html,
    text,
    weather_advisory,
    image_url,
    generated_language: resolve_language(settings),
  })
}

fn wrap_html(inner: &str, weather: Option<&str>, image_url: Option<&str>) -> String {
  let weather_block = weather
    .map(|w| format!("<p style=\"border:2px dashed #fff;padding:12px;margin:0 0 16px 0;font-size:14px\">{w}</p>"))
    .unwrap_or_default();
  let image_block = image_url
    .map(|url| format!("<p style=\"margin:0 0 16px 0;font-size:14px\">Visual cue: <a href=\"{url}\">{url}</a></p>"))
    .unwrap_or_default();

  format!(
    "<!doctype html><html><body style=\"background:#000;color:#fff;font-family:'Courier New',Courier,monospace;padding:32px\"><table width=\"100%\" cellpadding=0 cellspacing=0 style=\"max-width:640px;margin:0 auto;border:2px solid #fff\"><tr><td style=\"padding:24px\"><h1 style=\"text-transform:uppercase;font-size:20px;margin-bottom:16px\">W9 Daily Reminder</h1>{weather_block}{image_block}<div style=\"font-size:15px;line-height:1.5\">{inner}</div><p style=\"margin-top:24px;font-size:13px;text-transform:uppercase\">Console generated · zai-glm-4.6</p></td></tr></table></body></html>"
  )
}

fn wrap_text(inner: &str, weather: Option<&str>) -> String {
  match weather {
    Some(w) => format!("W9 Daily Reminder\nWeather: {w}\n\n{inner}"),
    None => format!("W9 Daily Reminder\n\n{inner}"),
  }
}

fn resolve_language(settings: &ReminderSettings) -> String {
  match (&settings.language[..], &settings.custom_language) {
    ("custom", Some(custom)) => custom.clone(),
    _ => settings.language.clone(),
  }
}
