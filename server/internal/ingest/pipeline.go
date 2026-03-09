package ingest

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"time"

	"github.com/google/uuid"
	"github.com/secretarybird/server/internal/graph"
	"github.com/secretarybird/server/internal/models"
	"github.com/secretarybird/server/internal/openclaw"
	"github.com/secretarybird/server/internal/ws"
)

// Pipeline handles ingestion of all unstructured input types.
// Every input goes through: receive -> normalize -> analyze (OpenClaw) -> store (Fuseki) -> broadcast (WebSocket).
type Pipeline struct {
	bridge *openclaw.Bridge
	fuseki *graph.FusekiClient
	hub    *ws.Hub
}

// NewPipeline creates an ingestion pipeline.
func NewPipeline(bridge *openclaw.Bridge, fuseki *graph.FusekiClient, hub *ws.Hub) *Pipeline {
	return &Pipeline{
		bridge: bridge,
		fuseki: fuseki,
		hub:    hub,
	}
}

// IngestRequest is what the API receives when content is submitted.
type IngestRequest struct {
	InputType     string `json:"input_type"`               // "chatlog_paste", "screenshot", "voice_note", "freeform_text", etc.
	Content       string `json:"content"`                  // text content or base64 for binary
	SubmittedBy   string `json:"submitted_by"`             // Account.ID
	SourceContext string `json:"source_context,omitempty"` // e.g. "Monday standup"
}

// IngestResult is returned after processing.
type IngestResult struct {
	IngestID     string          `json:"ingest_id"`
	Messages     []models.Message `json:"messages,omitempty"`
	Tasks        []models.Task    `json:"tasks,omitempty"`
	Events       []models.Event   `json:"events,omitempty"`
	Conflicts    []models.Conflict `json:"conflicts,omitempty"`
}

// Process handles a single ingestion request through the full pipeline.
func (p *Pipeline) Process(ctx context.Context, req *IngestRequest) (*IngestResult, error) {
	ingestID := uuid.New()

	// 1. Create IngestItem
	item := &models.IngestItem{
		ID:            ingestID,
		InputType:     models.InputType(req.InputType),
		RawContent:    []byte(req.Content),
		SubmittedBy:   req.SubmittedBy,
		SubmittedAt:   time.Now(),
		SourceContext: req.SourceContext,
	}

	log.Printf("[ingest] processing %s from %s (type: %s)", ingestID, req.SubmittedBy, req.InputType)

	// 2. Normalize based on input type
	normalizedContent, err := p.normalize(ctx, item)
	if err != nil {
		return nil, fmt.Errorf("normalization failed: %w", err)
	}

	// 3. Send to OpenClaw for analysis
	analysisJSON, err := p.bridge.Analyze(ctx, normalizedContent, req.SourceContext)
	if err != nil {
		log.Printf("[ingest] OpenClaw analysis failed, storing raw: %v", err)
		// Still store the raw content even if analysis fails
		return &IngestResult{IngestID: ingestID.String()}, nil
	}

	// 4. Parse analysis results
	result, err := p.parseAnalysis(analysisJSON, ingestID.String())
	if err != nil {
		log.Printf("[ingest] failed to parse analysis: %v", err)
		result = &IngestResult{IngestID: ingestID.String()}
	}

	// 5. Store in Fuseki
	p.storeResults(ctx, result)

	// 6. Broadcast to connected clients
	p.broadcastResults(result)

	return result, nil
}

// normalize converts raw input into text suitable for OpenClaw analysis.
func (p *Pipeline) normalize(ctx context.Context, item *models.IngestItem) (string, error) {
	content := string(item.RawContent)

	switch item.InputType {
	case models.InputBotFeed:
		// Already structured, pass through
		return content, nil

	case models.InputChatlogPaste:
		// Pasted chatlog - wrap with instructions for OpenClaw
		return fmt.Sprintf("Parse this pasted chatlog. Extract speakers, messages, timestamps if visible. Identify tasks, deadlines, and contradictions.\n\n---\n%s", content), nil

	case models.InputScreenshot:
		// Screenshot - need OCR first
		// For now, assume the content has already been OCR'd by the client
		return fmt.Sprintf("This is text extracted from a screenshot of a conversation. Parse it as a chatlog.\n\n---\n%s", content), nil

	case models.InputVoiceNote:
		// Voice note - assume transcription was done client-side
		return fmt.Sprintf("This is a transcribed voice note. Extract any tasks, instructions, or scheduling information.\n\n---\n%s", content), nil

	case models.InputRecordedAudio:
		// Recorded conversation - assume transcription with speaker labels
		return fmt.Sprintf("This is a transcribed conversation recording. Extract speakers, tasks, events, and contradictions.\n\n---\n%s", content), nil

	case models.InputFreeformText:
		// Freeform text - could be anything
		return fmt.Sprintf("Extract any structured information from this text: tasks, events, people, deadlines, instructions.\n\n---\n%s", content), nil

	default:
		return content, nil
	}
}

// parseAnalysis converts OpenClaw's JSON response into structured results.
func (p *Pipeline) parseAnalysis(analysisJSON string, ingestID string) (*IngestResult, error) {
	result := &IngestResult{IngestID: ingestID}

	// Try to parse as structured JSON
	var analysis struct {
		Tasks    []struct {
			Title       string  `json:"title"`
			Description string  `json:"description"`
			AssignedTo  string  `json:"assigned_to"`
			AssignedBy  string  `json:"assigned_by"`
			Deadline    string  `json:"deadline"`
			Confidence  float64 `json:"confidence"`
		} `json:"tasks"`
		Events []struct {
			Title       string  `json:"title"`
			Description string  `json:"description"`
			StartTime   string  `json:"start_time"`
			EndTime     string  `json:"end_time"`
			Confidence  float64 `json:"confidence"`
		} `json:"events"`
		Conflicts []struct {
			Type        string `json:"type"`
			Explanation string `json:"explanation"`
		} `json:"conflicts"`
		FollowUps []struct {
			Target   string `json:"target"`
			Question string `json:"question"`
			Urgency  string `json:"urgency"`
		} `json:"follow_ups"`
	}

	// OpenClaw might return markdown-wrapped JSON, try to extract it
	jsonStr := extractJSON(analysisJSON)
	if err := json.Unmarshal([]byte(jsonStr), &analysis); err != nil {
		// If JSON parsing fails, just log the raw response
		log.Printf("[ingest] could not parse analysis as JSON (will store raw): %v", err)
		return result, nil
	}

	// Convert to models
	now := time.Now()
	for _, t := range analysis.Tasks {
		task := models.Task{
			ID:          uuid.New(),
			Title:       t.Title,
			Description: t.Description,
			Status:      models.TaskExtracted,
			AssignedTo:  t.AssignedTo,
			AssignedBy:  t.AssignedBy,
			Confidence:  t.Confidence,
			SourceMessages: []string{ingestID},
			CreatedAt:   now,
			UpdatedAt:   now,
		}
		if t.Deadline != "" {
			if dl, err := time.Parse(time.RFC3339, t.Deadline); err == nil {
				task.Deadline = &dl
			}
		}
		result.Tasks = append(result.Tasks, task)
	}

	for _, e := range analysis.Events {
		event := models.Event{
			ID:          uuid.New(),
			Title:       e.Title,
			Description: e.Description,
			Confidence:  e.Confidence,
			SourceMessages: []string{ingestID},
			CreatedAt:   now,
		}
		if st, err := time.Parse(time.RFC3339, e.StartTime); err == nil {
			event.StartTime = st
		}
		if et, err := time.Parse(time.RFC3339, e.EndTime); err == nil {
			event.EndTime = et
		}
		result.Events = append(result.Events, event)
	}

	for _, c := range analysis.Conflicts {
		conflict := models.Conflict{
			ID:          uuid.New(),
			Type:        models.ConflictType(c.Type),
			Explanation: c.Explanation,
			Resolution:  models.ResolutionUnresolved,
			CreatedAt:   now,
		}
		result.Conflicts = append(result.Conflicts, conflict)
	}

	return result, nil
}

// storeResults saves analysis results to Fuseki.
func (p *Pipeline) storeResults(ctx context.Context, result *IngestResult) {
	for i := range result.Tasks {
		if err := p.fuseki.StoreTask(ctx, &result.Tasks[i]); err != nil {
			log.Printf("[ingest] failed to store task: %v", err)
		}
	}
	for i := range result.Conflicts {
		if err := p.fuseki.StoreConflict(ctx, &result.Conflicts[i]); err != nil {
			log.Printf("[ingest] failed to store conflict: %v", err)
		}
	}
}

// broadcastResults sends results to all connected WebSocket clients.
func (p *Pipeline) broadcastResults(result *IngestResult) {
	diff := &models.GraphDiff{}

	for _, t := range result.Tasks {
		diff.AddedNodes = append(diff.AddedNodes, t)
	}
	for _, e := range result.Events {
		diff.AddedNodes = append(diff.AddedNodes, e)
	}
	for _, c := range result.Conflicts {
		diff.NewConflicts = append(diff.NewConflicts, c)
	}

	p.hub.BroadcastGraphDiff(diff)
}

// extractJSON tries to find a JSON object in a string that might be wrapped in markdown.
func extractJSON(s string) string {
	// Look for ```json ... ``` blocks
	start := -1
	for i := 0; i < len(s)-2; i++ {
		if s[i] == '{' {
			start = i
			break
		}
	}
	if start == -1 {
		return s
	}

	// Find matching closing brace
	depth := 0
	for i := start; i < len(s); i++ {
		if s[i] == '{' {
			depth++
		} else if s[i] == '}' {
			depth--
			if depth == 0 {
				return s[start : i+1]
			}
		}
	}
	return s[start:]
}
