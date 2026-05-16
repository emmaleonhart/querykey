//! Port of server-go-old/internal/ingest/pipeline.go.
//! Accept raw input -> normalize -> local-agent analyze -> parse ->
//! store in the (derived) graph -> broadcast a graph diff.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use chrono::Utc;

use crate::graph::GraphStore;
use crate::models::{
    AnalysisResult, Conflict, ConflictResolution, ConflictType, Event, GraphDiff, InputType,
    Task, TaskStatus,
};
use crate::openclaw::Bridge;
use crate::ws::Hub;

#[derive(Debug, Clone, Deserialize)]
pub struct IngestRequest {
    pub input_type: InputType,
    pub content: String,
    #[serde(default)]
    pub submitted_by: String,
    #[serde(default)]
    pub source_context: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IngestResult {
    pub ingest_id: String,
    #[serde(flatten)]
    pub analysis: AnalysisResult,
}

pub struct Pipeline {
    bridge: Arc<Bridge>,
    graph: Arc<dyn GraphStore>,
    hub: Arc<Hub>,
}

impl Pipeline {
    pub fn new(bridge: Arc<Bridge>, graph: Arc<dyn GraphStore>, hub: Arc<Hub>) -> Self {
        Self { bridge, graph, hub }
    }

    pub async fn process(&self, req: &IngestRequest) -> anyhow::Result<IngestResult> {
        let ingest_id = uuid::Uuid::new_v4().to_string();
        let normalized = self.normalize(req);
        let analysis_json = self
            .bridge
            .analyze(&normalized, &req.source_context)
            .await
            .unwrap_or_default();

        let analysis = parse_analysis(&analysis_json, &ingest_id);
        self.store_results(&analysis).await;
        self.broadcast_results(&analysis);

        Ok(IngestResult {
            ingest_id,
            analysis,
        })
    }

    /// Mirrors pipeline.go normalize(). For text inputs the content is
    /// already text; binary inputs (screenshots/audio) are TODO(port).
    fn normalize(&self, req: &IngestRequest) -> String {
        match req.input_type {
            InputType::Screenshot | InputType::VoiceNote | InputType::RecordedAudio => {
                // TODO(port): OCR / transcription — see pipeline.go
                req.content.clone()
            }
            _ => req.content.clone(),
        }
    }

    /// Mirrors pipeline.go storeResults(): persist tasks + conflicts.
    /// (Events are broadcast but not persisted, matching Go.)
    async fn store_results(&self, a: &AnalysisResult) {
        for t in &a.tasks {
            if let Err(e) = self.graph.store_task(t).await {
                tracing::warn!("[ingest] failed to store task: {e}");
            }
        }
        for c in &a.conflicts {
            if let Err(e) = self.graph.store_conflict(c).await {
                tracing::warn!("[ingest] failed to store conflict: {e}");
            }
        }
    }

    /// Mirrors pipeline.go broadcastResults(): tasks+events as added
    /// nodes, conflicts as new_conflicts, over the typed GraphDiff.
    fn broadcast_results(&self, a: &AnalysisResult) {
        let mut added: Vec<serde_json::Value> = Vec::new();
        for t in &a.tasks {
            if let Ok(v) = serde_json::to_value(t) {
                added.push(v);
            }
        }
        for e in &a.events {
            if let Ok(v) = serde_json::to_value(e) {
                added.push(v);
            }
        }
        let diff = GraphDiff {
            added_nodes: added,
            new_conflicts: a.conflicts.clone(),
            ..Default::default()
        };
        self.hub.broadcast_graph_diff(&diff);
    }
}

/// Relaxed shape the agent actually returns (loose fields, no ids or
/// timestamps). Mirrors pipeline.go's anonymous parse struct.
#[derive(Default, Deserialize)]
struct RawAnalysis {
    #[serde(default)]
    tasks: Vec<RawTask>,
    #[serde(default)]
    events: Vec<RawEvent>,
    #[serde(default)]
    conflicts: Vec<RawConflict>,
}
#[derive(Default, Deserialize)]
struct RawTask {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    assigned_to: String,
    #[serde(default)]
    assigned_by: String,
    #[serde(default)]
    deadline: String,
    #[serde(default)]
    confidence: f64,
}
#[derive(Default, Deserialize)]
struct RawEvent {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    start_time: String,
    #[serde(default)]
    end_time: String,
    #[serde(default)]
    confidence: f64,
}
#[derive(Default, Deserialize)]
struct RawConflict {
    #[serde(default, rename = "type")]
    conflict_type: String,
    #[serde(default)]
    explanation: String,
}

/// Port of pipeline.go parseAnalysis(): extract the JSON object (the
/// agent wraps it in prose/fences), parse the *relaxed* schema, then
/// construct full typed models with generated ids + timestamps and
/// `source_messages = [ingest_id]`. Never errors — returns whatever
/// parsed (empty on failure), matching Go's "store raw" fallback.
pub fn parse_analysis(s: &str, ingest_id: &str) -> AnalysisResult {
    let json = extract_json(s);
    let raw: RawAnalysis = serde_json::from_str(&json).unwrap_or_default();
    let now = Utc::now();
    let mut out = AnalysisResult::default();
    for t in raw.tasks {
        out.tasks.push(Task {
            id: uuid::Uuid::new_v4(),
            title: t.title,
            description: t.description,
            status: TaskStatus::Extracted,
            assigned_to: t.assigned_to,
            assigned_by: t.assigned_by,
            deadline: chrono::DateTime::parse_from_rfc3339(&t.deadline)
                .ok()
                .map(|d| d.with_timezone(&Utc)),
            confidence: t.confidence,
            ambiguity_score: 0.0,
            source_messages: vec![ingest_id.to_string()],
            created_at: now,
            updated_at: now,
        });
    }
    for e in raw.events {
        out.events.push(Event {
            id: uuid::Uuid::new_v4(),
            title: e.title,
            description: e.description,
            start_time: chrono::DateTime::parse_from_rfc3339(&e.start_time)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or(now),
            end_time: chrono::DateTime::parse_from_rfc3339(&e.end_time)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or(now),
            participants: Vec::new(),
            confidence: e.confidence,
            source_messages: vec![ingest_id.to_string()],
            created_at: now,
        });
    }
    for c in raw.conflicts {
        out.conflicts.push(Conflict {
            id: uuid::Uuid::new_v4(),
            conflict_type: serde_json::from_value(serde_json::Value::String(c.conflict_type))
                .unwrap_or(ConflictType::ContradictoryTasks),
            message_a: String::new(),
            message_b: String::new(),
            task_id: String::new(),
            explanation: c.explanation,
            resolution: ConflictResolution::Unresolved,
            resolved_by: String::new(),
            created_at: now,
            resolved_at: None,
        });
    }
    out
}

fn extract_json(s: &str) -> String {
    let start = s.find('{');
    let end = s.rfind('}');
    match (start, end) {
        (Some(a), Some(b)) if b >= a => s[a..=b].to_string(),
        _ => "{}".to_string(),
    }
}
