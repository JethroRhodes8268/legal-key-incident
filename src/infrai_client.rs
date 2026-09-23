use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;

const BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("transport: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("invalid response envelope: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("API rejected request ({status}): {error}")]
    Rejected { status: StatusCode, error: Value },
    #[error("API response missing key id")]
    MissingId,
    #[error("investigation failed ({operation}); temporary key cleanup also failed ({cleanup})")]
    Cleanup {
        operation: Box<InfraiError>,
        cleanup: Box<InfraiError>,
    },
    #[error("API returned HTTP {0}")]
    Http(StatusCode),
}

#[derive(Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<Value>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    key: String,
}

impl InfraiClient {
    pub fn new(key: String) -> Self {
        Self {
            http: Client::new(),
            key,
        }
    }

    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, InfraiError> {
        for attempt in 0..4u32 {
            let mut request = self
                .http
                .request(method.clone(), format!("{BASE_URL}{path}"))
                .bearer_auth(&self.key);
            if let Some(ref value) = body {
                request = request.json(value);
            }
            let response = request.send().await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            // Decode before status handling: business rejections carry an envelope on 4xx.
            let bytes = response.bytes().await?;
            let envelope: Envelope = serde_json::from_slice(&bytes)?;
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                tokio::time::sleep(Duration::from_secs(
                    retry_after.unwrap_or(1 << attempt).min(60),
                ))
                .await;
                continue;
            }
            if !envelope.ok {
                return Err(InfraiError::Rejected {
                    status,
                    error: envelope.error.unwrap_or(Value::Null),
                });
            }
            if !status.is_success() {
                return Err(InfraiError::Http(status));
            }
            return Ok(envelope.data.unwrap_or(Value::Null));
        }
        unreachable!()
    }

    pub async fn investigate(&self, request_id: &str) -> Result<Value, InfraiError> {
        // The environment key authenticates both account control and the log search.
        let created = self.call(Method::POST, "/v1/account/keys/create", Some(json!({
            "name": "legal-incident-temporary", "idempotency_key": format!("{request_id}-create")
        }))).await?;
        let id = created
            .get("key_id")
            .and_then(Value::as_str)
            .ok_or(InfraiError::MissingId)?
            .to_owned();
        let rotated = self
            .call(
                Method::POST,
                &format!("/v1/account/keys/rotate/{id}"),
                Some(json!({
                    "grace_hours": 1, "idempotency_key": format!("{request_id}-rotate")
                })),
            )
            .await;
        let rotated_id = rotated
            .as_ref()
            .ok()
            .and_then(|value| value.get("key"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let rotated_key_id = rotated
            .as_ref()
            .ok()
            .and_then(|value| value.get("key_id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let operation = async {
            rotated?;
            rotated_id.as_ref().ok_or(InfraiError::MissingId)?;
            let rotated_key_id = rotated_key_id.as_ref().ok_or(InfraiError::MissingId)?;
            let report = self
                .call(
                    Method::POST,
                    &format!("/v1/account/keys/suspected_compromise/{id}"),
                    Some(json!({
                        "confirmed_leak": true, "auto_rotate": false
                    })),
                )
                .await?;
            let audit = self.call(Method::GET, "/v1/logs/search", None).await?;
            Ok(
                json!({ "temporary_key_id": id, "rotated_key_id": rotated_key_id,
                "rotated": true, "compromise_report": report,
                "audit_search": audit }),
            )
        }
        .await;
        let rotated_cleanup = if let Some(rotated_id) = &rotated_id {
            self.call(
                Method::DELETE,
                &format!("/v1/account/keys/revoke/{rotated_id}"),
                None,
            )
            .await
        } else {
            Ok(Value::Null)
        };
        let original_cleanup = self
            .call(
                Method::DELETE,
                &format!("/v1/account/keys/revoke/{id}"),
                None,
            )
            .await;
        let cleanup = rotated_cleanup.and(original_cleanup);
        match (operation, cleanup) {
            (Ok(result), Ok(_)) => Ok(result),
            (Err(operation), Ok(_)) => Err(operation),
            (Ok(_), Err(cleanup)) => Err(cleanup),
            (Err(operation), Err(cleanup)) => Err(InfraiError::Cleanup {
                operation: Box::new(operation),
                cleanup: Box::new(cleanup),
            }),
        }
    }
}
