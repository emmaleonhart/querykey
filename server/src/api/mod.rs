//! Port of server-go-old/internal/api/{router,handlers}.go to axum.
//! Same routes, same JSON contract (the Flutter app depends on it).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

use crate::graph::GraphStore;
use crate::ingest::{IngestRequest, Pipeline};
use crate::models::{Person, Task};
use crate::openclaw::Bridge;
use crate::ws::{ws_handler, Hub};

pub struct AppState {
    pub bridge: Arc<Bridge>,
    pub graph: Arc<dyn GraphStore>,
    pub hub: Arc<Hub>,
    pub pipeline: Arc<Pipeline>,
}

pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health & status
        .route("/health", get(health))
        .route("/api/openclaw/status", get(openclaw_status))
        .route("/api/status", get(system_status))
        // WebSocket
        .route("/ws/chat", get(ws_handler))
        // Ingestion
        .route("/api/ingest", post(ingest))
        // Persons
        .route("/api/persons", get(list_persons).post(create_person))
        .route("/api/persons/:id/tasks", get(person_tasks))
        // Tasks
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/:id", patch(update_task))
        // Conflicts
        .route("/api/conflicts", get(list_conflicts))
        .route("/api/conflicts/:id/resolve", post(resolve_conflict))
        // Open questions
        .route("/api/questions", get(list_questions))
        .route("/api/questions/:id/resolve", post(resolve_question))
        // Follow-ups
        .route("/api/followups", get(list_followups).post(create_followup))
        // Local-agent management
        .route("/api/openclaw/kill", post(openclaw_kill))
        .route("/api/openclaw/restart", post(openclaw_restart))
        // Graph (SPARQL passthrough)
        .route("/api/graph/query", post(graph_query))
        // MCP server endpoint (model-agnostic agent entrypoint)
        .route("/mcp", post(crate::mcp::mcp_handler))
        .layer(cors)
        .with_state(state)
}

async fn health(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "clients": s.hub.client_count(),
        "graph": s.graph.backend(),
        "graph_ok": s.graph.ping().await.is_ok(),
    }))
}

async fn openclaw_status(State(s): State<Arc<AppState>>) -> Json<Value> {
    let st = s.bridge.detect().await;
    Json(json!({
        "available": st.available,
        "gateway_url": st.gateway_url,
        "agent_id": st.agent_id,
        "error": st.error,
    }))
}

async fn system_status(State(s): State<Arc<AppState>>) -> Json<Value> {
    let oc = s.bridge.detect().await;
    Json(json!({
        "openclaw": { "available": oc.available, "error": oc.error },
        "graph": { "backend": s.graph.backend(), "ok": s.graph.ping().await.is_ok() },
        "ws_clients": s.hub.client_count(),
    }))
}

async fn ingest(
    State(s): State<Arc<AppState>>,
    Json(req): Json<IngestRequest>,
) -> Json<Value> {
    match s.pipeline.process(&req).await {
        Ok(r) => Json(serde_json::to_value(r).unwrap_or_else(|_| json!({}))),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn list_persons(State(s): State<Arc<AppState>>) -> Json<Value> {
    let persons = s.graph.get_all_persons().await.unwrap_or_default();
    Json(json!({ "persons": persons }))
}

async fn create_person(
    State(s): State<Arc<AppState>>,
    Json(p): Json<Person>,
) -> Json<Value> {
    match s.graph.store_person(&p).await {
        Ok(()) => Json(serde_json::to_value(p).unwrap()),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn person_tasks(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Value> {
    let tasks = s.graph.get_tasks_for_person(&id).await.unwrap_or_default();
    Json(json!({ "tasks": tasks }))
}

async fn list_tasks(State(s): State<Arc<AppState>>) -> Json<Value> {
    let tasks = s.graph.get_all_tasks().await.unwrap_or_default();
    Json(json!({ "tasks": tasks }))
}

async fn create_task(
    State(s): State<Arc<AppState>>,
    Json(t): Json<Task>,
) -> Json<Value> {
    match s.graph.store_task(&t).await {
        Ok(()) => Json(serde_json::to_value(t).unwrap()),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn update_task(
    State(_s): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(_body): Json<Value>,
) -> Json<Value> {
    // Honest: mutation is the canonical-markdown write path, which is
    // not built yet (the derived graph is read-only/rebuildable, and
    // loka has no SPARQL UPDATE writer — see R4-2). Not faking success.
    Json(json!({
        "id": id,
        "status": "not_implemented",
        "detail": "task mutation goes through the canonical markdown write path (docs/markdown-schema.md), not yet implemented"
    }))
}

async fn list_conflicts(State(s): State<Arc<AppState>>) -> Json<Value> {
    let conflicts = s.graph.get_unresolved_conflicts().await.unwrap_or_default();
    Json(json!({ "conflicts": conflicts }))
}

// Conflicts/questions/followups belong to the open-questions model on
// the canonical markdown layer (not yet built). These return honest
// "not implemented" rather than faking success; lists are empty
// because nothing is persisted yet.
const NI: &str = "not implemented — canonical markdown open-questions model (docs/markdown-schema.md) not built yet";

async fn resolve_conflict(Path(id): Path<String>) -> Json<Value> {
    Json(json!({ "id": id, "status": "not_implemented", "detail": NI }))
}

async fn list_questions() -> Json<Value> {
    Json(json!({ "questions": [] }))
}
async fn resolve_question(Path(id): Path<String>) -> Json<Value> {
    Json(json!({ "id": id, "status": "not_implemented", "detail": NI }))
}
async fn list_followups() -> Json<Value> {
    Json(json!({ "followups": [] }))
}
async fn create_followup(Json(_body): Json<Value>) -> Json<Value> {
    Json(json!({ "status": "not_implemented", "detail": NI }))
}

async fn openclaw_kill(State(s): State<Arc<AppState>>) -> Json<Value> {
    let r = s.bridge.force_kill();
    Json(json!({ "ok": r.is_ok(), "error": r.err().map(|e| e.to_string()) }))
}

async fn openclaw_restart(State(s): State<Arc<AppState>>) -> Json<Value> {
    s.bridge.clone().ensure_gateway().await;
    Json(json!({ "ok": true }))
}

#[derive(Deserialize)]
struct GraphQuery {
    query: String,
}

async fn graph_query(
    State(s): State<Arc<AppState>>,
    Json(q): Json<GraphQuery>,
) -> Json<Value> {
    match s.graph.query(&q.query).await {
        Ok(r) => Json(serde_json::to_value(r).unwrap_or_else(|_| json!({}))),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}
