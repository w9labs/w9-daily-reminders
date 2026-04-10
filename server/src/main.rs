use axum::{routing::get, Router, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use tower_http::{cors::CorsLayer, trace::TraceLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"status": "ok", "service": "w9-daily-reminders", "timestamp": Utc::now().to_rfc3339()})))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry().with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "w9_reminders=debug".into())).with(tracing_subscriber::fmt::layer()).init();
    dotenvy::dotenv().ok();
    let port = std::env::var("PORT").unwrap_or_else(|_| "8084".to_string());
    let router = Router::new().route("/api/health", get(health_check)).nest_service("/", ServeDir::new("site/pkg")).layer(tower::ServiceBuilder::new().layer(CorsLayer::permissive()));
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("W9 Daily Reminders listening on {}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}
