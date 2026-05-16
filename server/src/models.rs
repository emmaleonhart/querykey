//! Port of server-go-old/internal/models/models.go.
//! JSON field names preserved (snake_case) so the Flutter app and the
//! existing API contract keep working.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn skip_none<T>(o: &Option<T>) -> bool {
    o.is_none()
}

// --- Core Entities ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub person_id: String,
    pub auth_provider: String,
    pub auth_provider_id: String,
    pub created_at: DateTime<Utc>,
    pub last_login: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handle {
    pub platform: String,
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub handles: Vec<Handle>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,
    #[serde(default)]
    pub contact_cascade: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Extracted,
    Confirmed,
    InProgress,
    Done,
    Disputed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub assigned_to: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub assigned_by: String,
    #[serde(default, skip_serializing_if = "skip_none")]
    pub deadline: Option<DateTime<Utc>>,
    pub confidence: f64,
    pub ambiguity_score: f64,
    #[serde(default)]
    pub source_messages: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    #[serde(default)]
    pub participants: Vec<String>,
    /// Optional RFC-5545-subset recurrence rule, e.g.
    /// `FREQ=WEEKLY;INTERVAL=1;COUNT=10`. `None` = one-off event.
    #[serde(default, skip_serializing_if = "skip_none")]
    pub recurrence: Option<String>,
    pub confidence: f64,
    #[serde(default)]
    pub source_messages: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
    BotFeed,
    ChatlogPaste,
    Screenshot,
    VoiceNote,
    RecordedAudio,
    FreeformText,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestItem {
    pub id: Uuid,
    pub input_type: InputType,
    #[serde(default)]
    pub raw_content: Vec<u8>,
    pub submitted_by: String,
    pub submitted_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub source_ingest: String,
    pub author: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "skip_none")]
    pub timestamp: Option<DateTime<Utc>>,
    pub confidence: f64,
    #[serde(default, skip_serializing_if = "skip_none")]
    pub raw_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    ContradictoryTasks,
    Reassignment,
    DeadlineChange,
    ScopeChange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    Unresolved,
    AWins,
    BWins,
    Merged,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub conflict_type: ConflictType,
    pub message_a: String,
    pub message_b: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_id: String,
    pub explanation: String,
    pub resolution: ConflictResolution,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resolved_by: String,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "skip_none")]
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub id: Uuid,
    pub content: String,
    pub speaker: String,
    #[serde(default)]
    pub audience: Vec<String>,
    pub is_task: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub task_id: String,
    pub source_message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Asap,
    ByTime,
    EndOfDay,
    Whenever,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    Conflict,
    Ambiguity,
    MissedDeadline,
    UnconfirmedTask,
    Reassignment,
    ScopeChange,
    DailyCheckin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    Open,
    Resolved,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenQuestion {
    pub id: String,
    pub target: String,
    pub question: String,
    pub context: String,
    pub urgency: Urgency,
    #[serde(default, skip_serializing_if = "skip_none")]
    pub urgency_deadline: Option<DateTime<Utc>>,
    pub trigger_type: TriggerType,
    pub trigger_id: String,
    pub status: QuestionStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resolution: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resolved_by: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resolved_via: String,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "skip_none")]
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Sent,
    Delivered,
    Read,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryAttempt {
    pub channel: String,
    pub status: DeliveryStatus,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FollowUpStatus {
    Pending,
    Sent,
    Answered,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUp {
    pub id: String,
    pub trigger_type: TriggerType,
    pub trigger_id: String,
    pub target: String,
    pub question: String,
    pub context: String,
    #[serde(default)]
    pub delivery_attempts: Vec<DeliveryAttempt>,
    pub status: FollowUpStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub response: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub response_channel: String,
    #[serde(default, skip_serializing_if = "skip_none")]
    pub response_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub id: Uuid,
    pub person_id: String,
    #[serde(default)]
    pub embedding: Vec<u8>,
    pub sample_count: i64,
    pub confidence: f64,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalSync {
    pub id: Uuid,
    pub task_id: String,
    pub platform: String,
    pub external_id: String,
    pub external_url: String,
    pub sync_direction: String,
    pub last_synced_at: DateTime<Utc>,
    pub status_in_external: String,
}

// --- Graph Diff (for WebSocket broadcast) ---

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphDiff {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_nodes: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updated_nodes: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_edges: Vec<Edge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_edges: Vec<Edge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub new_conflicts: Vec<Conflict>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

// --- WebSocket Protocol Messages ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content: String,
    #[serde(default, skip_serializing_if = "skip_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatRequest {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub context: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub history: Vec<ChatHistoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistoryItem {
    pub role: String,
    pub content: String,
}

// --- Local AI agent analysis result ---

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisResult {
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub instructions: Vec<Instruction>,
    #[serde(default)]
    pub conflicts: Vec<Conflict>,
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default)]
    pub follow_ups: Vec<FollowUp>,
}
