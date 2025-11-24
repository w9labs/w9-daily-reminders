use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderSettings {
  pub user_email: String,
  pub reminder_time: String,
  pub timezone: String,
  pub language: String,
  #[serde(default)]
  pub custom_language: Option<String>,
  pub weather_location: String,
  pub include_weather: bool,
  pub include_image: bool,
  #[serde(default = "default_summary_style")]
  pub summary_style: SummaryStyle,
}

fn default_summary_style() -> SummaryStyle {
  SummaryStyle::Concise
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SummaryStyle {
  Concise,
  Detailed,
  Bullet,
}

impl Default for ReminderSettings {
  fn default() -> Self {
    Self {
      user_email: "".into(),
      reminder_time: "07:30".into(),
      timezone: "Europe/Stockholm".into(),
      language: "English".into(),
      custom_language: None,
      weather_location: "Stockholm, Sweden".into(),
      include_weather: true,
      include_image: true,
      summary_style: SummaryStyle::Concise,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderPreview {
  pub subject: String,
  pub html: String,
  pub text: String,
  pub weather_advisory: Option<String>,
  pub image_url: Option<String>,
  pub generated_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
  pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
  pub scheduler: SchedulerState,
  pub last_dispatch: Option<DateTime<Utc>>,
  pub next_run: Option<DateTime<Utc>>,
  pub google_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchedulerState {
  Idle,
  Waiting,
  Sending,
}

impl Default for HealthStatus {
  fn default() -> Self {
    Self {
      scheduler: SchedulerState::Idle,
      last_dispatch: None,
      next_run: None,
      google_connected: false,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
  pub id: Uuid,
  pub summary: String,
  pub start: DateTime<Utc>,
  pub end: DateTime<Utc>,
  pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleAuthStartResponse {
  pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleAuthPayload {
  pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleAuthResult {
  pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTokens {
  pub access_token: String,
  pub refresh_token: String,
  pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SenderSelection {
  pub address: String,
  #[serde(default)]
  pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfig {
  #[serde(default)]
  pub mail_api_base: Option<String>,
  #[serde(default)]
  pub mail_service_token: Option<String>,
  #[serde(default)]
  pub daily_sender: Option<SenderSelection>,
  #[serde(default)]
  pub noreply_sender: Option<SenderSelection>,
}

impl Default for SystemConfig {
  fn default() -> Self {
    Self {
      mail_api_base: None,
      mail_service_token: None,
      daily_sender: None,
      noreply_sender: None,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailSenderOption {
  pub id: String,
  pub address: String,
  pub display_name: Option<String>,
  pub kind: SenderKind,
  pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SenderKind {
  Account,
  Alias,
}
