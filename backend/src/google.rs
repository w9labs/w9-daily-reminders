use crate::models::{CalendarEvent, GoogleTokens, Todo};
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
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
    let scope = urlencoding::encode("https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/tasks.readonly");
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

  pub async fn list_todos(
    &self,
    tokens: &GoogleTokens,
  ) -> Result<(Vec<Todo>, Option<GoogleTokens>), GoogleError> {
    let mut active_tokens = tokens.clone();
    let mut updated_tokens = None;
    if needs_refresh(&active_tokens) {
      active_tokens = self.refresh_tokens(&active_tokens.refresh_token).await?;
      updated_tokens = Some(active_tokens.clone());
    }

    // First, get all task lists
    let lists_url = "https://tasks.googleapis.com/tasks/v1/users/@me/lists";
    let lists_resp: TaskListsResponse = match self
      .http
      .get(lists_url)
      .bearer_auth(&active_tokens.access_token)
      .send()
      .await?
      .error_for_status()
    {
      Ok(resp) => resp.json().await?,
      Err(e) => {
        tracing::warn!(?e, "failed to fetch task lists");
        return Ok((vec![], updated_tokens));
      }
    };

    // Fetch tasks from all task lists (not just default)
    let mut all_todos = Vec::new();
    for task_list in lists_resp.items {
      let tasks_url = format!(
        "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks?showCompleted=false&showHidden=false&maxResults=100",
        urlencoding::encode(&task_list.id)
      );
      
      match self
        .http
        .get(&tasks_url)
        .bearer_auth(&active_tokens.access_token)
        .send()
        .await?
        .error_for_status()
      {
        Ok(resp) => {
          let tasks_resp: TasksResponse = match resp.json().await {
            Ok(t) => t,
            Err(e) => {
              tracing::warn!(?e, list_id = %task_list.id, "failed to parse tasks response");
              continue;
            }
          };
          
          if let Some(items) = tasks_resp.items {
            for item in items {
              if let Ok(todo) = parse_todo(item) {
                all_todos.push(todo);
              }
            }
          }
        }
        Err(e) => {
          tracing::warn!(?e, list_id = %task_list.id, "failed to fetch tasks from list");
          continue;
        }
      }
    }

    Ok((all_todos, updated_tokens))
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
      .map(|dt| Utc.from_utc_datetime(&dt))
  } else {
    Err(GoogleError::Invalid("start"))
  }
}

#[derive(Debug, Deserialize)]
struct TaskListsResponse {
  items: Vec<TaskList>,
}

#[derive(Debug, Deserialize)]
struct TaskList {
  id: String,
  title: String,
}

#[derive(Debug, Deserialize)]
struct TasksResponse {
  #[serde(default)]
  items: Option<Vec<GoogleTask>>,
}

#[derive(Debug, Deserialize)]
struct GoogleTask {
  id: String,
  title: String,
  #[serde(default)]
  notes: Option<String>,
  #[serde(default)]
  due: Option<String>,
  #[serde(default)]
  status: String,
  #[serde(default)]
  hidden: Option<bool>,
}

fn parse_todo(task: GoogleTask) -> Result<Todo, GoogleError> {
  // Skip hidden tasks
  if task.hidden.unwrap_or(false) {
    return Err(GoogleError::Invalid("hidden task"));
  }
  
  // Parse due date - Google Tasks API returns RFC3339 format
  let due = task.due.and_then(|d| {
    // Try RFC3339 first
    DateTime::parse_from_rfc3339(&d)
      .map(|dt| dt.with_timezone(&Utc))
      .ok()
      .or_else(|| {
        // Try date-only format (YYYY-MM-DD)
        NaiveDate::parse_from_str(&d, "%Y-%m-%d")
          .ok()
          .and_then(|date| {
            NaiveTime::from_hms_opt(23, 59, 59)
              .map(|time| NaiveDateTime::new(date, time))
              .and_then(|ndt| ndt.and_local_timezone(Utc).single())
          })
      })
  });
  
  Ok(Todo {
    id: Uuid::new_v4(),
    title: task.title,
    notes: task.notes,
    due,
    completed: task.status == "completed",
  })
}
