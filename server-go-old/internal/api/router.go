package api

import (
	"net/http"

	"github.com/secretarybird/server/internal/discord"
	"github.com/secretarybird/server/internal/graph"
	"github.com/secretarybird/server/internal/ingest"
	"github.com/secretarybird/server/internal/openclaw"
	"github.com/secretarybird/server/internal/ws"
)

// Server holds all dependencies for the HTTP API.
type Server struct {
	bridge   *openclaw.Bridge
	fuseki   *graph.FusekiClient
	hub      *ws.Hub
	bot      *discord.Bot
	pipeline *ingest.Pipeline
}

// NewServer creates an API server with all dependencies.
func NewServer(bridge *openclaw.Bridge, fuseki *graph.FusekiClient, hub *ws.Hub, bot *discord.Bot, pipeline *ingest.Pipeline) *Server {
	return &Server{
		bridge:   bridge,
		fuseki:   fuseki,
		hub:      hub,
		bot:      bot,
		pipeline: pipeline,
	}
}

// NewRouter creates the HTTP router with all endpoints.
func (s *Server) NewRouter() http.Handler {
	mux := http.NewServeMux()

	// Health & status
	mux.HandleFunc("GET /health", s.handleHealth)
	mux.HandleFunc("GET /api/openclaw/status", s.handleOpenClawStatus)
	mux.HandleFunc("GET /api/status", s.handleSystemStatus)

	// WebSocket - real-time sync (matches old protocol at /ws/chat)
	mux.HandleFunc("/ws/chat", s.hub.HandleWebSocket)

	// Ingestion
	mux.HandleFunc("POST /api/ingest", s.handleIngest)

	// Persons
	mux.HandleFunc("GET /api/persons", s.handleListPersons)
	mux.HandleFunc("POST /api/persons", s.handleCreatePerson)
	mux.HandleFunc("GET /api/persons/{id}/tasks", s.handlePersonTasks)

	// Tasks
	mux.HandleFunc("GET /api/tasks", s.handleListTasks)
	mux.HandleFunc("POST /api/tasks", s.handleCreateTask)
	mux.HandleFunc("PATCH /api/tasks/{id}", s.handleUpdateTask)

	// Conflicts
	mux.HandleFunc("GET /api/conflicts", s.handleListConflicts)
	mux.HandleFunc("POST /api/conflicts/{id}/resolve", s.handleResolveConflict)

	// Open questions
	mux.HandleFunc("GET /api/questions", s.handleListQuestions)
	mux.HandleFunc("POST /api/questions/{id}/resolve", s.handleResolveQuestion)

	// Follow-ups
	mux.HandleFunc("GET /api/followups", s.handleListFollowUps)
	mux.HandleFunc("POST /api/followups", s.handleCreateFollowUp)

	// OpenClaw management
	mux.HandleFunc("POST /api/openclaw/kill", s.handleOpenClawKill)
	mux.HandleFunc("POST /api/openclaw/restart", s.handleOpenClawRestart)

	// Graph queries (SPARQL passthrough)
	mux.HandleFunc("POST /api/graph/query", s.handleGraphQuery)

	// Wrap with CORS middleware
	return corsMiddleware(mux)
}

// corsMiddleware adds CORS headers for Flutter app and other clients.
func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusOK)
			return
		}

		next.ServeHTTP(w, r)
	})
}
