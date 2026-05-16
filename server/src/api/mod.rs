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
use crate::models::{
    ConflictResolution, FollowUp, GraphDiff, Person, QuestionStatus, Task,
};
use crate::openclaw::Bridge;
use crate::ws::{ws_handler, Hub};

pub struct AppState {
    pub bridge: Arc<Bridge>,
    pub graph: Arc<dyn GraphStore>,
    pub vault: Arc<crate::vault::Vault>,
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

// Reads come from the canonical vault (full fidelity — no lossy
// graph reconstruction). Writes go to the vault first, then project
// into the derived graph (best-effort; the graph is rebuildable).

async fn list_persons(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "persons": s.vault.list_persons() }))
}

async fn create_person(
    State(s): State<Arc<AppState>>,
    Json(p): Json<Person>,
) -> Json<Value> {
    if let Err(e) = s.vault.upsert_person(&p) {
        return Json(json!({ "error": e.to_string() }));
    }
    let _ = s.graph.store_person(&p).await; // derived projection
    Json(serde_json::to_value(p).unwrap())
}

async fn person_tasks(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Value> {
    Json(json!({ "tasks": s.vault.tasks_for_person(&id) }))
}

async fn list_tasks(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "tasks": s.vault.list_tasks() }))
}

async fn create_task(
    State(s): State<Arc<AppState>>,
    Json(t): Json<Task>,
) -> Json<Value> {
    if let Err(e) = s.vault.upsert_task(&t) {
        return Json(json!({ "error": e.to_string() }));
    }
    let _ = s.graph.store_task(&t).await; // derived projection
    Json(serde_json::to_value(t).unwrap())
}

#[derive(Deserialize)]
struct TaskPatch {
    status: Option<crate::models::TaskStatus>,
    title: Option<String>,
    description: Option<String>,
}

async fn update_task(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(patch): Json<TaskPatch>,
) -> Json<Value> {
    // Real mutation now: the vault is canonical. Read the markdown,
    // apply the patch, write it back, re-project into the graph.
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Json(json!({ "error": format!("bad task id: {id}") })),
    };
    let mut task = match s.vault.get_task(&uuid) {
        Some(t) => t,
        None => return Json(json!({ "error": "task not found" })),
    };
    if let Some(st) = patch.status {
        task.status = st;
    }
    if let Some(t) = patch.title {
        task.title = t;
    }
    if let Some(d) = patch.description {
        task.description = d;
    }
    task.updated_at = chrono::Utc::now();
    if let Err(e) = s.vault.upsert_task(&task) {
        return Json(json!({ "error": e.to_string() }));
    }
    let _ = s.graph.store_task(&task).await; // derived projection
    Json(serde_json::to_value(task).unwrap())
}

// Conflicts/questions/followups now have a canonical on-disk form
// (R6). Reads come from the vault at full fidelity; resolutions are
// real markdown mutations (read → patch → write → project/broadcast),
// mirroring update_task. No more faked success / empty lists.

async fn list_conflicts(State(s): State<Arc<AppState>>) -> Json<Value> {
    let conflicts: Vec<_> = s
        .vault
        .list_conflicts()
        .into_iter()
        .filter(|c| c.resolution == ConflictResolution::Unresolved)
        .collect();
    Json(json!({ "conflicts": conflicts }))
}

#[derive(Deserialize)]
struct ConflictResolve {
    resolution: ConflictResolution,
    #[serde(default)]
    resolved_by: String,
}

async fn resolve_conflict(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ConflictResolve>,
) -> Json<Value> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Json(json!({ "error": format!("bad conflict id: {id}") })),
    };
    let mut c = match s.vault.get_conflict(&uuid) {
        Some(c) => c,
        None => return Json(json!({ "error": "conflict not found" })),
    };
    c.resolution = body.resolution;
    c.resolved_by = body.resolved_by;
    c.resolved_at = Some(chrono::Utc::now());
    if let Err(e) = s.vault.upsert_conflict(&c) {
        return Json(json!({ "error": e.to_string() }));
    }
    let _ = s.graph.store_conflict(&c).await; // derived projection
    s.hub.broadcast_graph_diff(&GraphDiff {
        resolved_conflicts: vec![c.id.to_string()],
        ..Default::default()
    });
    Json(serde_json::to_value(c).unwrap())
}

async fn list_questions(State(s): State<Arc<AppState>>) -> Json<Value> {
    let questions: Vec<_> = s
        .vault
        .list_questions()
        .into_iter()
        .filter(|q| q.status == QuestionStatus::Open)
        .collect();
    Json(json!({ "questions": questions }))
}

#[derive(Deserialize)]
struct QuestionResolve {
    #[serde(default)]
    resolution: String,
    #[serde(default)]
    resolved_by: String,
    #[serde(default)]
    resolved_via: String,
}

async fn resolve_question(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<QuestionResolve>,
) -> Json<Value> {
    let mut q = match s.vault.get_question(&id) {
        Some(q) => q,
        None => return Json(json!({ "error": "question not found" })),
    };
    q.status = QuestionStatus::Resolved;
    q.resolution = body.resolution;
    q.resolved_by = body.resolved_by;
    q.resolved_via = body.resolved_via;
    q.resolved_at = Some(chrono::Utc::now());
    if let Err(e) = s.vault.upsert_question(&q) {
        return Json(json!({ "error": e.to_string() }));
    }
    Json(serde_json::to_value(q).unwrap())
}

async fn list_followups(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "followups": s.vault.list_followups() }))
}

async fn create_followup(
    State(s): State<Arc<AppState>>,
    Json(mut f): Json<FollowUp>,
) -> Json<Value> {
    if f.id.is_empty() {
        f.id = uuid::Uuid::new_v4().to_string();
    }
    if f.created_at.timestamp() == 0 {
        f.created_at = chrono::Utc::now();
    }
    if let Err(e) = s.vault.upsert_followup(&f) {
        return Json(json!({ "error": e.to_string() }));
    }
    Json(serde_json::to_value(f).unwrap())
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
