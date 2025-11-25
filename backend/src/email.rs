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

fn html_escape(input: &str) -> String {
  input
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
    .replace('\'', "&#x27;")
}

fn wrap_html(inner: &str, weather: Option<&str>, image_url: Option<&str>) -> String {
  let weather_block = weather
    .map(|w| {
      let escaped = html_escape(w);
      format!(
        "<div style=\"border:2px dashed #fff;padding:12px;margin:0 0 24px 0;font-size:14px;line-height:1.5;color:#fff;\">{}</div>",
        escaped
      )
    })
    .unwrap_or_default();
  
  let image_block = image_url
    .map(|url| {
      format!(
        "<div style=\"margin:0 0 24px 0;text-align:center;\"><img src=\"{}\" alt=\"Daily visual\" style=\"max-width:100%;height:auto;border:2px solid #fff;display:block;margin:0 auto;\" /></div>",
        html_escape(url)
      )
    })
    .unwrap_or_default();

  // html_body from Cerebras is already HTML, but we need to ensure all text has proper color
  // Sanitize: remove any structural elements that shouldn't be there
  let sanitized = sanitize_html_body(inner);
  // Wrap content in a div that ensures text color is set
  let html_body = format!(
    r#"<div style="color:#fdfdfd;">{}</div>"#,
    sanitized
  );

  format!(
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>W9 Daily Reminder</title>
</head>
<body style="background:#050505;padding:32px;font-family:'Courier New',Courier,monospace;">
  <table role="presentation" cellpadding="0" cellspacing="0" width="100%">
    <tr>
      <td align="center">
        <table role="presentation" cellpadding="0" cellspacing="0" width="100%" style="max-width:640px;border:2px solid #fdfdfd;padding:28px;background:#000;">
          <tr><td style="text-align:left;">
            <table role="presentation" cellpadding="0" cellspacing="0" style="margin-bottom:24px;">
              <tr>
                <td style="width:42px;height:42px;border:2px solid #fdfdfd;text-align:center;vertical-align:middle;font-weight:bold;color:#fdfdfd;line-height:42px;font-size:16px;padding:0;margin:0;">W9</td>
                <td style="padding-left:12px;vertical-align:middle;">
                  <div style="color:#fdfdfd;font-size:18px;letter-spacing:0.1em;text-transform:uppercase;">W9 Daily Reminders</div>
                  <div style="color:#9a9a9a;font-size:12px;">AI-assisted daily briefings</div>
                </td>
              </tr>
            </table>
            {weather_block}
            {image_block}
            <div style="color:#fdfdfd;font-size:15px;line-height:1.6;font-family:'Courier New',Courier,monospace;margin-bottom:24px;">
              {html_body}
            </div>
            <hr style="border:none;border-top:2px solid #1a1a1a;margin:32px 0;" />
            <p style="margin:0;color:#686868;font-size:11px;line-height:1.4;text-transform:uppercase;">Console generated · zai-glm-4.6</p>
          </td></tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#,
    weather_block = weather_block,
    image_block = image_block,
    html_body = html_body
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

fn sanitize_html_body(html: &str) -> String {
  // Remove common structural elements that Cerebras might incorrectly add
  // Focus on removing headers, section dividers that shouldn't be in content
  let mut cleaned = html.to_string();
  
  // Remove h1-h6 tags (both opening and closing) - these are structural, not content
  for i in 1..=6 {
    let open_tag = format!("<h{}", i);
    let close_tag = format!("</h{}>", i);
    
    // Remove opening tags
    while let Some(start) = cleaned.find(&open_tag) {
      if let Some(end) = cleaned[start..].find('>') {
        cleaned.replace_range(start..start + end + 1, "");
      } else {
        break;
      }
    }
    
    // Remove closing tags
    while let Some(pos) = cleaned.find(&close_tag) {
      cleaned.replace_range(pos..pos + close_tag.len(), "");
    }
  }
  
  // Remove hr tags (horizontal rules - structural dividers)
  while let Some(start) = cleaned.find("<hr") {
    if let Some(end) = cleaned[start..].find('>') {
      cleaned.replace_range(start..start + end + 1, "");
    } else {
      break;
    }
  }
  
  // Clean up extra whitespace
  cleaned = cleaned.trim().to_string();
  
  // If the result is empty or just whitespace, return a simple paragraph
  if cleaned.trim().is_empty() {
    return "<p>No events scheduled.</p>".to_string();
  }
  
  cleaned
}
