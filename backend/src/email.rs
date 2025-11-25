use crate::models::{ReminderPreview, ReminderSettings};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
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

  let html = wrap_html(&parsed.html_body, weather_advisory.as_deref(), image_url.as_deref(), settings);
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

fn wrap_html(inner: &str, weather: Option<&str>, image_url: Option<&str>, settings: &ReminderSettings) -> String {
  let (day_label, date_label) = resolve_temporal_labels(&settings.timezone);
  let header_icons = build_header_icons();

  let image_block = image_url
    .map(|url| {
      format!(
        "<div style=\"border:1px solid #2D2D2D;margin:0 0 20px 0;padding:0;background:#E8E6DE;\"><img src=\"{}\" alt=\"Daily visual\" style=\"width:100%;height:auto;display:block;\" /></div>",
        url
      )
    })
    .unwrap_or_else(|| {
      "<div style=\"border:1px solid #2D2D2D;margin:0 0 20px 0;padding:40px 0;text-align:center;letter-spacing:0.2em;color:#2D2D2D;background:#ECEAE0;\">IMAGE WINDOW</div>".to_string()
    });

  let sanitized = sanitize_html_body(inner);
  let html_body = format!(
    r#"<div style="color:#2D2D2D;font-size:14px;line-height:1.7;">{}</div>"#,
    sanitized
  );

  let quote = weather
    .map(|w| html_escape(w))
    .unwrap_or_else(|| "“Observe the rhythm of the day.”".into());

  let barcode = build_barcode();

  format!(
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>W9 Daily Reminder</title>
</head>
<body style="margin:0;padding:32px;background:#E4E1D8;font-family:'Courier New',Courier,monospace;">
  <table role="presentation" cellpadding="0" cellspacing="0" width="100%">
    <tr>
      <td align="center">
        <table role="presentation" cellpadding="0" cellspacing="0" width="100%" style="max-width:640px;background:#F9F9F7;border:1px solid #2D2D2D;padding:32px;">
          <tr><td>
            <table role="presentation" cellpadding="0" cellspacing="0" width="100%" style="border:1px solid #2D2D2D;border-collapse:collapse;font-size:13px;text-transform:uppercase;color:#2D2D2D;margin-bottom:24px;">
              <tr>
                <td style="width:33%;border-right:1px solid #2D2D2D;padding:10px;text-align:center;letter-spacing:0.2em;">{day_label}</td>
                <td style="width:34%;border-right:1px solid #2D2D2D;padding:10px;text-align:center;letter-spacing:0.15em;">{date_label}</td>
                <td style="width:33%;padding:10px;text-align:center;">{header_icons}</td>
              </tr>
            </table>
            {image_block}
            {html_body}
            <div style="border-top:1px solid #2D2D2D;margin-top:28px;padding-top:18px;display:flex;flex-wrap:wrap;gap:16px;color:#2D2D2D;">
              <div style="flex:1;min-width:220px;text-align:center;font-size:13px;">{quote}</div>
              <div style="flex:1;min-width:220px;display:flex;justify-content:center;align-items:flex-end;gap:2px;">{barcode}</div>
            </div>
          </td></tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#,
    day_label = day_label,
    date_label = date_label,
    header_icons = header_icons,
    image_block = image_block,
    html_body = html_body,
    quote = quote,
    barcode = barcode
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
  
  // Remove blockquote tags which Gmail treats as quoted text (causing collapsing)
  loop {
    if let Some(start) = cleaned.find("<blockquote") {
      if let Some(end) = cleaned[start..].find('>') {
        cleaned.replace_range(start..start + end + 1, "");
      } else {
        break;
      }
    } else {
      break;
    }
  }
  while let Some(pos) = cleaned.find("</blockquote>") {
    cleaned.replace_range(pos..pos + "</blockquote>".len(), "");
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

fn resolve_temporal_labels(tz_name: &str) -> (String, String) {
  let tz: Tz = tz_name.parse().unwrap_or(chrono_tz::UTC);
  let now: DateTime<Tz> = Utc::now().with_timezone(&tz);
  let day = now.format("%A").to_string();
  let date = now.format("%d.%m.%Y").to_string();
  (day, date)
}

fn build_header_icons() -> String {
  let icons = ["USR", "CLK", "DOC"];
  icons
    .iter()
    .map(|label| {
      format!(
        "<span style=\"display:inline-block;border:1px solid #2D2D2D;padding:4px 8px;margin:0 2px;font-size:11px;letter-spacing:0.15em;\">{}</span>",
        label
      )
    })
    .collect::<Vec<_>>()
    .join("")
}

fn build_barcode() -> String {
  let pattern = [4, 2, 1, 3, 2, 5, 1, 4, 2, 3, 1, 4];
  let mut dark = true;
  pattern
    .iter()
    .map(|width| {
      let color = if dark { "#2D2D2D" } else { "transparent" };
      dark = !dark;
      format!(
        "<span style=\"display:inline-block;width:{}px;height:48px;background:{};\"></span>",
        width, color
      )
    })
    .collect::<Vec<_>>()
    .join("")
}
