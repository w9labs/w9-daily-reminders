mod cerebras;
mod cloudflare;
mod email;
mod google;
mod models;
mod nvidia;
mod pollinations;
mod store;
mod w9mail;
mod weather;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use chrono::{Datelike, Duration, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::cerebras::{CerebrasClient, CerebrasError};
use crate::cloudflare::CloudflareAiClient;
use crate::email::{build_preview, EmailBuildError};
use crate::google::{GoogleClient, GoogleError};
use crate::models::{
    AiProvider, ApiResponse, CalendarEvent, GoogleAuthPayload, GoogleAuthResult,
    GoogleAuthStartResponse, GoogleTokens, HealthStatus, ImageModelOptions, ImageProvider,
    ReminderPreview, ReminderSettings, ScheduleType, Todo, WeekStartDay,
};
use crate::nvidia::{NvidiaClient, NvidiaError, NvidiaModel};
use crate::pollinations::{PollinationsClient, PollinationsError};
use crate::store::{DataStore, ExecutionLogEntry, UserPreviewCache};
use crate::w9mail::{SendEmailPayload, W9MailClient, W9MailError};
use crate::weather::{WeatherClient, WeatherError};

const CSS: &str = include_str!("../infra/templates/voxel.css");
const W9_DB: &str = "https://db.w9.nu";
const W9_MAIL_TOKEN: &str = "mail-w9-reminders-9a2e8b9c-c85e-42dd-914a-37e37595479d-1776146894";

#[derive(Clone)]
pub struct AppState {
    pub store: DataStore,
    pub http_client: reqwest::Client,
    pub weather: Arc<WeatherClient>,
    pub cerebras: Arc<Option<CerebrasClient>>,
    pub nvidia: Arc<Option<NvidiaClient>>,
    pub pollinations: Arc<PollinationsClient>,
    pub cloudflare: Arc<Option<CloudflareAiClient>>,
    pub google: Arc<Option<GoogleClient>>,
    pub mail_client: Arc<W9MailClient>,
    pub mail_api_base: String,
}

// ============ Session ============

fn set_s(j: CookieJar, t: String) -> CookieJar {
    j.add(
        axum_extra::extract::cookie::Cookie::build(("w9_rem_session", t))
            .path("/")
            .http_only(true)
            .same_site(axum_extra::extract::cookie::SameSite::Lax)
            .max_age(time::Duration::days(7))
            .build(),
    )
}

fn clr_s(j: CookieJar) -> CookieJar {
    j.remove(axum_extra::extract::cookie::Cookie::from("w9_rem_session"))
}

fn get_s(j: &CookieJar) -> Option<String> {
    j.get("w9_rem_session").map(|c| c.value().to_string())
}

async fn verify(a: &AppState, t: &str) -> Option<serde_json::Value> {
    let r = a
        .http_client
        .get(format!("{}/api/auth/me", W9_DB))
        .header("Authorization", format!("Bearer {}", t))
        .send()
        .await
        .ok()?;
    if r.status().is_success() {
        r.json().await.ok()
    } else {
        None
    }
}

async fn require_email(j: &CookieJar, a: &AppState) -> Option<String> {
    let t = get_s(j)?;
    let user = verify(a, &t).await?;
    user.get("email").and_then(|v| v.as_str()).map(String::from)
}

// ============ Layout ============

fn layout(t: &str, b: &str, n: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"/><link rel="icon" type="image/svg+xml" href="/w9-logo/favicon.svg"/><meta name="viewport" content="width=device-width,initial-scale=1.0"/><title>{} — W9 Reminders</title><style>{}</style></head><body><div class="app"><nav class="nav"><div class="nav-inner"><a href="/" class="brand"><img src="/w9-logo/workmark-transparent.svg" alt="W9 Labs"/><span class="brand-text">Reminders</span></a><div class="nav-links">{}</div></div></nav><main class="app-main">{}</main><footer class="footer"><img class="footer-logo" src="/w9-logo/workmark-transparent.svg" alt="W9 Labs"/><p>W9 Daily Reminders — AI Calendar Digest</p><p class="text-xs text-muted">Google Calendar + AI + Email</p><a href="https://pollinations.ai" target="_blank" rel="noopener"><img src="https://img.shields.io/badge/Built%20with-Pollinations-8a2be2?style=for-the-badge&logo=data:image/png;base64,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&logoColor=white&labelColor=6a0dad" alt="Built with Pollinations" style="margin-top:12px;"/></a></footer></div></body></html>"#,
        t, CSS, n, b
    )
}

fn pub_layout(t: &str, b: &str) -> String {
    layout(
        t,
        b,
        r#"<a href="/login">Login</a><a href="/settings">Settings</a>"#,
    )
}

fn user_layout(t: &str, b: &str) -> String {
    layout(
        t,
        b,
        r#"<a href="/settings">Settings</a><a href="/preview">Preview</a><a href="/system">System</a><a href="/logout">Logout</a>"#,
    )
}

// ============ HTML Pages ============

fn home_html() -> String {
    pub_layout(
        "W9 Reminders",
        r#"<div class="hero"><img class="hero-logo" src="/w9-logo/logo-landscape-transparent.svg" alt="W9 Labs"/><h1>W9 Daily Reminders</h1><p class="hero-sub">AI-powered daily email digests from your Google Calendar</p><p class="hero-muted">Never miss a meeting again</p><div class="hero-actions"><a href="https://db.w9.nu/oauth/authorize?redirect_uri=https://reminder.w9.nu/oauth/callback&response_type=code&client_id=w9-reminders" class="btn" onclick="const w=window.open(this.href,'w9-reminders-login','width=520,height=720'); if (w) { w.focus(); return false; }">Login with W9</a></div></div><div class="grid"><div class="card"><h3>📅 Google Calendar</h3><p>Connect your Google Calendar for daily event summaries.</p></div><div class="card"><h3>🤖 AI Summaries</h3><p>AI generates personalized daily summaries with images.</p></div><div class="card"><h3>📧 Email Delivery</h3><p>Beautiful HTML emails delivered via W9 Mail every morning.</p></div><div class="card"><h3>🌤️ Weather</h3><p>Location-based weather advisories with practical recommendations.</p></div><div class="card"><h3>🎨 AI Images</h3><p>Dynamic visuals from Pollinations.ai or Cloudflare Workers AI.</p></div><div class="card"><h3>✅ Google Tasks</h3><p>Your tasks and events together in one daily briefing.</p></div></div>"#,
    )
}

fn login_html() -> String {
    pub_layout(
        "Login",
        r#"<div class="card" style="max-width:420px;margin:3rem auto;text-align:center"><h1>⏰ W9 Reminders</h1><p class="text-sm text-muted mb-2">Sign in with W9 DB</p><a href="https://db.w9.nu/oauth/authorize?redirect_uri=https://reminder.w9.nu/oauth/callback&response_type=code&client_id=w9-reminders" class="btn" style="width:100%" onclick="const w=window.open(this.href,'w9-reminders-login','width=520,height=720'); if (w) { w.focus(); return false; }">Login with W9 DB</a></div>"#,
    )
}

fn popup_close_html(target: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>W9 Reminders Login</title></head><body><script>(function(){{const target = {target:?}; if (window.opener && !window.opener.closed) {{ try {{ window.opener.location.href = target; window.opener.focus(); }} catch (_) {{}} window.close(); }} else {{ window.location.replace(target); }}}})();</script><p>Signing you in…</p></body></html>"#
    )
}

fn settings_html(
    settings: &ReminderSettings,
    google_connected: bool,
    ai_models: &AiModelsList,
    msg: Option<&str>,
) -> String {
    let al = msg
        .map(|x| format!(r#"<div class="alert alert--ok">{}</div>"#, x))
        .unwrap_or_default();
    let google_status = if google_connected {
        r#"<div class="card mt-2" style="background:#1a3a1a;border:1px solid #2d5a2d"><h3>✅ Google Calendar & Tasks</h3><p class="text-muted">Connected and syncing events/tasks.</p><a href="/google/connect" class="btn" style="margin-top:1rem">Re-connect Google</a></div>"#
    } else {
        r#"<div class="card mt-2" style="background:#2a1a1a;border:1px solid #5a2d2d"><h3>❌ Google Calendar & Tasks</h3><p class="text-muted">Not connected. Connect to include your calendar events and tasks in daily reminders.</p><a href="/google/connect" class="btn" style="margin-top:1rem">🔗 Connect Google Calendar</a></div>"#
    };

    let ai_provider_options = [
        (
            AiProvider::Cerebras,
            "Cerebras",
            settings.ai_provider == AiProvider::Cerebras,
        ),
        (
            AiProvider::Nvidia,
            "NVIDIA NIM",
            settings.ai_provider == AiProvider::Nvidia,
        ),
    ];
    let ai_provider_html: String = ai_provider_options
        .iter()
        .map(|(p, label, sel)| {
            format!(
                r#"<label><input type="radio" name="ai_provider" value="{}" {} /> {}</label>"#,
                match p {
                    AiProvider::Cerebras => "cerebras",
                    AiProvider::Nvidia => "nvidia",
                },
                if *sel { "checked" } else { "" },
                label
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    let cerebras_models = ai_models
        .cerebras
        .iter()
        .map(|m| {
            let sel = settings.cerebras_model.as_deref() == Some(m);
            format!(
                r#"<option value="{}" {}>{}</option>"#,
                m,
                if sel { "selected" } else { "" },
                m
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let nvidia_models = ai_models
        .nvidia
        .iter()
        .map(|(id, label)| {
            let sel = settings.nvidia_model.as_deref() == Some(id);
            format!(
                r#"<option value="{}" {}>{}</option>"#,
                id,
                if sel { "selected" } else { "" },
                label
            )
        })
        .collect::<Vec<_>>()
        .join("");

    user_layout(
        "Settings",
        &format!(
            r#"<div class="card" style="max-width:700px;margin:2rem auto"><h1>⚙️ Reminder Settings</h1>{}<form id="settings-form"><label>Email</label><input type="email" id="user_email" value="{}" required placeholder="you@w9.nu"/><label>Reminder Time</label><input type="time" id="reminder_time" value="{}" required/><label>Timezone</label><input type="text" id="timezone" value="{}" placeholder="Europe/Stockholm"/><label>Language</label><input type="text" id="language" value="{}" placeholder="English"/><label>Weather Location</label><input type="text" id="weather_location" value="{}" placeholder="Stockholm, Sweden"/><label><input type="checkbox" id="include_weather" {} /> Include Weather</label><label><input type="checkbox" id="include_image" {} /> Include AI Image</label><label>Image Provider</label><select id="image_provider"><option value="pollinations" {}>Pollinations</option><option value="cloudflare" {}>Cloudflare</option></select><label>AI Provider</label><div style="display:flex;gap:1rem;margin:0.5rem 0">{}</div><label>Cerebras Model</label><select id="cerebras_model">{}</select><label>NVIDIA Model</label><select id="nvidia_model">{}</select><label>Summary Style</label><select id="summary_style"><option value="concise" {}>Concise</option><option value="detailed" {}>Detailed</option><option value="bullet" {}>Bullet</option></select><label>Schedule Type</label><select id="schedule_type"><option value="day" {}>Day</option><option value="week" {}>Week</option></select><button type="submit" class="btn mt-1" style="width:100%">Save Settings</button></form><div id="settings-msg" class="mt-1"></div>{}</div><script>
document.getElementById('settings-form').addEventListener('submit', async (e) => {{
    e.preventDefault();
    const msg = document.getElementById('settings-msg');
    msg.textContent = 'Saving...';
    msg.className = 'mt-1';
    const aiProvider = document.querySelector('input[name="ai_provider"]:checked')?.value || 'cerebras';
    const body = {{
        userEmail: document.getElementById('user_email').value,
        reminderTime: document.getElementById('reminder_time').value,
        timezone: document.getElementById('timezone').value,
        language: document.getElementById('language').value,
        weatherLocation: document.getElementById('weather_location').value,
        includeWeather: document.getElementById('include_weather').checked,
        includeImage: document.getElementById('include_image').checked,
        imageProvider: document.getElementById('image_provider').value,
        aiProvider: aiProvider,
        cerebrasModel: document.getElementById('cerebras_model').value || undefined,
        nvidiaModel: document.getElementById('nvidia_model').value || undefined,
        summaryStyle: document.getElementById('summary_style').value,
        scheduleType: document.getElementById('schedule_type').value,
    }};
    try {{
        const res = await fetch('/api/settings', {{ method: 'POST', credentials: 'include', headers: {{'Content-Type':'application/json'}}, body: JSON.stringify(body) }});
        const data = await res.json();
        if (res.ok) {{ msg.textContent = '✅ Settings saved!'; msg.className = 'mt-1 alert alert--ok'; }}
        else {{ msg.textContent = '❌ ' + (data.error || 'Save failed'); msg.className = 'mt-1 alert alert--err'; }}
    }} catch(err) {{ msg.textContent = '❌ Network error'; msg.className = 'mt-1 alert alert--err'; }}
}});
</script>"#,
            al,
            settings.user_email,
            settings.reminder_time,
            settings.timezone,
            settings.language,
            settings.weather_location,
            if settings.include_weather {
                "checked"
            } else {
                ""
            },
            if settings.include_image {
                "checked"
            } else {
                ""
            },
            if settings.image_provider == ImageProvider::Pollinations {
                "selected"
            } else {
                ""
            },
            if settings.image_provider == ImageProvider::Cloudflare {
                "selected"
            } else {
                ""
            },
            ai_provider_html,
            cerebras_models,
            nvidia_models,
            if settings.summary_style == models::SummaryStyle::Concise {
                "selected"
            } else {
                ""
            },
            if settings.summary_style == models::SummaryStyle::Detailed {
                "selected"
            } else {
                ""
            },
            if settings.summary_style == models::SummaryStyle::Bullet {
                "selected"
            } else {
                ""
            },
            if settings.schedule_type == ScheduleType::Day {
                "selected"
            } else {
                ""
            },
            if settings.schedule_type == ScheduleType::Week {
                "selected"
            } else {
                ""
            },
            google_status,
        ),
    )
}

struct AiModelsList {
    cerebras: Vec<String>,
    nvidia: Vec<(String, String)>,
}

fn preview_html(preview: Option<&UserPreviewCache>) -> String {
    let content = match preview {
        Some(p) => format!(
            r#"<div class="card"><h2>Subject: {}</h2><p><strong>Generated:</strong> {}</p><p><strong>Language:</strong> {}</p><hr/><div style="background:#fff;color:#000;padding:1rem;border:1px solid #333;">{}</div><hr/><p><strong>Weather:</strong> {}</p><p><strong>Image:</strong> {}</p></div>"#,
            p.subject, p.generated_at.format("%Y-%m-%d %H:%M UTC"), p.generated_language,
            &p.html,
            p.weather_advisory.as_deref().unwrap_or("N/A"),
            p.image_url.as_deref().unwrap_or("No image"),
        ),
        None => r#"<div class="card"><h2>No preview generated yet</h2><p>Click "Generate Preview" to create your daily reminder.</p></div>"#.to_string(),
    };
    user_layout(
        "Preview",
        &format!(
            r#"<div style="max-width:800px;margin:2rem auto"><h1>📧 Email Preview</h1><div style="display:flex;gap:1rem"><button id="gen-btn" class="btn">Generate Preview</button><button id="send-btn" class="btn" style="background:#2d5a2d">Send Test Email</button></div><div id="preview-msg" class="mt-1"></div>{}</div><script>
document.getElementById('gen-btn').addEventListener('click', async () => {{
    const msg = document.getElementById('preview-msg');
    msg.textContent = 'Generating preview... This may take 30-60s';
    msg.className = 'mt-1';
    try {{
        const res = await fetch('/api/reminders/preview', {{ method: 'POST', credentials: 'include', headers: {{'Content-Type':'application/json'}} }});
        const data = await res.json();
        if (res.ok) {{
            msg.textContent = '✅ Preview generated! Reloading...';
            msg.className = 'mt-1 alert alert--ok';
            setTimeout(() => window.location.reload(), 1500);
        }} else {{
            msg.textContent = '❌ ' + (data.error || 'Generation failed');
            msg.className = 'mt-1 alert alert--err';
        }}
    }} catch(err) {{ msg.textContent = '❌ Network error'; msg.className = 'mt-1 alert alert--err'; }}
}});
document.getElementById('send-btn').addEventListener('click', async () => {{
    const msg = document.getElementById('preview-msg');
    msg.textContent = 'Sending email...';
    msg.className = 'mt-1';
    try {{
        const res = await fetch('/api/reminders/send', {{ method: 'POST', credentials: 'include', headers: {{'Content-Type':'application/json'}} }});
        const data = await res.json();
        if (res.ok) {{ msg.textContent = '✅ Email sent to ' + (data.to || 'your address'); msg.className = 'mt-1 alert alert--ok'; }}
        else {{ msg.textContent = '❌ ' + (data.error || 'Send failed'); msg.className = 'mt-1 alert alert--err'; }}
    }} catch(err) {{ msg.textContent = '❌ Network error'; msg.className = 'mt-1 alert alert--err'; }}
}});
</script>"#,
            content
        ),
    )
}

fn system_html(health: &HealthStatus, log: &[ExecutionLogEntry]) -> String {
    let log_rows: String = log
        .iter()
        .map(|entry| {
            let badge = if entry.email_sent {
                r#"<span class="badge badge--ok">Sent</span>"#
            } else {
                r#"<span class="badge badge--err">Failed</span>"#
            };
            let err = entry
                .error_message
                .as_deref()
                .map(|e| format!(r#" <span class="text-xs text-muted">{}</span>"#, e))
                .unwrap_or_default();
            format!(
                r#"<tr><td class="text-xs">{}</td><td>{}</td><td>{}</td>{}</tr>"#,
                entry.executed_at, entry.events_count, badge, err
            )
        })
        .collect();
    user_layout(
        "System Status",
        &format!(
            r#"<div class="card" style="max-width:700px;margin:2rem auto"><h1>🖥️ System Status</h1><table><tr><th>Metric</th><th>Value</th></tr><tr><td>Scheduler State</td><td>{:?}</td></tr><tr><td>Last Dispatch</td><td>{}</td></tr><tr><td>Next Run</td><td>{}</td></tr><tr><td>Google Connected</td><td>{}</td></tr></table></div><div class="card mt-2" style="max-width:700px;margin:1rem auto"><h2>📊 Execution Log (Last 50)</h2><table><tr><th>Executed</th><th>Events</th><th>Status</th></tr>{}</table></div>"#,
            health.scheduler,
            health
                .last_dispatch
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Never".into()),
            health
                .next_run
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Not scheduled".into()),
            if health.google_connected {
                "✅ Yes"
            } else {
                "❌ No"
            },
            log_rows,
        ),
    )
}

// ============ Routes ============

async fn home() -> Html<String> {
    Html(home_html())
}
async fn login_page() -> Html<String> {
    Html(login_html())
}

async fn oauth_cb(
    State(s): State<AppState>,
    jar: CookieJar,
    Query(q): Query<serde_json::Value>,
) -> impl IntoResponse {
    let code = match q.get("code").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => return Html(login_html()).into_response(),
    };
    let res = match s
        .http_client
        .post(format!("{}/oauth/token", W9_DB))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", "https://reminder.w9.nu/oauth/callback"),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Html(login_html()).into_response(),
    };
    let json = match res.json::<serde_json::Value>().await {
        Ok(j) => j,
        Err(_) => return Html(login_html()).into_response(),
    };
    let token = match json.get("access_token").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return Html(login_html()).into_response(),
    };
    (set_s(jar, token), Html(popup_close_html("/settings"))).into_response()
}

async fn logout(jar: CookieJar) -> impl IntoResponse {
    (clr_s(jar), Redirect::to("/")).into_response()
}

async fn settings_page(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let email = match require_email(&jar, &s).await {
        Some(e) => e,
        None => return Redirect::to("/login").into_response(),
    };
    let _ = s.store.ensure_user(&email).await;
    let settings = match s.store.read_settings(&email).await {
        Ok(st) => st,
        Err(_) => {
            return Html(user_layout(
                "Error",
                "<div class=\"card\"><h1>Failed to load settings</h1></div>",
            ))
            .into_response()
        }
    };
    let health = match s.store.read_health().await {
        Ok(h) => h,
        Err(_) => HealthStatus::default(),
    };
    let models = AiModelsList {
        cerebras: if s.cerebras.as_ref().is_some() {
            CerebrasClient::supported_models()
        } else {
            vec![]
        },
        nvidia: if s.nvidia.as_ref().is_some() {
            NvidiaModel::all()
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect()
        } else {
            vec![]
        },
    };
    Html(settings_html(
        &settings,
        health.google_connected,
        &models,
        None,
    ))
    .into_response()
}

async fn preview_page(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let email = match require_email(&jar, &s).await {
        Some(e) => e,
        None => return Redirect::to("/login").into_response(),
    };
    let cached = s.store.read_preview(&email).await.ok().flatten();
    Html(preview_html(cached.as_ref())).into_response()
}

async fn system_page(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let email = match require_email(&jar, &s).await {
        Some(e) => e,
        None => return Redirect::to("/login").into_response(),
    };
    let health = match s.store.read_health().await {
        Ok(h) => h,
        Err(_) => HealthStatus::default(),
    };
    let log = s
        .store
        .get_execution_log(&email, 50)
        .await
        .unwrap_or_default();
    Html(system_html(&health, &log)).into_response()
}

async fn google_connect(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let email = match require_email(&jar, &s).await {
        Some(e) => e,
        None => return Redirect::to("/login").into_response(),
    };
    let _ = s.store.ensure_user(&email).await;
    let google = match s.google.as_ref().as_ref() {
        Some(g) => g,
        None => {
            return Html(user_layout(
                "Error",
                "<div class=\"card\"><h1>Google OAuth not configured</h1></div>",
            ))
            .into_response()
        }
    };
    let url = google.auth_url(&Uuid::new_v4().to_string());
    Redirect::to(&url).into_response()
}

async fn google_callback(
    State(s): State<AppState>,
    jar: CookieJar,
    Query(q): Query<serde_json::Value>,
) -> impl IntoResponse {
    let email = match require_email(&jar, &s).await {
        Some(e) => e,
        None => return Redirect::to("/login").into_response(),
    };
    let _ = s.store.ensure_user(&email).await;
    let code = match q.get("code").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => {
            let error = q.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
            return Html(user_layout("Google OAuth Error", &format!("<div class=\"card\" style=\"max-width:600px;margin:2rem auto\"><h1>❌ Google OAuth Error</h1><p>Error: {}</p><a href=\"/settings\" class=\"btn\">Back to Settings</a></div>", error))).into_response();
        }
    };
    let google = match s.google.as_ref().as_ref() {
        Some(g) => g,
        None => {
            return Html(user_layout(
                "Error",
                "<div class=\"card\"><h1>Google OAuth not configured</h1></div>",
            ))
            .into_response()
        }
    };
    match google.exchange_code(&code).await {
        Ok(tokens) => {
            if let Err(e) = s.store.write_google_tokens(&email, Some(&tokens)).await {
                tracing::error!(?e, "Failed to store Google tokens");
            }
            let mut health = match s.store.read_health().await {
                Ok(h) => h,
                Err(_) => HealthStatus::default(),
            };
            health.google_connected = true;
            let _ = s.store.write_health(&health).await;
            Html(user_layout("Google Connected", r#"<div class="card" style="max-width:600px;margin:2rem auto;text-align:center"><h1>✅ Google Calendar Connected</h1><p class="text-muted">Your Google Calendar and Tasks are now synced.</p><a href="/settings" class="btn mt-2">Back to Settings</a><a href="/preview" class="btn mt-1" style="margin-left:1rem">Generate Preview</a></div>"#)).into_response()
        }
        Err(e) => {
            tracing::error!(?e, "Failed to exchange Google OAuth code");
            Html(user_layout("Google OAuth Error", &format!("<div class=\"card\" style=\"max-width:600px;margin:2rem auto\"><h1>❌ Failed to Exchange Code</h1><p>{}</p><a href=\"/settings\" class=\"btn\">Back to Settings</a></div>", e))).into_response()
        }
    }
}

// ============ API ============

async fn api_settings_get(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let email = match require_email(&jar, &s).await {
        Some(e) => e,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error":"unauthorized"})),
            )
                .into_response()
        }
    };
    match s.store.read_settings(&email).await {
        Ok(settings) => Json(ApiResponse { data: settings }).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_settings_post(
    State(s): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<ReminderSettings>,
) -> impl IntoResponse {
    let email = match require_email(&jar, &s).await {
        Some(e) => e,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error":"unauthorized"})),
            )
                .into_response()
        }
    };
    let _ = s.store.ensure_user(&email).await;

    tracing::info!(session_email = %email, form_email = %payload.user_email, ai_provider = ?payload.ai_provider, "Saving user settings");

    match s.store.write_settings(&email, &payload).await {
        Ok(_) => {
            // Return the saved settings so UI can confirm
            let saved = s
                .store
                .read_settings(&email)
                .await
                .unwrap_or(payload.clone());
            tracing::info!(session_email = %email, saved_email = %saved.user_email, "Settings saved successfully");
            Json(ApiResponse { data: saved }).into_response()
        }
        Err(e) => {
            tracing::error!(session_email = %email, error = %e, "Failed to save settings");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

async fn api_generate_preview(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let email = match require_email(&jar, &s).await {
        Some(e) => e,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error":"unauthorized"})),
            )
                .into_response()
        }
    };
    let settings = match s.store.read_settings(&email).await {
        Ok(st) => st,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let tokens = s.store.read_google_tokens(&email).await.ok().flatten();
    match generate_preview_for_user(&s, &email, settings, tokens).await {
        Ok(preview) => {
            let cache = UserPreviewCache {
                subject: preview.subject.clone(),
                html: preview.html.clone(),
                text: preview.text.clone(),
                weather_advisory: preview.weather_advisory.clone(),
                image_url: preview.image_url.clone(),
                generated_language: preview.generated_language.clone(),
                generated_at: Utc::now(),
            };
            let _ = s.store.write_preview(&email, &cache).await;
            Json(ApiResponse { data: preview }).into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn api_send_email(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let email = match require_email(&jar, &s).await {
        Some(e) => e,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error":"unauthorized"})),
            )
                .into_response()
        }
    };

    // Only use cached preview — don't auto-generate (avoids Caddy timeout)
    let preview = match s.store.read_preview(&email).await {
        Ok(Some(cached)) => ReminderPreview {
            subject: cached.subject,
            html: cached.html,
            text: cached.text,
            weather_advisory: cached.weather_advisory,
            image_url: cached.image_url,
            generated_language: cached.generated_language,
        },
        _ => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"No cached preview found. Generate a preview first."})),
        )
            .into_response(),
    };

    // Load settings to get the user's email for delivery
    let settings = match s.store.read_settings(&email).await {
        Ok(st) => st,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    // Send via W9 Mail (matches w9-db payload format: from_alias + body_html)
    let mail_base = s.mail_api_base.trim_end_matches('/');
    let to_email = settings.user_email.clone();
    let payload = SendEmailPayload {
        to: to_email.clone(),
        from_alias: "reminder@w9.nu".to_string(),
        subject: preview.subject.clone(),
        body_html: preview.html.clone(),
    };

    tracing::info!(to = %to_email, from_alias = "reminder@w9.nu", "Sending email via W9 Mail");

    match s
        .mail_client
        .send_email(mail_base, W9_MAIL_TOKEN, &payload)
        .await
    {
        Ok(_) => {
            let _ = s.store.log_execution(&email, 0, true, None).await;
            Json(serde_json::json!({"status": "sent", "to": to_email})).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to send email via W9 Mail");
            let _ = s
                .store
                .log_execution(&email, 0, false, Some(&e.to_string()))
                .await;
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}

async fn api_health(State(s): State<AppState>) -> Json<ApiResponse<HealthStatus>> {
    let health = match s.store.read_health().await {
        Ok(h) => h,
        Err(_) => HealthStatus::default(),
    };
    Json(ApiResponse { data: health })
}

async fn api_image_models(State(s): State<AppState>) -> Json<ApiResponse<ImageModelOptions>> {
    let pollinations = s
        .pollinations
        .get_available_models()
        .await
        .unwrap_or_default();
    let cloudflare = CloudflareAiClient::supported_models();
    let cerebras = if s.cerebras.as_ref().is_some() {
        CerebrasClient::supported_models()
    } else {
        vec![]
    };
    Json(ApiResponse {
        data: ImageModelOptions {
            pollinations,
            cloudflare,
            cerebras,
        },
    })
}

// ============ Core: Generate Preview ============

#[derive(Debug)]
enum PreviewError {
    AiProvider(String),
    Weather(WeatherError),
    Google(GoogleError),
    Email(EmailBuildError),
    Serde(serde_json::Error),
}

impl std::fmt::Display for PreviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreviewError::AiProvider(e) => write!(f, "AI: {}", e),
            PreviewError::Weather(e) => write!(f, "Weather: {}", e),
            PreviewError::Google(e) => write!(f, "Google: {}", e),
            PreviewError::Email(e) => write!(f, "Email: {}", e),
            PreviewError::Serde(e) => write!(f, "JSON: {}", e),
        }
    }
}

impl From<WeatherError> for PreviewError {
    fn from(e: WeatherError) -> Self {
        PreviewError::Weather(e)
    }
}
impl From<GoogleError> for PreviewError {
    fn from(e: GoogleError) -> Self {
        PreviewError::Google(e)
    }
}
impl From<EmailBuildError> for PreviewError {
    fn from(e: EmailBuildError) -> Self {
        PreviewError::Email(e)
    }
}
impl From<serde_json::Error> for PreviewError {
    fn from(e: serde_json::Error) -> Self {
        PreviewError::Serde(e)
    }
}

fn sample_events() -> Vec<CalendarEvent> {
    let now = Utc::now();
    vec![
        CalendarEvent {
            id: Uuid::new_v4(),
            summary: "Standup".into(),
            start: now + Duration::hours(2),
            end: now + Duration::hours(3),
            location: Some("Meet / video".into()),
        },
        CalendarEvent {
            id: Uuid::new_v4(),
            summary: "Client sync".into(),
            start: now + Duration::hours(5),
            end: now + Duration::hours(6),
            location: Some("HQ".into()),
        },
    ]
}

async fn generate_preview_for_user(
    state: &AppState,
    email: &str,
    settings: ReminderSettings,
    google_tokens: Option<GoogleTokens>,
) -> Result<ReminderPreview, PreviewError> {
    let now = Utc::now();
    let tz: Tz = settings.timezone.parse().unwrap_or(chrono_tz::UTC);
    let local_now = now.with_timezone(&tz);
    let start_date = local_now.date_naive();
    let end_date = start_date + Duration::days(1);

    let start_dt_utc = start_date
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| tz.from_local_datetime(&dt).single())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(now);
    let end_dt_utc = end_date
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| tz.from_local_datetime(&dt).single())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(now + Duration::days(1));

    // Fetch calendar events
    let events = if let (Some(client), Some(tokens)) =
        (state.google.as_ref().as_ref(), google_tokens.as_ref())
    {
        let client = client.clone();
        let tokens = tokens.clone();
        match fetch_events(&client, tokens, start_dt_utc, end_dt_utc).await {
            Ok(ev) => {
                if ev.is_empty() {
                    sample_events()
                } else {
                    ev
                }
            }
            Err(e) => {
                tracing::warn!(?e, "Google events fetch failed, using sample");
                sample_events()
            }
        }
    } else {
        sample_events()
    };

    // Fetch todos
    let todos = if let (Some(client), Some(tokens)) =
        (state.google.as_ref().as_ref(), google_tokens.as_ref())
    {
        let client = client.clone();
        let tokens = tokens.clone();
        match client.list_todos(&tokens).await {
            Ok((t, _)) => t,
            Err(e) => {
                tracing::warn!(?e, "Google todos fetch failed");
                vec![]
            }
        }
    } else {
        vec![]
    };

    // Weather
    let weather_note = if settings.include_weather {
        match state
            .weather
            .day_forecast_4h(&settings.weather_location, start_dt_utc)
            .await
        {
            Ok(note) => Some(note),
            Err(_) => None,
        }
    } else {
        None
    };

    // AI generation
    let raw = match settings.ai_provider {
        AiProvider::Cerebras => {
            let client = state
                .cerebras
                .as_ref()
                .as_ref()
                .ok_or(PreviewError::AiProvider("Cerebras not configured".into()))?;
            let model = settings.cerebras_model.as_deref().unwrap_or("zai-glm-4.6");
            tracing::info!(
                model,
                provider = "cerebras",
                "Generating email via Cerebras"
            );
            client
                .generate_email(model, &settings, &events, &todos, weather_note.as_deref())
                .await
                .map_err(|e| PreviewError::AiProvider(e.to_string()))?
        }
        AiProvider::Nvidia => {
            let client = state
                .nvidia
                .as_ref()
                .as_ref()
                .ok_or(PreviewError::AiProvider("NVIDIA not configured".into()))?;
            let model_str = settings
                .nvidia_model
                .as_deref()
                .unwrap_or("minimaxai/minimax-m2.7");
            let model = NvidiaModel::from_str(model_str);
            let prompt = build_nvidia_prompt(&settings, &events, &todos, weather_note.as_deref());
            tracing::info!(
                model = model_str,
                provider = "nvidia",
                "Generating email via NVIDIA"
            );
            client.chat(model, "You are W9 Reminders AI. Output ONLY valid JSON. Follow the user's formatting instructions precisely.", &prompt).await
                .map_err(|e| PreviewError::AiProvider(e.to_string()))?
        }
    };

    // Image
    let mut image_url = None;
    if settings.include_image {
        if let Ok(prompt) = extract_image_prompt(&raw) {
            match settings.image_provider {
                ImageProvider::Pollinations => {
                    match state
                        .pollinations
                        .generate(&prompt, settings.image_model.as_deref())
                        .await
                    {
                        Ok(url) => image_url = Some(url),
                        Err(e) => tracing::warn!(?e, "Pollinations generation failed"),
                    }
                }
                ImageProvider::Cloudflare => {
                    if let Some(cf) = state.cloudflare.as_ref().as_ref() {
                        match cf
                            .generate(&prompt, settings.cloudflare_model.as_deref())
                            .await
                        {
                            Ok(url) => image_url = Some(url),
                            Err(e) => tracing::warn!(?e, "Cloudflare generation failed"),
                        }
                    }
                }
            }
        }
    }

    let preview = build_preview(&settings, &raw, weather_note, image_url)?;
    tracing::info!(email, "Generated preview for user");
    Ok(preview)
}

fn build_nvidia_prompt(
    settings: &ReminderSettings,
    events: &[CalendarEvent],
    todos: &[Todo],
    weather: Option<&str>,
) -> String {
    let mut prompt = String::new();
    prompt.push_str("Generate AI reminder email copy for W9 brand. JSON only.\n");
    prompt.push_str(&format!("Language: {}\n", settings.language));
    prompt.push_str(&format!("Summary style: {:?}\n", settings.summary_style));
    prompt.push_str("IMPORTANT: Output the JSON keys in this EXACT order: subject, preview, text_body, image_prompt, html_body.\n");
    prompt.push_str("The html_body field must contain ONLY the event and task content as HTML.\n");
    prompt.push_str("Use simple HTML tags like <p>, <ul>, <li>, <strong>, <em>, <br>. Do NOT include headers or structural layout.\n");
    prompt.push_str("Image prompt guidelines: describe a wide cinematic illustration or painted landscape that mirrors the emotional tone of the upcoming schedule. Use muted colors, natural light, film grain, and contemplative mood.\n");

    let tz: Tz = settings.timezone.parse().unwrap_or(chrono_tz::UTC);
    if !events.is_empty() {
        prompt.push_str("\nEvents (Local Time):\n");
        for event in events {
            let start = event.start.with_timezone(&tz);
            let end = event.end.with_timezone(&tz);
            prompt.push_str(&format!(
                "- {} from {} to {} at {}\n",
                event.summary,
                start.format("%H:%M"),
                end.format("%H:%M"),
                event.location.as_deref().unwrap_or("N/A")
            ));
        }
    }
    if !todos.is_empty() {
        prompt.push_str("\nTasks:\n");
        for todo in todos {
            if let Some(due) = todo.due {
                let due_local = due.with_timezone(&tz);
                prompt.push_str(&format!(
                    "- {} (due: {})\n",
                    todo.title,
                    due_local.format("%H:%M")
                ));
            } else {
                prompt.push_str(&format!("- {}\n", todo.title));
            }
            if let Some(notes) = &todo.notes {
                if !notes.trim().is_empty() {
                    prompt.push_str(&format!("  Note: {}\n", notes));
                }
            }
        }
    }
    if let Some(w) = weather {
        prompt.push_str(&format!(
            "\nWeather: {}. Do NOT include weather in html_body.\n",
            w
        ));
    }
    prompt.push_str("Return stringified JSON.");
    prompt
}

async fn fetch_events(
    client: &GoogleClient,
    tokens: GoogleTokens,
    time_min: chrono::DateTime<Utc>,
    time_max: chrono::DateTime<Utc>,
) -> Result<Vec<CalendarEvent>, GoogleError> {
    let (events, _refreshed) = client.list_events(&tokens, time_min, time_max).await?;
    Ok(events)
}

fn extract_image_prompt(raw: &str) -> Result<String, PreviewError> {
    #[derive(Deserialize)]
    struct Helper {
        image_prompt: Option<String>,
    }
    let helper: Helper = serde_json::from_str(raw)?;
    helper
        .image_prompt
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| PreviewError::AiProvider("image prompt missing".into()))
}

// ============ Main ============

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "w9_daily_reminders=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    dotenvy::dotenv().ok();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8084".into());
    let db_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("W9_REMINDERS_DB_URL"))
        .unwrap_or_else(|_| "postgres://w9_admin:password@w9-postgres:5432/w9_reminders".into());
    let mail_api_base =
        std::env::var("W9_MAIL_API_BASE").unwrap_or_else(|_| "https://mail.w9.nu/api".into());

    let store = DataStore::new(&db_url).await?;
    let mail_client = W9MailClient::new();

    let state = AppState {
        store,
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?,
        weather: Arc::new(WeatherClient::new()),
        cerebras: Arc::new(CerebrasClient::new().ok()),
        nvidia: Arc::new(NvidiaClient::new().ok()),
        pollinations: Arc::new(PollinationsClient::new().unwrap_or_else(|_| {
            tracing::warn!("Pollinations client init failed, using fallback");
            PollinationsClient::fallback()
        })),
        cloudflare: Arc::new(CloudflareAiClient::new().ok()),
        google: Arc::new(GoogleClient::new().ok()),
        mail_client: Arc::new(mail_client),
        mail_api_base,
    };

    let router = Router::new()
        .nest_service("/w9-logo", ServeDir::new("public/w9-logo"))
        .route("/", get(home))
        .route("/login", get(login_page))
        .route("/oauth/callback", get(oauth_cb))
        .route("/logout", get(logout))
        .route("/settings", get(settings_page))
        .route("/preview", get(preview_page))
        .route("/system", get(system_page))
        .route("/google/connect", get(google_connect))
        .route("/google/callback", get(google_callback))
        .route(
            "/api/settings",
            get(api_settings_get).post(api_settings_post),
        )
        .route("/api/reminders/preview", post(api_generate_preview))
        .route("/api/reminders/send", post(api_send_email))
        .route("/api/system/health", get(api_health))
        .route("/api/system/image-models", get(api_image_models))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("W9 Daily Reminders on {}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}
