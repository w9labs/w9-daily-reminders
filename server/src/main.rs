mod models;
mod store;
mod weather;
mod cerebras;
mod pollinations;
mod cloudflare;
mod email;
mod google;
mod w9mail;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::cerebras::{CerebrasClient, CerebrasError};
use crate::cloudflare::CloudflareAiClient;
use crate::email::{build_preview, EmailBuildError};
use crate::google::{GoogleClient, GoogleError};
use crate::models::{
    ApiResponse, CachedPreview, CalendarEvent, GoogleAuthPayload, GoogleAuthResult, GoogleAuthStartResponse, GoogleTokens,
    HealthStatus, ImageModelOptions, ImageProvider, MailSenderOption, ReminderPreview, ReminderSettings, ScheduleType,
    SenderSelection, SystemConfig, Todo, WeekStartDay,
};
use crate::pollinations::{PollinationsClient, PollinationsError};
use crate::store::DataStore;
use crate::weather::{WeatherClient, WeatherError};
use crate::w9mail::{SendEmailPayload, W9MailClient, W9MailError, W9MailProfile};

const CSS: &str = include_str!("../infra/templates/voxel.css");
const W9_DB: &str = "https://db.w9.nu";

#[derive(Clone)]
pub struct AppState {
    pub store: DataStore,
    pub db_url: String,
    pub http_client: reqwest::Client,
    pub weather: Arc<WeatherClient>,
    pub cerebras: Arc<Option<CerebrasClient>>,
    pub pollinations: Arc<PollinationsClient>,
    pub cloudflare: Arc<Option<CloudflareAiClient>>,
    pub google: Arc<Option<GoogleClient>>,
    pub mail_client: Arc<W9MailClient>,
    pub mail_api_base: String,
    pub mail_service_token_env: Option<String>,
}

impl AppState {
    fn resolve_mail_base(&self, config: &SystemConfig) -> String {
        config
            .mail_api_base
            .as_ref()
            .and_then(|v| {
                let trimmed = v.trim();
                if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
            })
            .unwrap_or_else(|| self.mail_api_base.clone())
    }

    fn resolve_service_token(&self, config: &SystemConfig) -> Option<String> {
        config
            .mail_service_token
            .as_ref()
            .and_then(|token| {
                let trimmed = token.trim();
                if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
            })
            .or_else(|| self.mail_service_token_env.clone())
    }
}

// ==================== Layout Functions ====================

fn layout(t: &str, b: &str, n: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"/><link rel="icon" type="image/svg+xml" href="/w9-logo/favicon.svg"/><meta name="viewport" content="width=device-width,initial-scale=1.0"/><title>{} — W9 Reminders</title><style>{}</style></head><body><div class="app"><nav class="nav"><div class="nav-inner"><a href="/" class="brand"><img src="/w9-logo/workmark-transparent.svg" alt="W9 Labs"/><span class="brand-text">Reminders</span></a><div class="nav-links">{}</div></div></nav><main class="app-main">{}</main><footer class="footer"><img class="footer-logo" src="/w9-logo/workmark-transparent.svg" alt="W9 Labs"/><p>W9 Daily Reminders — AI Calendar Digest</p><p class="text-xs text-muted">Google Calendar + AI + Email</p><a href="https://pollinations.ai" target="_blank" rel="noopener"><img src="https://img.shields.io/badge/Built%20with-Pollinations-8a2be2?style=for-the-badge&logo=data:image/png;base64,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&logoColor=white&labelColor=6a0dad" alt="Built with Pollinations" style="margin-top:12px;"/></a></footer></div></body></html>"#,
        t, CSS, n, b
    )
}

fn pub_layout(t: &str, b: &str) -> String {
    layout(t, b, r#"<a href="/login">Login</a><a href="/settings">Settings</a>"#)
}

fn user_layout(t: &str, b: &str) -> String {
    layout(t, b, r#"<a href="/settings">Settings</a><a href="/preview">Preview</a><a href="/system">System</a><a href="/logout">Logout</a>"#)
}

// ==================== Session Helpers ====================

fn set_s(j: CookieJar, t: String) -> CookieJar {
    let cookie = axum_extra::extract::cookie::Cookie::build(("w9_rem_session", t))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::days(7))
        .build();
    j.add(cookie)
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

async fn require(j: &CookieJar, a: &AppState) -> Option<serde_json::Value> {
    let t = get_s(j)?;
    verify(a, &t).await
}

// ==================== HTML Pages ====================

fn home_html() -> String {
    pub_layout(
        "W9 Reminders",
        r#"<div class="hero"><img class="hero-logo" src="/w9-logo/logo-landscape-transparent.svg" alt="W9 Labs"/><h1>W9 Daily Reminders</h1><p class="hero-sub">AI-powered daily email digests from your Google Calendar</p><p class="hero-muted">Never miss a meeting again</p><div class="hero-actions"><a href="/login" class="btn">Login with W9</a></div></div><div class="grid"><div class="card"><h3>📅 Google Calendar</h3><p>Connect your Google Calendar for daily event summaries.</p></div><div class="card"><h3>🤖 AI Summaries</h3><p>AI generates personalized daily summaries with images.</p></div><div class="card"><h3>📧 Email Delivery</h3><p>Beautiful HTML emails delivered via W9 Mail every morning.</p></div><div class="card"><h3>🌤️ Weather</h3><p>Location-based weather advisories with practical recommendations.</p></div><div class="card"><h3>🎨 AI Images</h3><p>Dynamic visuals from Pollinations.ai or Cloudflare Workers AI.</p></div><div class="card"><h3>✅ Google Tasks</h3><p>Your tasks and events together in one daily briefing.</p></div></div>"#,
    )
}

fn login_html() -> String {
    pub_layout(
        "Login",
        r#"<div class="card" style="max-width:420px;margin:3rem auto;text-align:center"><h1>⏰ W9 Reminders</h1><p class="text-sm text-muted mb-2">Sign in with W9 DB</p><a href="https://db.w9.nu/oauth/authorize?redirect_uri=https://reminder.w9.nu/oauth/callback&response_type=code&client_id=w9-reminders" class="btn" style="width:100%">Login with W9 DB</a></div>"#,
    )
}

fn settings_html(settings: &ReminderSettings, msg: Option<&str>) -> String {
    let al = msg.map(|x| format!(r#"<div class="alert alert--ok">{}</div>"#, x)).unwrap_or_default();
    user_layout(
        "Settings",
        &format!(
            r#"<div class="card" style="max-width:700px;margin:2rem auto"><h1>⚙️ Reminder Settings</h1>{}<form id="settings-form"><label>Email</label><input type="email" id="user_email" value="{}" required placeholder="you@w9.nu"/><label>Reminder Time</label><input type="time" id="reminder_time" value="{}" required/><label>Timezone</label><input type="text" id="timezone" value="{}" placeholder="Europe/Stockholm"/><label>Language</label><input type="text" id="language" value="{}" placeholder="English"/><label>Weather Location</label><input type="text" id="weather_location" value="{}" placeholder="Stockholm, Sweden"/><label><input type="checkbox" id="include_weather" {} /> Include Weather</label><label><input type="checkbox" id="include_image" {} /> Include AI Image</label><label>Image Provider</label><select id="image_provider"><option value="pollinations" {}>Pollinations</option><option value="cloudflare" {}>Cloudflare</option></select><label>Summary Style</label><select id="summary_style"><option value="concise" {}>Concise</option><option value="detailed" {}>Detailed</option><option value="bullet" {}>Bullet</option></select><label>Schedule Type</label><select id="schedule_type"><option value="day" {}>Day</option><option value="week" {}>Week</option></select><button type="submit" class="btn mt-1" style="width:100%">Save Settings</button></form><div class="mt-3"><a href="/google/connect" class="btn">🔗 Connect Google Calendar</a></div></div>"#,
            al,
            settings.user_email,
            settings.reminder_time,
            settings.timezone,
            settings.language,
            settings.weather_location,
            if settings.include_weather { "checked" } else { "" },
            if settings.include_image { "checked" } else { "" },
            if settings.image_provider == ImageProvider::Pollinations { "selected" } else { "" },
            if settings.image_provider == ImageProvider::Cloudflare { "selected" } else { "" },
            if settings.summary_style == models::SummaryStyle::Concise { "selected" } else { "" },
            if settings.summary_style == models::SummaryStyle::Detailed { "selected" } else { "" },
            if settings.summary_style == models::SummaryStyle::Bullet { "selected" } else { "" },
            if settings.schedule_type == ScheduleType::Day { "selected" } else { "" },
            if settings.schedule_type == ScheduleType::Week { "selected" } else { "" },
        ),
    )
}

fn preview_html(preview: Option<&ReminderPreview>) -> String {
    let content = match preview {
        Some(p) => format!(
            r#"<div class="card"><h2>Subject: {}</h2><p><strong>Preview:</strong> {}</p><hr/><div style="background:#fff;color:#000;padding:1rem;border:1px solid #333;">{}</div><hr/><p><strong>Weather:</strong> {}</p><p><strong>Image:</strong> {}</p></div>"#,
            p.subject,
            p.generated_language,
            &p.html,
            p.weather_advisory.as_deref().unwrap_or("N/A"),
            p.image_url.as_deref().unwrap_or("No image"),
        ),
        None => r#"<div class="card"><h2>No preview generated yet</h2><p>Click "Generate Preview" to create your daily reminder.</p></div>"#.to_string(),
    };
    user_layout("Preview", &format!(r#"<div style="max-width:800px;margin:2rem auto"><h1>📧 Email Preview</h1><form id="preview-form"><button type="submit" class="btn">Generate Preview</button></form>{}</div>"#, content))
}

fn system_html(health: &HealthStatus) -> String {
    user_layout(
        "System Status",
        &format!(
            r#"<div class="card" style="max-width:600px;margin:2rem auto"><h1>🖥️ System Status</h1><table><tr><th>Metric</th><th>Value</th></tr><tr><td>Scheduler State</td><td>{:?}</td></tr><tr><td>Last Dispatch</td><td>{}</td></tr><tr><td>Next Run</td><td>{}</td></tr><tr><td>Google Connected</td><td>{}</td></tr></table></div>"#,
            health.scheduler,
            health.last_dispatch.map(|t| t.to_rfc3339()).unwrap_or_else(|| "Never".into()),
            health.next_run.map(|t| t.to_rfc3339()).unwrap_or_else(|| "Not scheduled".into()),
            if health.google_connected { "✅ Yes" } else { "❌ No" },
        ),
    )
}

// ==================== Route Handlers ====================

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
    (set_s(jar, token), Redirect::to("/settings")).into_response()
}

async fn logout(jar: CookieJar) -> impl IntoResponse {
    (clr_s(jar), Redirect::to("/")).into_response()
}

async fn settings_page(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    if require(&jar, &s).await.is_none() {
        return Redirect::to("/login").into_response();
    }
    let settings = s.store.read_settings();
    Html(settings_html(&settings, None)).into_response()
}

async fn preview_page(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    if require(&jar, &s).await.is_none() {
        return Redirect::to("/login").into_response();
    }
    let cached = s.store.read_preview();
    let preview_ref = cached.as_ref().map(|c| &c.preview);
    Html(preview_html(preview_ref)).into_response()
}

async fn system_page(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    if require(&jar, &s).await.is_none() {
        return Redirect::to("/login").into_response();
    }
    let health = s.store.read_health();
    Html(system_html(&health)).into_response()
}

async fn google_connect(State(s): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    if require(&jar, &s).await.is_none() {
        return Redirect::to("/login").into_response();
    }
    let google = match s.google.as_ref().as_ref() {
        Some(g) => g,
        None => return Html(user_layout("Error", "<div class=\"card\"><h1>Google OAuth not configured</h1></div>")).into_response(),
    };
    let url = google.auth_url(&Uuid::new_v4().to_string());
    (Redirect::to(&url)).into_response()
}

// ==================== API Handlers ====================

async fn api_settings_get(State(s): State<AppState>) -> Json<ApiResponse<ReminderSettings>> {
    Json(ApiResponse { data: s.store.read_settings() })
}

async fn api_settings_post(State(s): State<AppState>, Json(payload): Json<ReminderSettings>) -> Json<ApiResponse<ReminderSettings>> {
    let _ = s.store.write_settings(&payload).await;
    Json(ApiResponse { data: payload })
}

async fn api_preview(
    State(s): State<AppState>,
    Json(payload): Json<ReminderSettings>,
) -> Result<Json<ApiResponse<ReminderPreview>>, ApiError> {
    let preview = generate_preview(&s, payload.clone()).await?;
    let snapshot = CachedPreview {
        preview: preview.clone(),
        settings: payload,
        generated_at: Utc::now(),
    };
    s.store.write_preview(&snapshot).await?;
    Ok(Json(ApiResponse { data: preview }))
}

async fn api_health(State(s): State<AppState>) -> Json<ApiResponse<HealthStatus>> {
    Json(ApiResponse { data: s.store.read_health() })
}

async fn api_google_start(State(s): State<AppState>) -> Result<Json<ApiResponse<GoogleAuthStartResponse>>, ApiError> {
    let google = s.google.as_ref().as_ref().ok_or(ApiError::Unavailable("Google OAuth not configured"))?;
    let url = google.auth_url(&Uuid::new_v4().to_string());
    Ok(Json(ApiResponse {
        data: GoogleAuthStartResponse { url },
    }))
}

async fn api_google_callback(
    State(s): State<AppState>,
    Json(payload): Json<GoogleAuthPayload>,
) -> Result<Json<ApiResponse<GoogleAuthResult>>, ApiError> {
    let google = s.google.as_ref().as_ref().ok_or(ApiError::Unavailable("Google OAuth not configured"))?;
    let tokens = google.exchange_code(&payload.code).await?;
    s.store.write_google_tokens(Some(tokens)).await?;
    let mut health = s.store.read_health();
    health.google_connected = true;
    s.store.write_health(&health).await?;
    Ok(Json(ApiResponse {
        data: GoogleAuthResult { connected: true },
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfigResponse {
    pub mail_api_base: String,
    pub daily_sender: Option<SenderSelection>,
    pub noreply_sender: Option<SenderSelection>,
    pub service_token_present: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSystemConfigRequest {
    pub mail_api_base: Option<String>,
    pub mail_service_token: Option<String>,
    pub daily_sender: Option<SenderSelection>,
    pub noreply_sender: Option<SenderSelection>,
}

#[derive(Deserialize)]
pub struct SendTestRequest {
    pub recipient: Option<String>,
}

async fn api_system_config_get(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<SystemConfigResponse>>, ApiError> {
    let token = extract_bearer(&headers)?;
    let config = s.store.read_config();
    let base = s.resolve_mail_base(&config);
    require_admin(&s, &config, &token).await?;
    let response = sanitize_config_response(&s, &config, &base);
    Ok(Json(ApiResponse { data: response }))
}

async fn api_system_config_update(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateSystemConfigRequest>,
) -> Result<Json<ApiResponse<SystemConfigResponse>>, ApiError> {
    let token = extract_bearer(&headers)?;
    let mut config = s.store.read_config();
    require_admin(&s, &config, &token).await?;

    if let Some(new_base_raw) = payload.mail_api_base {
        let trimmed = new_base_raw.trim();
        if trimmed.is_empty() {
            config.mail_api_base = None;
        } else {
            config.mail_api_base = Some(trimmed.to_string());
        }
    }

    if let Some(token_value_raw) = payload.mail_service_token {
        let trimmed = token_value_raw.trim();
        if trimmed.is_empty() {
            config.mail_service_token = None;
        } else {
            config.mail_service_token = Some(trimmed.to_string());
        }
    }

    if let Some(sender) = payload.daily_sender {
        config.daily_sender = Some(sender);
    }

    if let Some(sender) = payload.noreply_sender {
        config.noreply_sender = Some(sender);
    }

    s.store.write_config(&config).await?;
    let resolved_base = s.resolve_mail_base(&config);
    let response = sanitize_config_response(&s, &config, &resolved_base);
    Ok(Json(ApiResponse { data: response }))
}

async fn api_get_image_models(State(s): State<AppState>) -> Result<Json<ApiResponse<ImageModelOptions>>, ApiError> {
    let pollinations = s.pollinations.get_available_models().await?;
    let cloudflare = CloudflareAiClient::supported_models();
    let cerebras = if s.cerebras.is_some() {
        cerebras::CerebrasClient::supported_models()
    } else {
        vec![]
    };
    Ok(Json(ApiResponse {
        data: ImageModelOptions {
            pollinations,
            cloudflare,
            cerebras,
        },
    }))
}

async fn api_list_mail_senders(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<MailSenderOption>>>, ApiError> {
    let token = extract_bearer(&headers)?;
    let config = s.store.read_config();
    let base = s.resolve_mail_base(&config);
    require_admin(&s, &config, &token).await?;
    let options = s.mail_client.list_senders(&base, &token).await?;
    Ok(Json(ApiResponse { data: options }))
}

async fn api_send_test_email(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SendTestRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let token = extract_bearer(&headers)?;
    let config = s.store.read_config();
    let base = s.resolve_mail_base(&config);
    require_admin(&s, &config, &token).await?;

    let settings = s.store.read_settings();
    let preview = match s.store.read_preview() {
        Some(cached) if cached.settings == settings => cached.preview,
        _ => {
            let fresh = generate_preview(&s, settings.clone()).await?;
            let snapshot = CachedPreview {
                preview: fresh.clone(),
                settings: settings.clone(),
                generated_at: Utc::now(),
            };
            s.store.write_preview(&snapshot).await?;
            fresh
        }
    };

    let recipient = request
        .recipient
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| settings.user_email.clone());

    if recipient.trim().is_empty() {
        return Err(ApiError::Unavailable("recipient email missing"));
    }

    let sender = config
        .daily_sender
        .clone()
        .ok_or(ApiError::Unavailable("daily sender not configured"))?;
    let service_token = s
        .resolve_service_token(&config)
        .ok_or(ApiError::Unavailable("mail service token not configured"))?;

    let payload = SendEmailPayload {
        from: sender.address.clone(),
        to: recipient,
        cc: None,
        bcc: None,
        subject: preview.subject.clone(),
        body: preview.html.clone(),
        is_html: true,
    };

    s.mail_client.send_email(&base, &service_token, &payload).await?;

    Ok(Json(ApiResponse {
        data: serde_json::json!({ "status": "sent" }),
    }))
}

// ==================== Core Logic ====================

fn sample_events() -> Vec<CalendarEvent> {
    let now = Utc::now();
    vec![
        CalendarEvent {
            id: uuid::Uuid::new_v4(),
            summary: "Standup".into(),
            start: now + Duration::hours(2),
            end: now + Duration::hours(3),
            location: Some("Meet / video".into()),
        },
        CalendarEvent {
            id: uuid::Uuid::new_v4(),
            summary: "Client sync".into(),
            start: now + Duration::hours(5),
            end: now + Duration::hours(6),
            location: Some("HQ".into()),
        },
    ]
}

async fn fetch_google_events(
    state: &AppState,
    client: &GoogleClient,
    tokens: GoogleTokens,
    time_min: chrono::DateTime<chrono::Utc>,
    time_max: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<CalendarEvent>, ApiError> {
    let (events, refreshed) = client.list_events(&tokens, time_min, time_max).await?;
    if let Some(new_tokens) = refreshed {
        state.store.write_google_tokens(Some(new_tokens)).await?;
    }
    if events.is_empty() {
        Ok(sample_events())
    } else {
        Ok(events)
    }
}

async fn fetch_google_todos(state: &AppState, client: &GoogleClient, tokens: GoogleTokens) -> Result<Vec<Todo>, ApiError> {
    match client.list_todos(&tokens).await {
        Ok((todos, refreshed)) => {
            if let Some(new_tokens) = refreshed {
                state.store.write_google_tokens(Some(new_tokens)).await?;
            }
            Ok(todos)
        }
        Err(err) => {
            tracing::warn!(?err, "google todos fetch failed, returning empty list");
            Ok(vec![])
        }
    }
}

async fn generate_preview(state: &AppState, payload: ReminderSettings) -> Result<ReminderPreview, ApiError> {
    use chrono::{Datelike, TimeZone, Weekday};

    let now = chrono::Utc::now();
    let tz: chrono_tz::Tz = payload.timezone.parse().unwrap_or(chrono_tz::UTC);
    let local_now = now.with_timezone(&tz);

    let (start_date, end_date) = match payload.schedule_type {
        ScheduleType::Day => {
            let target_date = local_now.date_naive();
            (target_date, target_date + Duration::days(1))
        }
        ScheduleType::Week => {
            let week_start_offset = match payload.week_start_day {
                WeekStartDay::Monday => {
                    let weekday = local_now.weekday();
                    -(weekday.num_days_from_monday() as i64)
                }
                WeekStartDay::Sunday => {
                    let weekday = local_now.weekday();
                    -(match weekday {
                        Weekday::Sun => 0,
                        Weekday::Mon => 1,
                        Weekday::Tue => 2,
                        Weekday::Wed => 3,
                        Weekday::Thu => 4,
                        Weekday::Fri => 5,
                        Weekday::Sat => 6,
                    })
                }
            };
            let week_start = local_now.date_naive() + Duration::days(week_start_offset);
            (week_start, week_start + Duration::days(7))
        }
    };

    let start_dt_utc = start_date
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| tz.from_local_datetime(&dt).single())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or(now);
    let end_dt_utc = end_date
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| tz.from_local_datetime(&dt).single())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or(now + Duration::days(1));

    let all_events = match (state.google.as_ref().as_ref(), state.store.read_google_tokens()) {
        (Some(client), Some(tokens)) => match fetch_google_events(state, client, tokens.clone(), start_dt_utc, end_dt_utc).await {
            Ok(events) => events,
            Err(_) => sample_events(),
        },
        _ => sample_events(),
    };

    let events: Vec<CalendarEvent> = all_events
        .into_iter()
        .filter(|event| {
            let event_date = event.start.with_timezone(&tz).date_naive();
            event_date >= start_date && event_date < end_date
        })
        .collect();

    let all_todos = match (state.google.as_ref().as_ref(), state.store.read_google_tokens()) {
        (Some(client), Some(tokens)) => fetch_google_todos(state, client, tokens).await.unwrap_or_default(),
        _ => vec![],
    };

    let todos: Vec<Todo> = all_todos
        .into_iter()
        .filter(|todo| {
            if let Some(due) = todo.due {
                let todo_date = due.with_timezone(&tz).date_naive();
                todo_date >= start_date && todo_date < end_date
            } else {
                matches!(payload.schedule_type, ScheduleType::Day)
            }
        })
        .collect();

    let weather_note = if payload.include_weather {
        match payload.schedule_type {
            ScheduleType::Day => {
                let target_date = start_date
                    .and_hms_opt(0, 0, 0)
                    .and_then(|dt| tz.from_local_datetime(&dt).single())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or(now);
                match state.weather.day_forecast_4h(&payload.weather_location, target_date).await {
                    Ok(note) => Some(note),
                    Err(_) => None,
                }
            }
            ScheduleType::Week => {
                let week_start_dt = start_date
                    .and_hms_opt(0, 0, 0)
                    .and_then(|dt| tz.from_local_datetime(&dt).single())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or(now);
                match state.weather.week_forecast(&payload.weather_location, week_start_dt).await {
                    Ok(note) => Some(note),
                    Err(_) => None,
                }
            }
        }
    } else {
        None
    };

    let cerebras = state.cerebras.as_ref().as_ref().ok_or(ApiError::Unavailable("Cerebras API key missing"))?;
    let model = payload.cerebras_model.as_deref().unwrap_or("zai-glm-4.6");
    tracing::info!(model, "using Cerebras model for email generation");
    let raw = cerebras
        .generate_email(model, &payload, &events, &todos, weather_note.as_deref())
        .await?;

    let mut image_url = None;
    if payload.include_image {
        if let Ok(prompt) = extract_image_prompt(&raw) {
            match payload.image_provider {
                ImageProvider::Pollinations => {
                    let prepared = prepare_image_prompt(&prompt, ImageProvider::Pollinations, payload.image_model.as_deref());
                    match state.pollinations.generate(&prepared, payload.image_model.as_deref()).await {
                        Ok(url) => image_url = Some(url),
                        Err(err) => tracing::warn!(?err, "pollinations generation failed"),
                    }
                }
                ImageProvider::Cloudflare => {
                    let client = state
                        .cloudflare
                        .as_ref()
                        .as_ref()
                        .ok_or(ApiError::Unavailable("Cloudflare Workers AI not configured"))?;
                    let prepared = prepare_image_prompt(&prompt, ImageProvider::Cloudflare, payload.cloudflare_model.as_deref());
                    match client.generate(&prepared, payload.cloudflare_model.as_deref()).await {
                        Ok(url) => image_url = Some(url),
                        Err(err) => tracing::warn!(?err, "cloudflare image generation failed"),
                    }
                }
            }
        }
    }

    let preview = build_preview(&payload, &raw, weather_note.clone(), image_url)?;
    Ok(preview)
}

fn extract_image_prompt(raw: &str) -> Result<String, ApiError> {
    #[derive(Deserialize)]
    struct Helper {
        image_prompt: Option<String>,
    }
    let helper: Helper = serde_json::from_str(raw)?;
    helper
        .image_prompt
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| ApiError::Unavailable("image prompt missing from Cerebras payload"))
}

fn prepare_image_prompt(prompt: &str, provider: ImageProvider, model: Option<&str>) -> String {
    let cleaned = prompt
        .trim()
        .replace('"', "")
        .replace("“", "")
        .replace("”", "")
        .replace("—", "-")
        .replace("–", "-");

    if matches!(provider, ImageProvider::Cloudflare) && model.map(|m| m.contains("flux-2-dev")).unwrap_or(true) {
        format!(
            "Cinematic atmospheric environmental concept art inspired by: {}. Focus on architecture, weather, light, and texture only. No people, faces, silhouettes, public figures, logos, trademarks, text overlays, or recognizable locations. Anonymous scenery, abstract shapes, subdued palette.",
            cleaned
        )
    } else {
        cleaned
    }
}

// ==================== Error Handling ====================

#[derive(Debug)]
pub enum ApiError {
    Store(store::StoreError),
    Weather(WeatherError),
    Cerebras(CerebrasError),
    Google(GoogleError),
    Pollinations(PollinationsError),
    Email(EmailBuildError),
    W9Mail(W9MailError),
    Serde(serde_json::Error),
    Unavailable(&'static str),
    Unauthorized(&'static str),
}

impl From<store::StoreError> for ApiError {
    fn from(value: store::StoreError) -> Self { ApiError::Store(value) }
}
impl From<WeatherError> for ApiError {
    fn from(value: WeatherError) -> Self { ApiError::Weather(value) }
}
impl From<CerebrasError> for ApiError {
    fn from(value: CerebrasError) -> Self { ApiError::Cerebras(value) }
}
impl From<GoogleError> for ApiError {
    fn from(value: GoogleError) -> Self { ApiError::Google(value) }
}
impl From<PollinationsError> for ApiError {
    fn from(value: PollinationsError) -> Self { ApiError::Pollinations(value) }
}
impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self { ApiError::Serde(value) }
}
impl From<W9MailError> for ApiError {
    fn from(value: W9MailError) -> Self {
        match value {
            W9MailError::Unauthorized => ApiError::Unauthorized("invalid or expired token"),
            other => ApiError::W9Mail(other),
        }
    }
}
impl From<EmailBuildError> for ApiError {
    fn from(value: EmailBuildError) -> Self { ApiError::Email(value) }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(?self, "api error");
        let status = match self {
            ApiError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Store(_) | ApiError::Serde(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            _ => StatusCode::BAD_GATEWAY,
        };
        let message = match self {
            ApiError::Unavailable(reason) => reason.to_string(),
            ApiError::Store(err) => err.to_string(),
            ApiError::Weather(err) => err.to_string(),
            ApiError::Cerebras(err) => err.to_string(),
            ApiError::Google(err) => err.to_string(),
            ApiError::Pollinations(err) => err.to_string(),
            ApiError::Serde(err) => err.to_string(),
            ApiError::W9Mail(err) => err.to_string(),
            ApiError::Email(err) => err.to_string(),
            ApiError::Unauthorized(reason) => reason.to_string(),
        };
        let payload = serde_json::json!({ "error": message });
        (status, Json(payload)).into_response()
    }
}

fn extract_bearer(headers: &HeaderMap) -> Result<String, ApiError> {
    let header_value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Unauthorized("missing authorization header"))?;
    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized("invalid authorization header"))?;
    if token.trim().is_empty() {
        return Err(ApiError::Unauthorized("authorization header is empty"));
    }
    Ok(token.to_string())
}

async fn require_admin(state: &AppState, config: &SystemConfig, token: &str) -> Result<W9MailProfile, ApiError> {
    let base = state.resolve_mail_base(config);
    let profile = state.mail_client.profile(&base, token).await?;
    if profile.role.eq_ignore_ascii_case("admin") {
        Ok(profile)
    } else {
        Err(ApiError::Unauthorized("admin privileges required"))
    }
}

fn sanitize_config_response(state: &AppState, config: &SystemConfig, base: &str) -> SystemConfigResponse {
    let service_token_present = config
        .mail_service_token
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or_else(|| state.mail_service_token_env.is_some());
    SystemConfigResponse {
        mail_api_base: base.to_string(),
        daily_sender: config.daily_sender.clone(),
        noreply_sender: config.noreply_sender.clone(),
        service_token_present,
    }
}

// ==================== Main ====================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();
    dotenvy::dotenv().ok();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8084".into());
    let db_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("W9_REMINDERS_DB_URL"))
        .unwrap_or_else(|_| "postgres://w9_admin:password@w9-postgres:5432/w9_reminders".into());

    let mail_api_base = std::env::var("W9_MAIL_API_BASE").unwrap_or_else(|_| "https://mail.w9.nu/api".into());
    let mail_service_token_env = std::env::var("W9_MAIL_SERVICE_TOKEN").ok().filter(|v| !v.trim().is_empty());

    let store = DataStore::new(&db_url).await?;
    let mail_client = W9MailClient::new();

    let state = AppState {
        store,
        db_url,
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?,
        weather: Arc::new(WeatherClient::new()),
        cerebras: Arc::new(CerebrasClient::new().ok()),
        pollinations: Arc::new(
            PollinationsClient::new().unwrap_or_else(|err| {
                tracing::warn!(?err, "Pollinations client initialization failed, using fallback");
                PollinationsClient::fallback()
            }),
        ),
        cloudflare: Arc::new(CloudflareAiClient::new().ok()),
        google: Arc::new(GoogleClient::new().ok()),
        mail_client: Arc::new(mail_client),
        mail_api_base,
        mail_service_token_env,
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
        .route("/api/settings", get(api_settings_get).post(api_settings_post))
        .route("/api/reminders/preview", post(api_preview))
        .route("/api/system/health", get(api_health))
        .route("/api/google/start", post(api_google_start))
        .route("/api/google/callback", post(api_google_callback))
        .route("/api/system/config", get(api_system_config_get).post(api_system_config_update))
        .route("/api/system/senders", get(api_list_mail_senders))
        .route("/api/system/image-models", get(api_get_image_models))
        .route("/api/reminders/send-test", post(api_send_test_email))
        .with_state(state)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(CorsLayer::permissive()));

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("W9 Daily Reminders listening on {}", addr);
    axum::serve(listener, router).await?;

    Ok(())
}
