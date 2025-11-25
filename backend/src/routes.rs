use std::sync::Arc;

use axum::{
  extract::State,
  http::{HeaderMap, StatusCode},
  Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  cerebras::{CerebrasClient, CerebrasError},
  cloudflare::CloudflareAiClient,
  email::{build_preview, EmailBuildError},
  google::{GoogleClient, GoogleError},
  models::{
    ApiResponse, CachedPreview, CalendarEvent, GoogleAuthPayload, GoogleAuthResult, GoogleAuthStartResponse, GoogleTokens, HealthStatus,
    ImageModelOptions, ImageProvider, MailSenderOption, ReminderPreview, ReminderSettings, SenderSelection, SystemConfig,
  },
  pollinations::{PollinationsClient, PollinationsError},
  store::DataStore,
  weather::{WeatherClient, WeatherError},
  w9mail::{SendEmailPayload, W9MailClient, W9MailError, W9MailProfile},
};

const STATIC_IMAGE_PROMPT: &str = "A series of cinematic film photographs and painted images with a nostalgic, contemplative mood. The collection includes a moody urban scene with a laptop by a window at night, minimalist blue sky with clouds over an industrial structure, a lone wooden hut on rolling green hills with dramatic shadows, a person lying face down in tall grass, a close-up fragment of a classical painting showing two hands reaching for each other, and a coastal train passing by a turquoise ocean. All images share a muted color palette, natural light, film grain, and a quiet, peaceful atmosphere.";

#[derive(Clone)]
pub struct AppState {
  store: DataStore,
  weather: Arc<WeatherClient>,
  cerebras: Arc<Option<CerebrasClient>>,
  pollinations: Arc<PollinationsClient>,
  cloudflare: Arc<Option<CloudflareAiClient>>,
  google: Arc<Option<GoogleClient>>,
  mail_client: Arc<W9MailClient>,
  mail_api_base: String,
  mail_service_token_env: Option<String>,
}

impl AppState {
  pub fn new(store: DataStore, mail_client: W9MailClient, mail_api_base: String, mail_service_token_env: Option<String>) -> Self {
    let weather = Arc::new(WeatherClient::new());
    let cerebras = Arc::new(CerebrasClient::new().ok());
    let pollinations = Arc::new(
      PollinationsClient::new().unwrap_or_else(|err| {
        tracing::warn!(?err, "Pollinations client initialization failed, will use fallback URL generation");
        PollinationsClient::fallback()
      })
    );
    let cloudflare = Arc::new(CloudflareAiClient::new().ok());
    let google = Arc::new(GoogleClient::new().ok());
    Self {
      store,
      weather,
      cerebras,
      pollinations,
      cloudflare,
      google,
      mail_client: Arc::new(mail_client),
      mail_api_base,
      mail_service_token_env,
    }
  }

  fn resolve_mail_base(&self, config: &SystemConfig) -> String {
    config
      .mail_api_base
      .as_ref()
      .and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
          None
        } else {
          Some(trimmed.to_string())
        }
      })
      .unwrap_or_else(|| self.mail_api_base.clone())
  }

  fn resolve_service_token(&self, config: &SystemConfig) -> Option<String> {
    config
      .mail_service_token
      .as_ref()
      .and_then(|token| {
        let trimmed = token.trim();
        if trimmed.is_empty() {
          None
        } else {
          Some(trimmed.to_string())
        }
      })
      .or_else(|| self.mail_service_token_env.clone())
  }
}

pub async fn settings_get(State(state): State<AppState>) -> Result<Json<ApiResponse<ReminderSettings>>, ApiError> {
  Ok(Json(ApiResponse { data: state.store.read_settings() }))
}

pub async fn settings_post(
  State(state): State<AppState>,
  Json(payload): Json<ReminderSettings>,
) -> Result<Json<ApiResponse<ReminderSettings>>, ApiError> {
  state.store.write_settings(&payload).await?;
  Ok(Json(ApiResponse { data: payload }))
}

pub async fn preview(
  State(state): State<AppState>,
  Json(payload): Json<ReminderSettings>,
) -> Result<Json<ApiResponse<ReminderPreview>>, ApiError> {
  let preview = generate_preview(&state, payload.clone()).await?;
  let snapshot = CachedPreview {
    preview: preview.clone(),
    settings: payload,
    generated_at: Utc::now(),
  };
  state.store.write_preview(&snapshot).await?;
  Ok(Json(ApiResponse { data: preview }))
}

pub async fn health(State(state): State<AppState>) -> Result<Json<ApiResponse<HealthStatus>>, ApiError> {
  Ok(Json(ApiResponse { data: state.store.read_health() }))
}

pub async fn google_start(State(state): State<AppState>) -> Result<Json<ApiResponse<GoogleAuthStartResponse>>, ApiError> {
  let google = state.google.as_ref().as_ref().ok_or(ApiError::Unavailable("Google OAuth not configured"))?;
  let url = google.auth_url(&Uuid::new_v4().to_string());
  Ok(Json(ApiResponse { data: GoogleAuthStartResponse { url } }))
}

pub async fn google_callback(
  State(state): State<AppState>,
  Json(payload): Json<GoogleAuthPayload>,
) -> Result<Json<ApiResponse<GoogleAuthResult>>, ApiError> {
  let google = state.google.as_ref().as_ref().ok_or(ApiError::Unavailable("Google OAuth not configured"))?;
  let tokens = google.exchange_code(&payload.code).await?;
  state.store.write_google_tokens(Some(tokens)).await?;
  let mut health = state.store.read_health();
  health.google_connected = true;
  state.store.write_health(&health).await?;
  Ok(Json(ApiResponse { data: GoogleAuthResult { connected: true } }))
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

pub async fn system_config_get(
  State(state): State<AppState>,
  headers: HeaderMap,
) -> Result<Json<ApiResponse<SystemConfigResponse>>, ApiError> {
  let token = extract_bearer(&headers)?;
  let config = state.store.read_config();
  let base = state.resolve_mail_base(&config);
  require_admin(&state, &config, &token).await?;
  let response = sanitize_config_response(&state, &config, &base);
  Ok(Json(ApiResponse { data: response }))
}

pub async fn system_config_update(
  State(state): State<AppState>,
  headers: HeaderMap,
  Json(payload): Json<UpdateSystemConfigRequest>,
) -> Result<Json<ApiResponse<SystemConfigResponse>>, ApiError> {
  let token = extract_bearer(&headers)?;
  let mut config = state.store.read_config();
  require_admin(&state, &config, &token).await?;

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

  state.store.write_config(&config).await?;
  let resolved_base = state.resolve_mail_base(&config);
  let response = sanitize_config_response(&state, &config, &resolved_base);
  Ok(Json(ApiResponse { data: response }))
}

pub async fn get_image_models(
  State(state): State<AppState>,
) -> Result<Json<ApiResponse<ImageModelOptions>>, ApiError> {
  let pollinations = state.pollinations.get_available_models().await?;
  let cloudflare = CloudflareAiClient::supported_models();
  Ok(Json(ApiResponse {
    data: ImageModelOptions { pollinations, cloudflare },
  }))
}

pub async fn list_mail_senders(
  State(state): State<AppState>,
  headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<MailSenderOption>>>, ApiError> {
  let token = extract_bearer(&headers)?;
  let config = state.store.read_config();
  let base = state.resolve_mail_base(&config);
  require_admin(&state, &config, &token).await?;
  let options = state.mail_client.list_senders(&base, &token).await?;
  Ok(Json(ApiResponse { data: options }))
}

pub async fn send_test_email(
  State(state): State<AppState>,
  headers: HeaderMap,
  Json(request): Json<SendTestRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
  let token = extract_bearer(&headers)?;
  let config = state.store.read_config();
  let base = state.resolve_mail_base(&config);
  require_admin(&state, &config, &token).await?;

  let settings = state.store.read_settings();
  let preview = match state.store.read_preview() {
    Some(cached) if cached.settings == settings => cached.preview,
    _ => {
      let fresh = generate_preview(&state, settings.clone()).await?;
      let snapshot = CachedPreview {
        preview: fresh.clone(),
        settings: settings.clone(),
        generated_at: Utc::now(),
      };
      state.store.write_preview(&snapshot).await?;
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
  let service_token = state
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

  state.mail_client.send_email(&base, &service_token, &payload).await?;

  Ok(Json(ApiResponse {
    data: serde_json::json!({ "status": "sent" }),
  }))
}

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

async fn fetch_google_events(state: &AppState, client: &GoogleClient, tokens: GoogleTokens) -> Result<Vec<CalendarEvent>, ApiError> {
  let (events, refreshed) = client.list_events(&tokens).await?;
  if let Some(new_tokens) = refreshed {
    state.store.write_google_tokens(Some(new_tokens)).await?;
  }
  if events.is_empty() {
    Ok(sample_events())
  } else {
    Ok(events)
  }
}

async fn generate_preview(state: &AppState, payload: ReminderSettings) -> Result<ReminderPreview, ApiError> {
  let events = match (state.google.as_ref().as_ref(), state.store.read_google_tokens()) {
    (Some(client), Some(tokens)) => match fetch_google_events(state, client, tokens).await {
      Ok(events) => events,
      Err(err) => {
        tracing::warn!(?err, "google events fallback to sample");
        sample_events()
      }
    },
    _ => sample_events(),
  };
  let weather_note = if payload.include_weather {
    Some(state.weather.advisory(&payload.weather_location).await?)
  } else {
    None
  };

  let cerebras = state.cerebras.as_ref().as_ref().ok_or(ApiError::Unavailable("Cerebras API key missing"))?;
  let raw = cerebras.generate_email(&payload, &events, weather_note.as_deref()).await?;

  let mut image_url = None;
  if payload.include_image {
    let prompt = STATIC_IMAGE_PROMPT;
    match payload.image_provider {
      ImageProvider::Pollinations => match state.pollinations.generate(prompt, payload.image_model.as_deref()).await {
        Ok(url) => image_url = Some(url),
        Err(err) => tracing::warn!(?err, "pollinations generation failed"),
      },
      ImageProvider::Cloudflare => {
        let client = state
          .cloudflare
          .as_ref()
          .as_ref()
          .ok_or(ApiError::Unavailable("Cloudflare Workers AI not configured"))?;
        match client.generate(prompt, payload.cloudflare_model.as_deref()).await {
          Ok(url) => image_url = Some(url),
          Err(err) => tracing::warn!(?err, "cloudflare image generation failed"),
        }
      }
    }
  }

  let preview = build_preview(&payload, &raw, weather_note.clone(), image_url)?;
  Ok(preview)
}

#[derive(Debug)]
pub enum ApiError {
  Store(crate::store::StoreError),
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

impl From<crate::store::StoreError> for ApiError {
  fn from(value: crate::store::StoreError) -> Self {
    ApiError::Store(value)
  }
}
impl From<WeatherError> for ApiError {
  fn from(value: WeatherError) -> Self {
    ApiError::Weather(value)
  }
}
impl From<CerebrasError> for ApiError {
  fn from(value: CerebrasError) -> Self {
    ApiError::Cerebras(value)
  }
}
impl From<GoogleError> for ApiError {
  fn from(value: GoogleError) -> Self {
    ApiError::Google(value)
  }
}
impl From<PollinationsError> for ApiError {
  fn from(value: PollinationsError) -> Self {
    ApiError::Pollinations(value)
  }
}
impl From<serde_json::Error> for ApiError {
  fn from(value: serde_json::Error) -> Self {
    ApiError::Serde(value)
  }
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
  fn from(value: EmailBuildError) -> Self {
    ApiError::Email(value)
  }
}

impl axum::response::IntoResponse for ApiError {
  fn into_response(self) -> axum::response::Response {
    tracing::error!(?self, "api error");
    let status = match self {
      ApiError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
      ApiError::Store(_) | ApiError::Serde(_) => StatusCode::INTERNAL_SERVER_ERROR,
      ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
      ApiError::Weather(_) | ApiError::Cerebras(_) | ApiError::Pollinations(_) | ApiError::Google(_) | ApiError::W9Mail(_) | ApiError::Email(_) => {
        StatusCode::BAD_GATEWAY
      }
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
