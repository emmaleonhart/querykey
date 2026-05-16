//! Port of server-go-old/internal/ingest/pipeline.go.
//! Accept raw input -> normalize -> local-agent analyze -> parse ->
//! store in the (derived) graph -> broadcast a graph diff.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::graph::GraphStore;
use crate::models::{AnalysisResult, GraphDiff, InputType};
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

        let analysis = parse_analysis(&analysis_json);
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

    async fn store_results(&self, a: &AnalysisResult) {
        for m in &a.messages {
            let _ = self.graph.store_message(m).await;
        }
        for t in &a.tasks {
            let _ = self.graph.store_task(t).await;
        }
        for c in &a.conflicts {
            let _ = self.graph.store_conflict(c).await;
        }
    }

    fn broadcast_results(&self, a: &AnalysisResult) {
        let diff = GraphDiff {
            new_conflicts: a.conflicts.clone(),
            ..Default::default()
        };
        self.hub.broadcast_graph_diff(&diff);
    }
}

/// Mirrors pipeline.go parseAnalysis() + extractJSON(): the agent often
/// wraps JSON in prose / code fences; pull the JSON object out.
pub fn parse_analysis(s: &str) -> AnalysisResult {
    let json = extract_json(s);
    serde_json::from_str::<AnalysisResult>(&json).unwrap_or_default()
}

fn extract_json(s: &str) -> String {
    let start = s.find('{');
    let end = s.rfind('}');
    match (start, end) {
        (Some(a), Some(b)) if b >= a => s[a..=b].to_string(),
        _ => "{}".to_string(),
    }
}
