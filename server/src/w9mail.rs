use crate::models::{MailSenderOption, SenderKind};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum W9MailError {
    #[error("mail api base not configured")]
    MissingBase,
    #[error("unauthorized")]
    Unauthorized,
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Clone)]
pub struct W9MailClient {
    http: reqwest::Client,
}

impl W9MailClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    fn build_url(base: &str, path: &str) -> String {
        format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches('/'))
    }

    async fn get<T: DeserializeOwned>(&self, base: &str, token: &str, path: &str) -> Result<T, W9MailError> {
        if base.trim().is_empty() {
            return Err(W9MailError::MissingBase);
        }
        let url = Self::build_url(base, path);
        let resp = self.http.get(url).bearer_auth(token).send().await?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            return Err(W9MailError::Unauthorized);
        }
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(W9MailError::InvalidResponse(body));
        }
        Ok(resp.json().await?)
    }

    pub async fn profile(&self, base: &str, token: &str) -> Result<W9MailProfile, W9MailError> {
        self.get(base, token, "/auth/me").await
    }

    pub async fn list_senders(&self, base: &str, token: &str) -> Result<Vec<MailSenderOption>, W9MailError> {
        #[derive(Deserialize)]
        struct AccountSummary {
            pub id: String,
            pub email: String,
            #[serde(rename = "displayName")]
            pub display_name: String,
            #[serde(rename = "isActive")]
            pub is_active: bool,
        }

        #[derive(Deserialize)]
        struct AliasSummary {
            pub id: String,
            #[serde(rename = "aliasEmail")]
            pub alias_email: String,
            #[serde(rename = "displayName")]
            pub display_name: Option<String>,
            #[serde(rename = "isActive")]
            pub is_active: bool,
        }

        let accounts: Vec<AccountSummary> = self.get(base, token, "/accounts").await?;
        let aliases: Vec<AliasSummary> = self.get(base, token, "/aliases").await?;

        let mut senders: Vec<MailSenderOption> = accounts
            .into_iter()
            .map(|acc| MailSenderOption {
                id: format!("account:{}", acc.id),
                address: acc.email,
                display_name: Some(acc.display_name),
                kind: SenderKind::Account,
                is_active: acc.is_active,
            })
            .collect();

        senders.extend(aliases.into_iter().map(|alias| MailSenderOption {
            id: format!("alias:{}", alias.id),
            address: alias.alias_email,
            display_name: alias.display_name,
            kind: SenderKind::Alias,
            is_active: alias.is_active,
        }));

        Ok(senders)
    }

    pub async fn send_email(&self, base: &str, token: &str, payload: &SendEmailPayload) -> Result<(), W9MailError> {
        if base.trim().is_empty() {
            return Err(W9MailError::MissingBase);
        }
        let url = Self::build_url(base, "/send");
        let resp = self
            .http
            .post(url)
            .header("X-API-Token", token)
            .json(payload)
            .send()
            .await?;

        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(W9MailError::Unauthorized);
        }

        let body_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(W9MailError::InvalidResponse(body_text));
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_text) {
            if let Some(status) = json.get("status").and_then(|s| s.as_str()) {
                if status == "error" {
                    let message = json.get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error from w9-mail API");
                    return Err(W9MailError::InvalidResponse(message.to_string()));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct W9MailProfile {
    pub id: String,
    pub email: String,
    pub role: String,
    #[serde(rename = "mustChangePassword")]
    pub must_change_password: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailPayload {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc: Option<String>,
    pub subject: String,
    pub body: String,
    #[serde(rename = "isHtml")]
    pub is_html: bool,
}
