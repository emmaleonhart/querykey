//! Port of server-go-old/internal/api/{router,handlers}.go to axum.
//! Same routes, same JSON contract (the Flutter app depends on it).

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

use crate::graph::GraphStore;
use crate::ingest::{IngestRequest, Pipeline};
use crate::models::{
    ConflictResolution, FollowUp, GraphDiff, Person, QuestionStatus, Task, VoiceProfile,
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
        // Instructions (who said what to whom) — vault-canonical, read
        // surface (written by ingest).
        .route("/api/instructions", get(list_instructions))
        // Voice profiles (speaker identity) — read + upsert; the audio
        // pipeline that fills embeddings is a separate todo.
        .route(
            "/api/voiceprofiles",
            get(list_voice_profiles).put(put_voice_profile),
        )
        .route("/api/persons/:id/voiceprofile", get(person_voice_profile))
        // Calendar: merged agenda (events incl. recurrence + tasks
        // with deadlines) for a window. ?from&to (rfc3339); defaults
        // to [now, now+30d].
        .route("/api/calendar", get(calendar_agenda))
        // P2P card layer (your key/query signal). Transport is the
        // open question — these are local: edit, the 24h propagation
        // safety valve, revert-before-propagation, read local peers.
        .route("/api/card", get(get_card).put(put_card))
        .route("/api/card/published", get(get_card_published))
        .route("/api/card/revert", post(revert_card))
        .route("/api/card/draft", post(draft_card))
        .route("/api/identity", get(get_identity))
        .route("/api/peers", get(list_peers_h))
        .route("/api/peers/:handle/card", get(get_peer_card_h))
        // Semantic wikilink graph (derived live from the canonical
        // vault — backend-independent, never stale).
        .route("/api/links", get(list_links))
        .route("/api/entities/:kind/:id/links", get(entity_links))
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
        if !crate::workflow::task_transition_ok(task.status, st) {
            return Json(json!({
                "error": "invalid status transition",
                "from": task.status,
                "to": st,
            }));
        }
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
    if !crate::workflow::conflict_transition_ok(c.resolution, body.resolution) {
        return Json(json!({
            "error": "invalid conflict resolution transition (a resolved conflict cannot return to unresolved)",
            "from": c.resolution,
            "to": body.resolution,
        }));
    }
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
    if !crate::workflow::question_transition_ok(q.status, QuestionStatus::Resolved) {
        return Json(json!({
            "error": "invalid question transition",
            "from": q.status,
            "to": QuestionStatus::Resolved,
        }));
    }
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

// ---- P2P card layer ----
//
// Local-only by design. Editing stages a card behind a 24h
// propagation delay (the privacy safety valve); the published
// snapshot is what a *future* transport would broadcast. What that
// transport is remains the deliberately-unresolved open question —
// no exchange/relay code here. Identity goes through the swappable
// `crate::identity` provider (GitHub today), never named inline.

fn card_state(s: &AppState) -> Value {
    s.vault.promote_due_card(); // lazily propagate anything now due
    let pending = s.vault.card_pending();
    json!({
        "card": s.vault.get_card(),
        "propagation": {
            "pending": pending.is_some(),
            "eligible_at": pending.as_ref().map(|(_, t)| t.to_rfc3339()),
            "published": s.vault.card_published().is_some(),
        }
    })
}

async fn get_card(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(card_state(&s))
}

#[derive(Deserialize)]
struct CardInput {
    handle: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    website: String,
    #[serde(default)]
    bio: String,
    #[serde(default)]
    offering: Vec<String>,
    #[serde(default)]
    looking_for: Vec<String>,
    #[serde(default = "crate::card::default_visibility")]
    visibility: String,
}

async fn put_card(
    State(s): State<Arc<AppState>>,
    Json(inp): Json<CardInput>,
) -> Json<Value> {
    let handle = crate::identity::default_provider()
        .normalize(&inp.handle)
        .to_string();
    let card = crate::card::Card {
        handle,
        name: inp.name,
        website: inp.website,
        bio: inp.bio,
        offering: inp.offering,
        looking_for: inp.looking_for,
        updated: chrono::Utc::now(),
        visibility: inp.visibility,
    };
    if let Err(e) = s.vault.stage_card_edit(&card) {
        return Json(json!({ "error": e.to_string() }));
    }
    Json(card_state(&s))
}

// The agent drafts the card's key/query from the PRM it built; the
// user approves via PUT /api/card (NOT saved here — and the 24h
// propagation valve guards anyway). Falls back to a deterministic
// heuristic when no agent is reachable, so it works offline.
async fn draft_card(State(s): State<Arc<AppState>>) -> Json<Value> {
    let digest = s.vault.prm_digest();
    let base = digest.current_card.clone().unwrap_or_else(|| crate::card::Card {
        handle: String::new(),
        name: String::new(),
        website: String::new(),
        bio: String::new(),
        offering: vec![],
        looking_for: vec![],
        updated: chrono::Utc::now(),
        visibility: crate::card::default_visibility(),
    });
    let agents = s.vault.agents_md();
    let prompt = crate::card::draft_prompt(&digest, agents.as_deref());
    let (card, source) = match s.bridge.chat(&prompt, &[]).await {
        Ok(reply) => match crate::card::parse_draft_reply(&reply, &base) {
            Some(c) => (c, "agent"),
            None => (crate::card::heuristic_draft(&digest, &base), "heuristic"),
        },
        Err(_) => (crate::card::heuristic_draft(&digest, &base), "heuristic"),
    };
    Json(json!({
        "draft": card,
        "source": source,
        "saved": false,
        "note": "review, then approve via PUT /api/card",
    }))
}

async fn get_card_published(State(s): State<Arc<AppState>>) -> Json<Value> {
    s.vault.promote_due_card();
    let pending = s.vault.card_pending();
    Json(json!({
        "published": s.vault.card_published(),
        "pending": pending.is_some(),
        "eligible_at": pending.as_ref().map(|(_, t)| t.to_rfc3339()),
    }))
}

async fn revert_card(State(s): State<Arc<AppState>>) -> Json<Value> {
    let reverted = s.vault.revert_pending_card();
    let mut v = card_state(&s);
    v["reverted"] = json!(reverted);
    Json(v)
}

async fn get_identity(State(s): State<Arc<AppState>>) -> Json<Value> {
    let p = crate::identity::default_provider();
    match s.vault.get_card() {
        Some(c) => {
            let h = p.normalize(&c.handle);
            Json(json!({
                "handle": h.as_str(),
                "scheme": h.scheme(),
                "profile_url": p.profile_url(&h),
            }))
        }
        None => Json(json!({
            "handle": null,
            "scheme": p.scheme(),
            "profile_url": null,
        })),
    }
}

async fn list_peers_h(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "peers": s.vault.list_peers() }))
}

async fn get_peer_card_h(
    State(s): State<Arc<AppState>>,
    Path(handle): Path<String>,
) -> Json<Value> {
    match s.vault.get_peer_card(&handle) {
        Some(c) => Json(serde_json::to_value(c).unwrap_or_else(|_| json!({}))),
        None => Json(json!({ "error": "peer card not found", "handle": handle })),
    }
}

// ---- Calendar ----

#[derive(Deserialize)]
struct CalRange {
    from: Option<String>,
    to: Option<String>,
}

fn parse_dt_opt(s: &Option<String>) -> Option<chrono::DateTime<chrono::Utc>> {
    s.as_deref()
        .and_then(|x| chrono::DateTime::parse_from_rfc3339(x).ok())
        .map(|d| d.with_timezone(&chrono::Utc))
}

async fn calendar_agenda(
    State(s): State<Arc<AppState>>,
    Query(q): Query<CalRange>,
) -> Json<Value> {
    let now = chrono::Utc::now();
    let from = parse_dt_opt(&q.from).unwrap_or(now);
    let to = parse_dt_opt(&q.to).unwrap_or_else(|| now + chrono::Duration::days(30));
    Json(json!({
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "agenda": s.vault.agenda(from, to),
    }))
}

// ---- Instructions / Voice profiles (canonical vault) ----

async fn list_instructions(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "instructions": s.vault.list_instructions() }))
}

async fn list_voice_profiles(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "voiceprofiles": s.vault.list_voice_profiles() }))
}

async fn person_voice_profile(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Value> {
    match s.vault.voice_profile_for_person(&id) {
        Some(vp) => Json(serde_json::to_value(vp).unwrap_or_else(|_| json!({}))),
        None => Json(json!({ "error": "no voice profile", "person": id })),
    }
}

async fn put_voice_profile(
    State(s): State<Arc<AppState>>,
    Json(mut vp): Json<VoiceProfile>,
) -> Json<Value> {
    if vp.id.is_nil() {
        vp.id = uuid::Uuid::new_v4();
    }
    let now = chrono::Utc::now();
    if vp.created_at.timestamp() == 0 {
        vp.created_at = now;
    }
    vp.last_updated = now;
    if let Err(e) = s.vault.upsert_voice_profile(&vp) {
        return Json(json!({ "error": e.to_string() }));
    }
    Json(serde_json::to_value(vp).unwrap())
}

// ---- semantic wikilink graph ----
//
// Computed live from the canonical vault (collect_links), so these
// are correct on every build config and never stale — the derived
// triple store is only a startup-projected convenience for SPARQL.

async fn list_links(State(s): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({ "links": s.vault.collect_links() }))
}

async fn entity_links(
    State(s): State<Arc<AppState>>,
    Path((kind, id)): Path<(String, String)>,
) -> Json<Value> {
    Json(json!({
        "from": s.vault.links_from(&kind, &id), // outgoing
        "to": s.vault.links_to(&kind, &id),     // backlinks
    }))
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
