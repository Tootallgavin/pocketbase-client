use crate::httpc::Httpc;
use crate::{collections::CollectionsManager, logs::LogsManager, records::RecordsManager};
use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Deserialize)]
struct AuthSuccessResponse {
    token: String,
}

#[derive(Debug, Clone)]
pub struct NoAuth;

#[derive(Debug, Clone)]
pub struct Auth;

#[derive(Debug, Clone)]
pub struct Client<State> {
    pub base_url: String,
    pub auth_token: Option<String>,
    pub state: State,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HealthCheckResponse {
    pub code: i32,
    pub message: String,
}

#[derive(Error, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Error, Serialize)]
#[error("{message} ({status})")]
pub struct ErrorResponse {
    pub data: HashMap<String, ValidationError>,
    pub message: String,
    pub status: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthError {
    Validation(ErrorResponse),
    Other(String),
}

impl std::error::Error for AuthError {}

impl From<ErrorResponse> for AuthError {
    fn from(err: ErrorResponse) -> Self {
        AuthError::Validation(err)
    }
}

impl From<anyhow::Error> for AuthError {
    fn from(err: anyhow::Error) -> Self {
        AuthError::Other(err.to_string())
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(err: reqwest::Error) -> Self {
        AuthError::Other(err.to_string())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "variant", content = "payload")]
enum AuthErrorRepr {
    Validation(ErrorResponse),
    Other(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repr = match self {
            AuthError::Validation(err) => AuthErrorRepr::Validation(err.clone()),
            AuthError::Other(msg) => AuthErrorRepr::Other(msg.clone()),
        };

        match serde_json::to_string(&repr) {
            Ok(json_str) => write!(f, "{}", json_str),
            Err(_) => write!(
                f,
                r#"{{"variant":"Other","payload":"{}"}}"#,
                self.to_string()
            ),
        }
    }
}

impl FromStr for AuthError {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match serde_json::from_str::<AuthErrorRepr>(s) {
            Ok(AuthErrorRepr::Validation(err)) => Ok(AuthError::Validation(err)),
            Ok(AuthErrorRepr::Other(msg)) => Ok(AuthError::Other(msg)),
            Err(_) => Ok(AuthError::Other(s.to_string())),
        }
    }
}

impl<A> Client<A> {
    pub fn collections(&self) -> CollectionsManager<A> {
        CollectionsManager { client: self }
    }

    pub async fn health_check(&self) -> Result<HealthCheckResponse> {
        let url = format!("{}/api/health", self.base_url);
        let response = Httpc::get(self, &url, None)
            .await
            .map_err(|e| anyhow!("Network error: {}", e))?;

        let hc = response.json::<HealthCheckResponse>().await?;
        Ok(hc)
    }

    pub fn logs(&self) -> LogsManager<A> {
        LogsManager { client: self }
    }

    pub fn records(&self, record_name: &'static str) -> RecordsManager<A> {
        RecordsManager {
            client: self,
            name: record_name,
        }
    }
}

impl Client<NoAuth> {
    /// Construct a new “no‐auth” client:
    pub fn new(base_url: &str) -> Self {
        Client {
            base_url: base_url.to_string(),
            auth_token: None,
            state: NoAuth,
        }
    }

    /// Attempt to authenticate with identity/password. On success, return `Client<Auth>`.
    pub async fn auth_with_password(
        &self,
        collection: &str,
        identifier: &str,
        secret: &str,
    ) -> Result<Client<Auth>, AuthError> {
        let url = format!(
            "{}/api/collections/{}/auth-with-password",
            self.base_url, collection
        );
        let auth_payload = json!({
            "identity": identifier,
            "password": secret
        });

        let response = Httpc::post(self, &url, auth_payload.to_string()).await?;

        match response.status() {
            StatusCode::OK => {
                let raw_response = response.json::<AuthSuccessResponse>().await?;
                Ok(Client {
                    base_url: self.base_url.clone(),
                    state: Auth,
                    auth_token: Some(raw_response.token),
                })
            }

            status if status.is_client_error() => {
                let err_body = response.json::<ErrorResponse>().await?;
                Err(AuthError::Validation(err_body))
            }

            other => {
                let text = response.text().await.unwrap_or_else(|_| "<no body>".into());
                Err(AuthError::Other(format!(
                    "Unexpected status {} with body: {}",
                    other, text
                )))
            }
        }
    }
}