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
  let sanitized = sanitize_control_chars(cerebras_payload);
  let parsed: CerebrasPayload = serde_json::from_str(&sanitized).map_err(|err| {
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
        "<tr><td style=\"padding:0 0 20px 0;\"><table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" width=\"100%\" style=\"border:1px solid #2D2D2D;background:#E8E6DE;\"><tr><td><img src=\"{}\" alt=\"Daily visual\" style=\"display:block;width:100%;height:auto;\" /></td></tr></table></td></tr>",
        url
      )
    })
    .unwrap_or_else(|| {
      "<tr><td style=\"padding:0 0 20px 0;\"><table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" width=\"100%\" style=\"border:1px solid #2D2D2D;background:#ECEAE0;\"><tr><td style=\"padding:32px 0;text-align:center;letter-spacing:0.25em;color:#2D2D2D;\">IMAGE WINDOW</td></tr></table></td></tr>".to_string()
    });

  let sanitized = sanitize_html_body(inner);
  let html_body = format!(
    "<tr><td style=\"color:#2D2D2D;font-size:14px;line-height:1.7;padding:0 0 12px 0;\">{}</td></tr>",
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
<body style="margin:0;padding:16px;background:#E4E1D8;font-family:'Courier New',Courier,monospace;">
  <table role="presentation" cellpadding="0" cellspacing="0" width="100%">
    <tr>
      <td align="center">
        <table role="presentation" cellpadding="0" cellspacing="0" width="100%" style="width:100%;max-width:640px;background:#F9F9F7;border:1px solid #2D2D2D;padding:24px;box-sizing:border-box;">
          <tr><td>
            <table role="presentation" cellpadding="0" cellspacing="0" width="100%" style="border:1px solid #2D2D2D;border-collapse:collapse;font-size:13px;text-transform:uppercase;color:#2D2D2D;margin-bottom:20px;">
              <tr>
                <td style="width:33%;border-right:1px solid #2D2D2D;padding:10px;text-align:center;letter-spacing:0.2em;">{day_label}</td>
                <td style="width:34%;border-right:1px solid #2D2D2D;padding:10px;text-align:center;letter-spacing:0.15em;">{date_label}</td>
                <td style="width:33%;padding:10px;text-align:center;">{header_icons}</td>
              </tr>
            </table>
            <table role="presentation" cellpadding="0" cellspacing="0" width="100%">
              {image_block}
              {html_body}
            </table>
            <table role="presentation" cellpadding="0" cellspacing="0" width="100%" style="border-top:1px solid #2D2D2D;margin-top:20px;padding-top:16px;color:#2D2D2D;font-size:13px;">
              <tr>
                <td style="padding:0 0 12px 0;text-align:center;">{quote}</td>
              </tr>
              <tr>
                <td style="padding:12px 0;text-align:center;">{barcode}</td>
              </tr>
            </table>
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
          // Skip other control characters
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
