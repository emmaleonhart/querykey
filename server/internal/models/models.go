package models

import (
	"time"

	"github.com/google/uuid"
)

// --- Core Entities ---

// Account represents a user login on Secretarybird.
// Initial signup via Discord OAuth. Future: any platform the bot contacts you on.
type Account struct {
	ID             string    `json:"id"`               // human-readable slug, e.g. "john-smith"
	PersonID       string    `json:"person_id"`         // links to Person
	AuthProvider   string    `json:"auth_provider"`     // "discord" | "whatsapp" | "instagram" | "slack" | "email"
	AuthProviderID string    `json:"auth_provider_id"`  // ID from the auth provider
	CreatedAt      time.Time `json:"created_at"`
	LastLogin      time.Time `json:"last_login"`
}

// Handle represents a person's identity on a specific platform.
type Handle struct {
	Platform   string `json:"platform"`   // "discord" | "slack" | "whatsapp" | "instagram" | "phone"
	Identifier string `json:"identifier"` // username, phone number, etc.
}

// Person represents a team member tracked by the system.
// Cross-platform identity: same person across Discord, Slack, WhatsApp, voice, etc.
type Person struct {
	ID             string   `json:"id"`              // human-readable, e.g. "person:john-smith"
	DisplayName    string   `json:"display_name"`
	Handles        []Handle `json:"handles"`
	Role           string   `json:"role,omitempty"`
	ContactCascade []string `json:"contact_cascade"` // ordered: ["app", "discord", "whatsapp", "instagram", "slack", "sms"]
	CreatedAt      time.Time `json:"created_at"`
}

// TaskStatus represents the lifecycle of a task.
type TaskStatus string

const (
	TaskExtracted  TaskStatus = "extracted"
	TaskConfirmed  TaskStatus = "confirmed"
	TaskInProgress TaskStatus = "in_progress"
	TaskDone       TaskStatus = "done"
	TaskDisputed   TaskStatus = "disputed"
)

// Task is an actionable work item extracted from conversation.
// Tasks are time-flexible (optional deadline). If you can move it without asking, it's a task.
type Task struct {
	ID              uuid.UUID  `json:"id"`
	Title           string     `json:"title"`
	Description     string     `json:"description"`
	Status          TaskStatus `json:"status"`
	AssignedTo      string     `json:"assigned_to,omitempty"`      // Person.ID
	AssignedBy      string     `json:"assigned_by,omitempty"`      // Person.ID
	Deadline        *time.Time `json:"deadline,omitempty"`
	Confidence      float64    `json:"confidence"`       // AI confidence this is a real task
	AmbiguityScore  float64    `json:"ambiguity_score"`  // how vague the instruction was
	SourceMessages  []string   `json:"source_messages"`  // Message IDs
	CreatedAt       time.Time  `json:"created_at"`
	UpdatedAt       time.Time  `json:"updated_at"`
}

// Event is a time-fixed work item (meeting at 3 PM, deadline at midnight).
// If you can't move it without asking permission, it's an event.
type Event struct {
	ID             uuid.UUID  `json:"id"`
	Title          string     `json:"title"`
	Description    string     `json:"description"`
	StartTime      time.Time  `json:"start_time"`
	EndTime        time.Time  `json:"end_time"`
	Participants   []string   `json:"participants"`     // Person.IDs
	Confidence     float64    `json:"confidence"`
	SourceMessages []string   `json:"source_messages"`
	CreatedAt      time.Time  `json:"created_at"`
}

// InputType classifies how raw content arrived.
type InputType string

const (
	InputBotFeed       InputType = "bot_feed"
	InputChatlogPaste  InputType = "chatlog_paste"
	InputScreenshot    InputType = "screenshot"
	InputVoiceNote     InputType = "voice_note"
	InputRecordedAudio InputType = "recorded_audio"
	InputFreeformText  InputType = "freeform_text"
)

// IngestItem is a raw input submitted to the system.
type IngestItem struct {
	ID            uuid.UUID `json:"id"`
	InputType     InputType `json:"input_type"`
	RawContent    []byte    `json:"raw_content"`
	SubmittedBy   string    `json:"submitted_by"`           // Account.ID
	SubmittedAt   time.Time `json:"submitted_at"`
	SourceContext string    `json:"source_context,omitempty"` // e.g. "Monday standup"
}

// Message is a normalized record of something someone said.
// Extracted from IngestItems. A single chatlog may produce many Messages.
type Message struct {
	ID           uuid.UUID `json:"id"`
	SourceIngest string    `json:"source_ingest"` // IngestItem.ID
	Author       string    `json:"author"`        // Person.ID
	Content      string    `json:"content"`
	Timestamp    *time.Time `json:"timestamp,omitempty"`
	Confidence   float64   `json:"confidence"`
	RawMetadata  map[string]any `json:"raw_metadata,omitempty"`
}

// ConflictType classifies the kind of contradiction.
type ConflictType string

const (
	ConflictContradictoryTasks ConflictType = "contradictory_tasks"
	ConflictReassignment       ConflictType = "reassignment"
	ConflictDeadlineChange     ConflictType = "deadline_change"
	ConflictScopeChange        ConflictType = "scope_change"
)

// ConflictResolution describes how a conflict was resolved.
type ConflictResolution string

const (
	ResolutionUnresolved ConflictResolution = "unresolved"
	ResolutionAWins      ConflictResolution = "a_wins"
	ResolutionBWins      ConflictResolution = "b_wins"
	ResolutionMerged     ConflictResolution = "merged"
	ResolutionDismissed  ConflictResolution = "dismissed"
)

// Conflict represents contradictory instructions detected by the AI.
type Conflict struct {
	ID          uuid.UUID          `json:"id"`
	Type        ConflictType       `json:"type"`
	MessageA    string             `json:"message_a"`    // Message.ID
	MessageB    string             `json:"message_b"`    // Message.ID
	TaskID      string             `json:"task_id,omitempty"`
	Explanation string             `json:"explanation"`  // AI-generated
	Resolution  ConflictResolution `json:"resolution"`
	ResolvedBy  string             `json:"resolved_by,omitempty"`
	CreatedAt   time.Time          `json:"created_at"`
	ResolvedAt  *time.Time         `json:"resolved_at,omitempty"`
}

// Instruction is a directive or statement of intent, broader than Task.
type Instruction struct {
	ID            uuid.UUID `json:"id"`
	Content       string    `json:"content"`
	Speaker       string    `json:"speaker"`       // Person.ID
	Audience      []string  `json:"audience"`      // Person.IDs
	IsTask        bool      `json:"is_task"`
	TaskID        string    `json:"task_id,omitempty"`
	SourceMessage string    `json:"source_message"` // Message.ID
	CreatedAt     time.Time `json:"created_at"`
}

// Urgency describes how quickly an open question needs answering.
type Urgency string

const (
	UrgencyASAP     Urgency = "asap"
	UrgencyByTime   Urgency = "by_time"
	UrgencyEndOfDay Urgency = "end_of_day"
	UrgencyWhenever Urgency = "whenever"
)

// TriggerType describes what generated an open question or follow-up.
type TriggerType string

const (
	TriggerConflict        TriggerType = "conflict"
	TriggerAmbiguity       TriggerType = "ambiguity"
	TriggerMissedDeadline  TriggerType = "missed_deadline"
	TriggerUnconfirmedTask TriggerType = "unconfirmed_task"
	TriggerReassignment    TriggerType = "reassignment"
	TriggerScopeChange     TriggerType = "scope_change"
	TriggerDailyCheckin    TriggerType = "daily_checkin"
)

// QuestionStatus is the state of an open question.
type QuestionStatus string

const (
	QuestionOpen     QuestionStatus = "open"
	QuestionResolved QuestionStatus = "resolved"
	QuestionExpired  QuestionStatus = "expired"
)

// OpenQuestion is a question the system needs resolved, assigned to a person.
// Primary unit of interaction: bot generates these, delivers via DMs, people resolve them.
type OpenQuestion struct {
	ID              string         `json:"id"` // human-readable, e.g. "q:john-video-deadline"
	Target          string         `json:"target"`           // Person.ID
	Question        string         `json:"question"`         // short, clear question
	Context         string         `json:"context"`          // brief background
	Urgency         Urgency        `json:"urgency"`
	UrgencyDeadline *time.Time     `json:"urgency_deadline,omitempty"`
	TriggerType     TriggerType    `json:"trigger_type"`
	TriggerID       string         `json:"trigger_id"`
	Status          QuestionStatus `json:"status"`
	Resolution      string         `json:"resolution,omitempty"`
	ResolvedBy      string         `json:"resolved_by,omitempty"`
	ResolvedVia     string         `json:"resolved_via,omitempty"` // "app", "discord", etc.
	CreatedAt       time.Time      `json:"created_at"`
	ResolvedAt      *time.Time     `json:"resolved_at,omitempty"`
}

// DeliveryStatus tracks each attempt to reach someone.
type DeliveryStatus string

const (
	DeliverySent      DeliveryStatus = "sent"
	DeliveryDelivered DeliveryStatus = "delivered"
	DeliveryRead      DeliveryStatus = "read"
	DeliveryFailed    DeliveryStatus = "failed"
)

// DeliveryAttempt is a single attempt to reach someone on a specific channel.
type DeliveryAttempt struct {
	Channel string         `json:"channel"` // "app_dm", "discord_dm", "whatsapp", etc.
	Status  DeliveryStatus `json:"status"`
	SentAt  time.Time      `json:"sent_at"`
}

// FollowUpStatus is the state of an outbound follow-up.
type FollowUpStatus string

const (
	FollowUpPending  FollowUpStatus = "pending"
	FollowUpSent     FollowUpStatus = "sent"
	FollowUpAnswered FollowUpStatus = "answered"
	FollowUpExpired  FollowUpStatus = "expired"
)

// FollowUp is an outbound delivery attempt for an OpenQuestion.
type FollowUp struct {
	ID               string            `json:"id"` // human-readable
	TriggerType      TriggerType       `json:"trigger_type"`
	TriggerID        string            `json:"trigger_id"`
	Target           string            `json:"target"`   // Person.ID
	Question         string            `json:"question"` // short, clear question
	Context          string            `json:"context"`
	DeliveryAttempts []DeliveryAttempt  `json:"delivery_attempts"`
	Status           FollowUpStatus    `json:"status"`
	Response         string            `json:"response,omitempty"`
	ResponseChannel  string            `json:"response_channel,omitempty"`
	ResponseAt       *time.Time        `json:"response_at,omitempty"`
	CreatedAt        time.Time         `json:"created_at"`
}

// VoiceProfile is a learned voice embedding for speaker identification.
type VoiceProfile struct {
	ID          uuid.UUID `json:"id"`
	PersonID    string    `json:"person_id"`
	Embedding   []byte    `json:"embedding"`    // speaker embedding vector
	SampleCount int       `json:"sample_count"`
	Confidence  float64   `json:"confidence"`
	LastUpdated time.Time `json:"last_updated"`
	CreatedAt   time.Time `json:"created_at"`
}

// ExternalSync tracks tasks pushed to external project management tools.
type ExternalSync struct {
	ID               uuid.UUID `json:"id"`
	TaskID           string    `json:"task_id"`
	Platform         string    `json:"platform"` // "jira" | "azure_devops" | "github" | "gitlab" | "trello" | "asana"
	ExternalID       string    `json:"external_id"`
	ExternalURL      string    `json:"external_url"`
	SyncDirection    string    `json:"sync_direction"` // "push" | "bidirectional"
	LastSyncedAt     time.Time `json:"last_synced_at"`
	StatusInExternal string    `json:"status_in_external"`
}

// --- Graph Diff (for WebSocket broadcast) ---

// GraphDiff represents changes to the knowledge graph, broadcast to clients.
type GraphDiff struct {
	AddedNodes        []any      `json:"added_nodes,omitempty"`
	UpdatedNodes      []any      `json:"updated_nodes,omitempty"`
	AddedEdges        []Edge     `json:"added_edges,omitempty"`
	RemovedEdges      []Edge     `json:"removed_edges,omitempty"`
	NewConflicts      []Conflict `json:"new_conflicts,omitempty"`
	ResolvedConflicts []string   `json:"resolved_conflicts,omitempty"`
}

// Edge represents a relationship in the knowledge graph.
type Edge struct {
	Subject   string `json:"subject"`
	Predicate string `json:"predicate"`
	Object    string `json:"object"`
}

// --- WebSocket Protocol Messages ---

// WSMessage is the envelope for all WebSocket communication.
type WSMessage struct {
	Type    string `json:"type"`              // "stream_start", "stream_chunk", "stream_end", "status", "error", "graph_diff", "chat"
	Content string `json:"content,omitempty"`
	Data    any    `json:"data,omitempty"`
}

// ChatRequest is what a client sends over WebSocket.
type ChatRequest struct {
	Content string            `json:"content"`
	Message string            `json:"message"` // alias for content
	Context map[string]string `json:"context,omitempty"`
	History []ChatHistoryItem `json:"history,omitempty"`
}

// ChatHistoryItem is a previous message in the conversation.
type ChatHistoryItem struct {
	Role    string `json:"role"`    // "user" | "assistant"
	Content string `json:"content"`
}

// --- OpenClaw Analysis Results ---

// AnalysisResult is what OpenClaw returns after processing input.
type AnalysisResult struct {
	Tasks        []Task        `json:"tasks,omitempty"`
	Events       []Event       `json:"events,omitempty"`
	Instructions []Instruction `json:"instructions,omitempty"`
	Conflicts    []Conflict    `json:"conflicts,omitempty"`
	Messages     []Message     `json:"messages,omitempty"`
	FollowUps    []FollowUp    `json:"follow_ups,omitempty"`
}
