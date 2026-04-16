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

    let html = wrap_html(
        &parsed.html_body,
        &parsed.preview,
        weather_advisory.as_deref(),
        image_url.as_deref(),
        settings,
    );
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

fn wrap_html(
    inner: &str,
    preview_text: &str,
    weather: Option<&str>,
    image_url: Option<&str>,
    settings: &ReminderSettings,
) -> String {
    let (day_label, date_label) = resolve_temporal_labels(&settings.timezone);

    let image_block = image_url
        .map(|url| {
            format!(
                "<tr><td style=\"padding:0;\"><img src=\"{}\" alt=\"Daily visual\" style=\"display:block;width:100%;height:auto;border-bottom:1px solid #2D2D2D;\" /></td></tr>",
                url
            )
        })
        .unwrap_or_else(|| {
            "<tr><td style=\"padding:40px 0;text-align:center;letter-spacing:0.2em;color:#2D2D2D;border-bottom:1px solid #2D2D2D;background:#ECEAE0;\">IMAGE WINDOW</td></tr>".to_string()
        });

    let sanitized = sanitize_html_body(inner);
    let html_body = format!(
        "<tr><td class=\"content-padding\" style=\"font-family:'Roboto', 'Helvetica Neue', Helvetica, Arial, sans-serif;color:#1a1a1a;font-size:16px;line-height:1.6;padding:32px 40px;\">{}</td></tr>",
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
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Roboto+Mono:wght@400;700&family=Roboto:wght@400;500;700&display=swap" rel="stylesheet">
  <style>
     @import url('https://fonts.googleapis.com/css2?family=Roboto:wght@400;500;700&family=Roboto+Mono:wght@400;700&display=swap');
     .task-list {{ margin: 12px 0; padding-left: 20px; }}
     .task-list li {{ margin: 8px 0; line-height: 1.6; }}
     .task-list li em {{ color: #666; font-style: italic; font-size: 0.95em; }}
     @media screen and (max-width: 600px) {{
       .wrapper {{ padding: 0 !important; }}
       .container {{ width: 100% !important; border-left: none !important; border-right: none !important; }}
       .content-padding {{ padding: 24px 20px !important; }}
       .header-padding {{ padding: 16px 20px !important; }}
       .mobile-hidden {{ display: none !important; }}
     }}
   </style>
</head>
<body style="margin:0;padding:0;background-color:#E4E1D8;font-family:'Roboto', 'Helvetica Neue', Helvetica, Arial, sans-serif;color:#1a1a1a;">
  <div style="display:none;font-size:1px;color:#E4E1D8;line-height:1px;max-height:0px;max-width:0px;opacity:0;overflow:hidden;">
    {preview_text}
  </div>
  <table role="presentation" cellpadding="0" cellspacing="0" width="100%" style="background-color:#E4E1D8;">
    <tr>
      <td align="center" class="wrapper" style="padding: 32px 16px;">
        <table role="presentation" cellpadding="0" cellspacing="0" width="100%" class="container" style="max-width:600px;background-color:#F9F9F7;border:1px solid #2D2D2D;box-shadow: 4px 4px 0px rgba(45, 45, 45, 0.1);">
          <!-- Header -->
          <tr>
            <td class="header-padding" style="padding: 20px 40px; border-bottom: 1px solid #2D2D2D; background-color: #F9F9F7;">
              <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
                <tr>
                  <td style="width:33%;font-family:'Roboto Mono', 'Courier New', monospace; font-size: 12px; letter-spacing: 0.1em; text-transform: uppercase;">{day_label}</td>
                  <td align="center" style="width:34%;">
                    <span style="display:inline-block;border:1px solid #2D2D2D;width:32px;height:32px;line-height:32px;text-align:center;font-family:'Roboto Mono', 'Courier New', monospace;font-size:12px;font-weight:bold;color:#2D2D2D;">W9</span>
                  </td>
                  <td align="right" style="width:33%;font-family:'Roboto Mono', 'Courier New', monospace; font-size: 12px; letter-spacing: 0.1em; text-transform: uppercase;">{date_label}</td>
                </tr>
              </table>
            </td>
          </tr>

          <!-- Image -->
          {image_block}

          <!-- Content -->
          {html_body}

          <!-- Footer -->
          <tr>
            <td class="content-padding" style="padding: 0 40px 40px 40px; text-align: center;">
              <div style="border-top: 1px solid #2D2D2D; padding-top: 24px; margin-bottom: 24px;">
                <p style="font-family:'Roboto Mono', 'Courier New', monospace; font-size: 13px; font-style: italic; margin: 0; color: #4a4a4a;">{quote}</p>
              </div>
              {barcode}
              <div style="margin-top: 24px; font-family:'Roboto Mono', 'Courier New', monospace; font-size: 10px; color: #888; letter-spacing: 0.1em; text-transform: uppercase;">
                W9 Daily Reminder System
              </div>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#,
        preview_text = preview_text,
        day_label = day_label,
        date_label = date_label,
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
    let mut cleaned = html.to_string();

    for i in 1..=6 {
        let open_tag = format!("<h{}", i);
        let close_tag = format!("</h{}>", i);

        while let Some(start) = cleaned.find(&open_tag) {
            if let Some(end) = cleaned[start..].find('>') {
                cleaned.replace_range(start..start + end + 1, "");
            } else {
                break;
            }
        }

        while let Some(pos) = cleaned.find(&close_tag) {
            cleaned.replace_range(pos..pos + close_tag.len(), "");
        }
    }

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

    while let Some(start) = cleaned.find("<hr") {
        if let Some(end) = cleaned[start..].find('>') {
            cleaned.replace_range(start..start + end + 1, "");
        } else {
            break;
        }
    }

    cleaned = cleaned.trim().to_string();

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
                c if c.is_control() => {}
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
