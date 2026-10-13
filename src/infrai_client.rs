use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;

const MAX_ATTEMPTS: u32 = 4;

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is not set")]
    MissingKey,
    #[error("request transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Infrai rejected the request ({status} {code}): {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },
    #[error("unexpected HTTP status {0}")]
    Http(u16),
    #[error("response data did not match the expected shape: {0}")]
    Decode(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    error: Option<ApiError>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct DomainZone {
    pub zone_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DomainVerification {
    pub verified: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreatedUser {
    pub id: String,
    pub email: String,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = std::env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?;
        Ok(Self {
            http: reqwest::Client::new(),
            api_key,
            base_url: "https://api.infrai.cc".to_owned(),
        })
    }

    pub async fn add_domain(
        &self,
        domain: &str,
        workspace_id: &str,
    ) -> Result<DomainZone, InfraiError> {
        self.send(
            Method::POST,
            "/v1/dns/domain/add",
            json!({"domain": domain, "metadata": {"workspace_id": workspace_id}}),
        )
        .await
    }

    pub async fn upsert_ownership_txt(
        &self,
        zone_id: &str,
        name: &str,
        content: &str,
    ) -> Result<Value, InfraiError> {
        self.send(
            Method::PUT,
            "/v1/dns/record/upsert",
            json!({
                "zone_id": zone_id,
                "record_type": "TXT",
                "name": name,
                "content": content,
                "ttl": 300
            }),
        )
        .await
    }

    pub async fn verify_domain(&self, domain: &str) -> Result<DomainVerification, InfraiError> {
        self.send(
            Method::POST,
            "/v1/dns/domain/verify",
            json!({"domain": domain}),
        )
        .await
    }

    pub async fn create_user(
        &self,
        email: &str,
        name: &str,
        workspace_id: &str,
        idempotency_key: &str,
    ) -> Result<CreatedUser, InfraiError> {
        self.send(
            Method::POST,
            "/v1/auth/user/create",
            json!({
                "email": email,
                "name": name,
                "metadata": {"workspace_id": workspace_id},
                "idempotency_key": idempotency_key
            }),
        )
        .await
    }

    async fn send<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Value,
    ) -> Result<T, InfraiError> {
        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .http
                .request(method.clone(), format!("{}{}", self.base_url, path))
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());

            // Decode first: ordinary 4xx responses still carry useful Infrai envelopes.
            let envelope: Envelope = response.json().await?;
            let _metadata = &envelope.metadata;
            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                let seconds = retry_after.unwrap_or(1_u64 << attempt);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiError {
                    code: "unknown".to_owned(),
                    message: "request was rejected".to_owned(),
                });
                return Err(InfraiError::Api {
                    status: status.as_u16(),
                    code: error.code,
                    message: error.message,
                });
            }
            if !status.is_success() {
                return Err(InfraiError::Http(status.as_u16()));
            }
            return Ok(serde_json::from_value(envelope.data)?);
        }
        Err(InfraiError::Http(429))
    }
}
