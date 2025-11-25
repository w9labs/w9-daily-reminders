mod models;
mod store;
mod weather;
mod cerebras;
mod pollinations;
mod email;
mod routes;
mod google;
mod w9mail;

use axum::{routing::get, routing::post, Router};
use routes::{
  get_image_models, google_callback, google_start, health, list_mail_senders, preview, send_test_email, settings_get, settings_post,
  system_config_get, system_config_update,
};
use std::net::SocketAddr;
use store::DataStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();
  tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new(
      std::env::var("RUST_LOG").unwrap_or_else(|_| "w9_daily_reminders=info,axum=info".into()),
    ))
    .with(tracing_subscriber::fmt::layer())
    .init();

  let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".into());
  let store = DataStore::new(data_dir).await?;
  let mail_api_base = std::env::var("W9_MAIL_API_BASE").unwrap_or_else(|_| "https://w9.nu/api".into());
  let mail_service_token_env =
    std::env::var("W9_MAIL_SERVICE_TOKEN").ok().filter(|value| !value.trim().is_empty());

  let mail_client = w9mail::W9MailClient::new();

  let app_state = routes::AppState::new(store, mail_client, mail_api_base, mail_service_token_env);

  let app = Router::new()
    .route("/api/settings", get(settings_get).post(settings_post))
    .route("/api/reminders/preview", post(preview))
    .route("/api/reminders/send-test", post(send_test_email))
    .route("/api/system/config", get(system_config_get).post(system_config_update))
    .route("/api/system/senders", get(list_mail_senders))
    .route("/api/system/image-models", get(get_image_models))
    .route("/api/system/health", get(health))
    .route("/api/google/start", post(google_start))
    .route("/api/google/callback", post(google_callback))
    .with_state(app_state);

  let port: u16 = std::env::var("PORT").unwrap_or_else(|_| "8787".into()).parse().unwrap_or(8787);
  let addr = SocketAddr::from(([0, 0, 0, 0], port));
  tracing::info!(%addr, "starting w9 daily reminders backend");

  axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

  Ok(())
}
