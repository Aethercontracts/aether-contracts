use axum::{
    routing::{post, get},
    Router,
    Json,
    extract::State,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use serde_json::json;

use aether_quantum_vault::types::*;
use aether_quantum_vault::authenticity::AuthenticityEngine;
use aether_quantum_vault::escrow::EscrowEngine;

/// Shared application state
struct AppState {
    authenticity_engine: Mutex<AuthenticityEngine>,
}

#[tokio::main]
async fn main() {
    // Initialize the engine
    let state = Arc::new(AppState {
        authenticity_engine: Mutex::new(AuthenticityEngine::new()),
    });

    // Build the Axum router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/audit", post(run_audit))
        .layer(CorsLayer::permissive())
        .with_state(state);

    println!("════════════════════════════════════════════════════════");
    println!(" AetherContracts | Backend API Server");
    println!(" Listening on 0.0.0.0:8080");
    println!("════════════════════════════════════════════════════════");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Simple health check
async fn health_check() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "service": "aether-quantum-vault" }))
}

/// Request payload for an audit
#[derive(serde::Deserialize)]
struct AuditRequest {
    creator: CreatorId,
    events: Vec<EngagementEvent>,
    followers: Vec<FollowerFeatures>,
    content_score: f64,
}

/// Run a full authenticity audit
async fn run_audit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AuditRequest>,
) -> Json<Result<AuditReport, String>> {
    let mut engine = state.authenticity_engine.lock().await;
    
    // Update the content score from the Epsilon engine
    engine.set_content_score(payload.content_score);

    // Run the audit
    match engine.audit(payload.creator, &payload.events, &payload.followers) {
        Ok(report) => Json(Ok(report)),
        Err(e) => Json(Err(format!("Scoring error: {:?}", e))),
    }
}
