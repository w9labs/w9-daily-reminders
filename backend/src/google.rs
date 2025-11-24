use crate::models::{CalendarEvent, GoogleTokens};
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum GoogleError {
  #[error("missing google oauth config")]
  MissingConfig,
  #[error("request failed: {0}")]
  Request(#[from] reqwest::Error),
  #[error("response missing field {0}")]
  Invalid(&'static str),
  #[error("time parsing failed: {0}")]
  TimeParse(String),
}

#[derive(Clone)]
pub struct GoogleClient {
  http: reqwest::Client,
  client_id: String,
  client_secret: String,
  redirect_uri: String,
}

impl GoogleClient {
  pub fn new() -> Result<Self, GoogleError> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").map_err(|_| GoogleError::MissingConfig)?;
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").map_err(|_| GoogleError::MissingConfig)?;
    let redirect_uri = std::env::var("GOOGLE_REDIRECT_URI").map_err(|_| GoogleError::MissingConfig)?;
    Ok(Self {
      http: reqwest::Client::new(),
      client_id,
      client_secret,
      redirect_uri,
    })
  }

  pub fn auth_url(&self, state: &str) -> String {
    let scope = urlencoding::encode("https://www.googleapis.com/auth/calendar.readonly");
    format!(
      "https://accounts.google.com/o/oauth2/v2/auth?scope={scope}&access_type=offline&include_granted_scopes=true&response_type=code&prompt=consent&state={state}&redirect_uri={redirect}&client_id={client}",
      scope = scope,
      state = urlencoding::encode(state),
      redirect = urlencoding::encode(&self.redirect_uri),
      client = urlencoding::encode(&self.client_id),
    )
  }

  pub async fn exchange_code(&self, code: &str) -> Result<GoogleTokens, GoogleError> {
    self
      .request_tokens(&serde_json::json!({
        "code": code,
        "client_id": self.client_id,
        "client_secret": self.client_secret,
        "redirect_uri": self.redirect_uri,
        "grant_type": "authorization_code",
      }), None)
      .await
  }

  pub async fn refresh_tokens(&self, refresh_token: &str) -> Result<GoogleTokens, GoogleError> {
    self
      .request_tokens(&serde_json::json!({
        "refresh_token": refresh_token,
        "client_id": self.client_id,
        "client_secret": self.client_secret,
        "grant_type": "refresh_token",
      }), Some(refresh_token.to_string()))
      .await
  }

  pub async fn list_events(
    &self,
    tokens: &GoogleTokens,
  ) -> Result<(Vec<CalendarEvent>, Option<GoogleTokens>), GoogleError> {
    let mut active_tokens = tokens.clone();
    let mut updated_tokens = None;
    if needs_refresh(&active_tokens) {
      active_tokens = self.refresh_tokens(&active_tokens.refresh_token).await?;
      updated_tokens = Some(active_tokens.clone());
    }

    let now = Utc::now().to_rfc3339();
    let url = format!("https://www.googleapis.com/calendar/v3/calendars/primary/events?singleEvents=true&orderBy=startTime&timeMin={}&maxResults=10", urlencoding::encode(&now));
    let resp: EventsResponse = self
      .http
      .get(url)
      .bearer_auth(&active_tokens.access_token)
      .send()
      .await?
      .error_for_status()? 
      .json()
      .await?;

    let events = resp
      .items
      .into_iter()
      .filter_map(|item| parse_event(item).ok())
      .collect();

    Ok((events, updated_tokens))
  }

  async fn request_tokens(
    &self,
    payload: &serde_json::Value,
    existing_refresh: Option<String>,
  ) -> Result<GoogleTokens, GoogleError> {
    let resp: TokenResponse = self
      .http
      .post("https://oauth2.googleapis.com/token")
      .form(payload)
      .send()
      .await?
      .error_for_status()? 
      .json()
      .await?;

    let refresh = resp.refresh_token.or(existing_refresh).ok_or(GoogleError::Invalid("refresh_token"))?;
    let ttl = (resp.expires_in - 30).max(60);
    Ok(GoogleTokens {
      access_token: resp.access_token,
      refresh_token: refresh,
      expires_at: Utc::now() + Duration::seconds(ttl),
    })
  }
}

fn needs_refresh(tokens: &GoogleTokens) -> bool {
  Utc::now() >= tokens.expires_at
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
  access_token: String,
  expires_in: i64,
  #[serde(default)]
  refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EventsResponse {
  items: Vec<GoogleEvent>,
}

#[derive(Debug, Deserialize)]
struct GoogleEvent {
  id: String,
  summary: Option<String>,
  location: Option<String>,
  start: GoogleDateTime,
  end: GoogleDateTime,
}

#[derive(Debug, Deserialize)]
struct GoogleDateTime {
  #[serde(rename = "dateTime")]
  date_time: Option<String>,
  date: Option<String>,
}

fn parse_event(event: GoogleEvent) -> Result<CalendarEvent, GoogleError> {
  let start = parse_dt(event.start)?;
  let end = parse_dt(event.end)?;
  Ok(CalendarEvent {
    id: Uuid::new_v4(),
    summary: event.summary.unwrap_or_else(|| "(busy block)".into()),
    start,
    end,
    location: event.location,
  })
}

fn parse_dt(value: GoogleDateTime) -> Result<DateTime<Utc>, GoogleError> {
  if let Some(dt) = value.date_time {
    DateTime::parse_from_rfc3339(&dt)
      .map(|dt| dt.with_timezone(&Utc))
      .map_err(|err| GoogleError::TimeParse(err.to_string()))
  } else if let Some(date) = value.date {
    NaiveDate::parse_from_str(&date, "%Y-%m-%d")
      .map_err(|err| GoogleError::TimeParse(err.to_string()))
      .map(|d| NaiveDateTime::new(d, NaiveTime::from_hms_opt(0, 0, 0).unwrap()))
      .map(|dt| DateTime::<Utc>::from_utc(dt, Utc))
  } else {
    Err(GoogleError::Invalid("start"))
  }
}
