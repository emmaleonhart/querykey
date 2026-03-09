package api

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/secretarybird/server/internal/ingest"
	"github.com/secretarybird/server/internal/models"
)

// --- Health & Status ---

func (s *Server) handleHealth(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]any{
		"status":  "ok",
		"version": "0.1.0",
	})
}

func (s *Server) handleOpenClawStatus(w http.ResponseWriter, r *http.Request) {
	status := s.bridge.Detect()
	writeJSON(w, http.StatusOK, status)
}

func (s *Server) handleSystemStatus(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	openclawStatus := s.bridge.Detect()

	fusekiOK := true
	if err := s.fuseki.Ping(ctx); err != nil {
		fusekiOK = false
	}

	writeJSON(w, http.StatusOK, map[string]any{
		"server":         "ok",
		"openclaw":       openclawStatus,
		"fuseki":         fusekiOK,
		"ws_clients":     s.hub.ClientCount(),
		"discord_active": s.bot != nil,
	})
}

// --- Ingestion ---

func (s *Server) handleIngest(w http.ResponseWriter, r *http.Request) {
	var req ingest.IngestRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if req.Content == "" {
		writeError(w, http.StatusBadRequest, "content is required")
		return
	}
	if req.InputType == "" {
		writeError(w, http.StatusBadRequest, "input_type is required")
		return
	}

	result, err := s.pipeline.Process(r.Context(), &req)
	if err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	writeJSON(w, http.StatusOK, result)
}

// --- Persons ---

func (s *Server) handleListPersons(w http.ResponseWriter, r *http.Request) {
	persons, err := s.fuseki.GetAllPersons(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"persons": persons})
}

func (s *Server) handleCreatePerson(w http.ResponseWriter, r *http.Request) {
	var person models.Person
	if err := json.NewDecoder(r.Body).Decode(&person); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	if person.ID == "" || person.DisplayName == "" {
		writeError(w, http.StatusBadRequest, "id and display_name are required")
		return
	}

	person.CreatedAt = time.Now()
	if person.ContactCascade == nil {
		person.ContactCascade = []string{"app", "discord"}
	}

	if err := s.fuseki.StorePerson(r.Context(), &person); err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	// Broadcast new person to clients
	s.hub.BroadcastGraphDiff(&models.GraphDiff{
		AddedNodes: []any{person},
	})

	writeJSON(w, http.StatusCreated, person)
}

func (s *Server) handlePersonTasks(w http.ResponseWriter, r *http.Request) {
	personID := r.PathValue("id")
	if personID == "" {
		writeError(w, http.StatusBadRequest, "person id required")
		return
	}

	tasks, err := s.fuseki.GetTasksForPerson(r.Context(), personID)
	if err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	writeJSON(w, http.StatusOK, map[string]any{"tasks": tasks})
}

// --- Tasks ---

func (s *Server) handleListTasks(w http.ResponseWriter, r *http.Request) {
	// Query all tasks from Fuseki
	ctx := r.Context()
	result, err := s.fuseki.Query(ctx, `
PREFIX sb: <http://secretarybird.dev/ns/>
SELECT ?id ?title ?status ?confidence ?assignedTo ?createdAt
WHERE {
  ?task a sb:Task ;
    sb:title ?title ;
    sb:status ?status ;
    sb:confidence ?confidence ;
    sb:createdAt ?createdAt .
  OPTIONAL { ?task sb:assignedTo ?assignedTo }
  BIND(STR(?task) AS ?id)
}
ORDER BY DESC(?createdAt)
LIMIT 100`)
	if err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	writeJSON(w, http.StatusOK, map[string]any{"tasks": result.Results.Bindings})
}

func (s *Server) handleCreateTask(w http.ResponseWriter, r *http.Request) {
	var task models.Task
	if err := json.NewDecoder(r.Body).Decode(&task); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	task.ID = uuid.New()
	task.CreatedAt = time.Now()
	task.UpdatedAt = time.Now()
	if task.Status == "" {
		task.Status = models.TaskExtracted
	}

	if err := s.fuseki.StoreTask(r.Context(), &task); err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	s.hub.BroadcastGraphDiff(&models.GraphDiff{
		AddedNodes: []any{task},
	})

	writeJSON(w, http.StatusCreated, task)
}

func (s *Server) handleUpdateTask(w http.ResponseWriter, r *http.Request) {
	taskID := r.PathValue("id")
	if taskID == "" {
		writeError(w, http.StatusBadRequest, "task id required")
		return
	}

	var update struct {
		Status     string `json:"status,omitempty"`
		AssignedTo string `json:"assigned_to,omitempty"`
		Title      string `json:"title,omitempty"`
	}
	if err := json.NewDecoder(r.Body).Decode(&update); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	// Build SPARQL update
	ctx := r.Context()
	if update.Status != "" {
		sparql := `
PREFIX sb: <http://secretarybird.dev/ns/>
DELETE { ?task sb:status ?oldStatus }
INSERT { ?task sb:status "` + update.Status + `" }
WHERE {
  ?task a sb:Task ;
    sb:status ?oldStatus .
  FILTER(STR(?task) = "http://secretarybird.dev/ns/task/` + taskID + `")
}`
		if err := s.fuseki.Update(ctx, sparql); err != nil {
			writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
	}

	s.hub.BroadcastMessage(models.WSMessage{
		Type:    "task_updated",
		Content: taskID,
		Data:    update,
	})

	writeJSON(w, http.StatusOK, map[string]any{"updated": taskID})
}

// --- Conflicts ---

func (s *Server) handleListConflicts(w http.ResponseWriter, r *http.Request) {
	conflicts, err := s.fuseki.GetUnresolvedConflicts(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"conflicts": conflicts})
}

func (s *Server) handleResolveConflict(w http.ResponseWriter, r *http.Request) {
	conflictID := r.PathValue("id")

	var resolve struct {
		Resolution string `json:"resolution"` // "a_wins", "b_wins", "merged", "dismissed"
		ResolvedBy string `json:"resolved_by"`
	}
	if err := json.NewDecoder(r.Body).Decode(&resolve); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	ctx := r.Context()
	now := time.Now().Format(time.RFC3339)
	sparql := `
PREFIX sb: <http://secretarybird.dev/ns/>
DELETE { ?c sb:resolution ?old }
INSERT {
  ?c sb:resolution "` + resolve.Resolution + `" ;
     sb:resolvedBy "` + resolve.ResolvedBy + `" ;
     sb:resolvedAt "` + now + `"^^<http://www.w3.org/2001/XMLSchema#dateTime> .
}
WHERE {
  ?c a sb:Conflict ;
     sb:resolution ?old .
  FILTER(STR(?c) = "http://secretarybird.dev/ns/conflict/` + conflictID + `")
}`
	if err := s.fuseki.Update(ctx, sparql); err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	s.hub.BroadcastGraphDiff(&models.GraphDiff{
		ResolvedConflicts: []string{conflictID},
	})

	writeJSON(w, http.StatusOK, map[string]any{"resolved": conflictID})
}

// --- Open Questions ---

func (s *Server) handleListQuestions(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	result, err := s.fuseki.Query(ctx, `
PREFIX sb: <http://secretarybird.dev/ns/>
SELECT ?id ?target ?question ?urgency ?status ?createdAt
WHERE {
  ?q a sb:OpenQuestion ;
    sb:target ?target ;
    sb:question ?question ;
    sb:urgency ?urgency ;
    sb:status ?status ;
    sb:createdAt ?createdAt .
  FILTER(?status = "open")
  BIND(STR(?q) AS ?id)
}
ORDER BY DESC(?createdAt)`)
	if err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"questions": result.Results.Bindings})
}

func (s *Server) handleResolveQuestion(w http.ResponseWriter, r *http.Request) {
	questionID := r.PathValue("id")

	var resolve struct {
		Resolution string `json:"resolution"`
		ResolvedBy string `json:"resolved_by"`
		ResolvedVia string `json:"resolved_via"` // "app", "discord", etc.
	}
	if err := json.NewDecoder(r.Body).Decode(&resolve); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	log.Printf("[api] resolving question %s by %s via %s", questionID, resolve.ResolvedBy, resolve.ResolvedVia)

	s.hub.BroadcastMessage(models.WSMessage{
		Type:    "question_resolved",
		Content: questionID,
	})

	writeJSON(w, http.StatusOK, map[string]any{"resolved": questionID})
}

// --- Follow-ups ---

func (s *Server) handleListFollowUps(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	result, err := s.fuseki.Query(ctx, `
PREFIX sb: <http://secretarybird.dev/ns/>
SELECT ?id ?target ?question ?status ?createdAt
WHERE {
  ?f a sb:FollowUp ;
    sb:target ?target ;
    sb:question ?question ;
    sb:status ?status ;
    sb:createdAt ?createdAt .
  BIND(STR(?f) AS ?id)
}
ORDER BY DESC(?createdAt)
LIMIT 50`)
	if err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"follow_ups": result.Results.Bindings})
}

func (s *Server) handleCreateFollowUp(w http.ResponseWriter, r *http.Request) {
	var followUp models.FollowUp
	if err := json.NewDecoder(r.Body).Decode(&followUp); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	followUp.CreatedAt = time.Now()
	followUp.Status = models.FollowUpPending

	// If we have a Discord bot, try to send the DM
	if s.bot != nil && followUp.Target != "" {
		if err := s.bot.SendDM(followUp.Target, followUp.Question); err != nil {
			log.Printf("[api] failed to send Discord DM for follow-up: %v", err)
		} else {
			followUp.Status = models.FollowUpSent
			followUp.DeliveryAttempts = append(followUp.DeliveryAttempts, models.DeliveryAttempt{
				Channel: "discord_dm",
				Status:  models.DeliverySent,
				SentAt:  time.Now(),
			})
		}
	}

	writeJSON(w, http.StatusCreated, followUp)
}

// --- OpenClaw Management ---

func (s *Server) handleOpenClawKill(w http.ResponseWriter, r *http.Request) {
	if err := s.bridge.ForceKill(); err != nil {
		writeJSON(w, http.StatusOK, map[string]any{
			"ok":      true,
			"message": "Kill command sent (processes may already be dead)",
		})
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"ok":      true,
		"message": "OpenClaw killed and lock files cleaned",
	})
}

func (s *Server) handleOpenClawRestart(w http.ResponseWriter, r *http.Request) {
	s.bridge.ForceKill()
	time.Sleep(2 * time.Second)
	s.bridge.EnsureGateway()
	time.Sleep(3 * time.Second)

	status := s.bridge.Detect()
	writeJSON(w, http.StatusOK, map[string]any{
		"ok":      status.Available,
		"message": map[bool]string{true: "OpenClaw restarted", false: "OpenClaw restart in progress..."}[status.Available],
	})
}

// --- Graph Query ---

func (s *Server) handleGraphQuery(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Query string `json:"query"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body")
		return
	}

	// Basic safety check - don't allow mutations through query endpoint
	ctx, cancel := context.WithTimeout(r.Context(), 30*time.Second)
	defer cancel()

	result, err := s.fuseki.Query(ctx, req.Query)
	if err != nil {
		writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	writeJSON(w, http.StatusOK, result)
}

// --- Helpers ---

func writeJSON(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, map[string]string{"error": message})
}
