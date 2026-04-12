use axum::{
    extract::State, http::StatusCode, response::Html, routing::{get, post}, Json, Router,
};
use chrono::{Utc, NaiveTime};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_postgres::{Client, NoTls};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Client>,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub mail_api_url: String,
    pub mail_api_token: String,
    pub ai_api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateScheduleReq {
    pub user_email: String,
    pub send_time: Option<String>,
    pub timezone: Option<String>,
    pub ai_prompt: Option<String>,
    pub include_image: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TriggerReminderReq {
    pub user_email: String,
}

fn html_root() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html><html><head><title>W9 Daily Reminders</title></head><body style="background:#160c13;color:#fce126;font-family:monospace;text-align:center;padding:3rem"><h1>W9 DAILY REMINDERS</h1><p>AI + Google Calendar → Email Digest — PostgreSQL</p></body></html>"#)
}

async fn health_check(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    match state.db.query_one("SELECT 1", &[]).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({
            "status": "ok", "service": "w9-daily-reminders", "database": "connected",
            "timestamp": Utc::now().to_rfc3339()
        }))),
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
            "status": "error", "service": "w9-daily-reminders", "error": e.to_string()
        }))),
    }
}

async fn handle_create_schedule(
    State(state): State<AppState>,
    Json(req): Json<CreateScheduleReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    let id = Uuid::new_v4();
    let send_time = req.send_time.unwrap_or_else(|| "08:00:00".into());
    let timezone = req.timezone.unwrap_or_else(|| "Asia/Ho_Chi_Minh".into());
    let include_image = req.include_image.unwrap_or(true);

    // Parse time
    let _time = match NaiveTime::parse_from_str(&send_time, "%H:%M:%S") {
        Ok(t) => t,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid time format (HH:MM:SS)"}))),
    };

    match state.db.execute(
        "INSERT INTO reminder_schedules (id, user_email, send_time, timezone, ai_prompt, include_image) VALUES ($1,$2,$3,$4,$5,$6)",
        &[&id, &req.user_email, &send_time, &timezone, &req.ai_prompt, &include_image],
    ).await {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({
            "id": id.to_string(),
            "user_email": req.user_email,
            "send_time": send_time,
            "timezone": timezone,
            "include_image": include_image,
        }))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))),
    }
}

async fn handle_trigger(
    State(state): State<AppState>,
    Json(req): Json<TriggerReminderReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Get active schedule for user
    let row = match state.db.query_opt(
        "SELECT id, send_time, timezone, ai_prompt, include_image FROM reminder_schedules WHERE user_email = $1 AND is_active = true",
        &[&req.user_email],
    ).await {
        Ok(Some(r)) => r,
        _ => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "No active schedule found"}))),
    };

    let schedule_id: Uuid = row.get("id");
    let include_image: bool = row.get("include_image");
    let ai_prompt: Option<String> = row.get("ai_prompt");

    // TODO: Fetch Google Calendar events for today
    // TODO: Generate AI summary + image (via Pollinations)
    // TODO: Send email via w9-mail API

    // Log execution
    let log_id = Uuid::new_v4();
    let _ = state.db.execute(
        "INSERT INTO reminder_execution_log (id, schedule_id, events_count, email_sent) VALUES ($1,$2,$3,$4)",
        &[&log_id, &schedule_id, &0, &false],
    ).await;

    (StatusCode::OK, Json(serde_json::json!({
        "message": "Reminder processing started",
        "schedule_id": schedule_id.to_string(),
        "log_id": log_id.to_string(),
        "note": "Google Calendar + AI integration pending SMTP config"
    })))
}

async fn handle_list_schedules(
    State(state): State<AppState>,
) -> (StatusCode, Json<serde_json::Value>) {
    let rows = match state.db.query(
        "SELECT id, user_email, send_time, timezone, is_active, include_image FROM reminder_schedules ORDER BY created_at DESC",
        &[],
    ).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))),
    };

    let schedules: Vec<_> = rows.iter().map(|r| {
        serde_json::json!({
            "id": r.get::<_, Uuid>("id").to_string(),
            "user_email": r.get::<_, String>("user_email"),
            "send_time": r.get::<_, chrono::NaiveTime>("send_time").to_string(),
            "timezone": r.get::<_, String>("timezone"),
            "is_active": r.get::<_, bool>("is_active"),
            "include_image": r.get::<_, bool>("include_image"),
        })
    }).collect();

    (StatusCode::OK, Json(serde_json::json!(schedules)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer()).init();
    dotenvy::dotenv().ok();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8084".into());
    let db_url = std::env::var("W9_REMINDERS_DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://w9_admin:password@w9-postgres:5432/w9_reminders".into());
    let google_client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    let google_client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();
    let mail_api_url = std::env::var("W9_MAIL_API_URL").unwrap_or_else(|_| "https://mail.w9.nu".into());
    let mail_api_token = std::env::var("W9_MAIL_API_TOKEN").unwrap_or_default();
    let ai_api_key = std::env::var("POLLINATIONS_API_KEY").unwrap_or_default();

    tracing::info!("Connecting to PostgreSQL...");
    let (client, conn) = tokio_postgres::connect(&db_url, NoTls).await?;
    tokio::spawn(async move { if let Err(e) = conn.await { tracing::error!("DB: {}", e); } });
    client.query_one("SELECT 1", &[]).await?;
    tracing::info!("Connected to PostgreSQL");

    let state = AppState {
        db: Arc::new(client),
        google_client_id,
        google_client_secret,
        mail_api_url,
        mail_api_token,
        ai_api_key,
    };

    let router = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/schedules", post(handle_create_schedule))
        .route("/api/schedules", get(handle_list_schedules))
        .route("/api/trigger", post(handle_trigger))
        .fallback(|| async { html_root() })
        .with_state(state)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(CorsLayer::permissive()));

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("W9 Daily Reminders listening on {}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}
