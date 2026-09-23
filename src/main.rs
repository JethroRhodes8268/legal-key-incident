mod incident;
mod infrai_client;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use incident::{follow_up, MatterHandoff};
use infrai_client::{InfraiClient, InfraiError};
use serde_json::{json, Value};
use uuid::Uuid;

async fn handle_incident(
    State(client): State<InfraiClient>,
    Json(matter): Json<MatterHandoff>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if matter.matter_id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "matter_id is required"}))));
    }
    let next_action = follow_up(&matter);
    let request_id = Uuid::new_v4().to_string();
    let evidence = client.investigate(&request_id).await.map_err(|error| {
        let status = match &error {
            InfraiError::Rejected { status, .. } if status.is_client_error() => *status,
            _ => StatusCode::BAD_GATEWAY,
        };
        (status, Json(json!({"error": error.to_string()})))
    })?;
    Ok(Json(json!({"matter_id": matter.matter_id, "next_action": next_action,
        "request_id": request_id, "evidence": evidence})))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("INFRAI_API_KEY")?;
    let app = Router::new().route("/incident", post(handle_incident))
        .with_state(InfraiClient::new(key));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
