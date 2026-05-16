package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/secretarybird/server/internal/api"
	"github.com/secretarybird/server/internal/config"
	"github.com/secretarybird/server/internal/discord"
	"github.com/secretarybird/server/internal/graph"
	"github.com/secretarybird/server/internal/ingest"
	"github.com/secretarybird/server/internal/openclaw"
	"github.com/secretarybird/server/internal/ws"
)

func main() {
	log.SetFlags(log.Ldate | log.Ltime | log.Lshortfile)
	log.Println("[secretarybird] starting up...")

	// Load configuration
	cfg := config.Load()

	// Initialize OpenClaw bridge (connects to WSL gateway)
	bridge := openclaw.NewBridge(cfg.OpenClawGatewayURL, cfg.OpenClawAgentID, cfg.OpenClawToken)

	// Detect OpenClaw gateway
	status := bridge.Detect()
	if status.Available {
		log.Printf("[secretarybird] OpenClaw gateway connected: %s (agent: %s)", status.GatewayURL, status.AgentID)
	} else {
		log.Printf("[secretarybird] OpenClaw gateway not available: %s", status.Error)
		// Try to auto-start if WSL is available
		go bridge.EnsureGateway()
	}

	// Initialize Fuseki graph store
	fuseki := graph.NewFusekiClient(cfg.FusekiURL, cfg.FusekiDataset)
	ctx := context.Background()
	if err := fuseki.Ping(ctx); err != nil {
		log.Printf("[secretarybird] Fuseki not reachable: %v (continuing without graph store)", err)
	} else {
		log.Printf("[secretarybird] Fuseki connected: %s/%s", cfg.FusekiURL, cfg.FusekiDataset)
		if err := fuseki.EnsureDataset(ctx); err != nil {
			log.Printf("[secretarybird] failed to ensure dataset: %v", err)
		}
	}

	// Initialize WebSocket hub
	hub := ws.NewHub(bridge)
	go hub.Run()

	// Initialize ingestion pipeline
	pipeline := ingest.NewPipeline(bridge, fuseki, hub)

	// Initialize Discord bot (optional - only if token is set)
	var bot *discord.Bot
	if cfg.DiscordToken != "" {
		var err error
		bot, err = discord.NewBot(cfg.DiscordToken, cfg.DiscordGuildIDs, cfg.DiscordBatchInterval, bridge, fuseki, hub)
		if err != nil {
			log.Printf("[secretarybird] Discord bot init failed: %v (continuing without Discord)", err)
		} else {
			if err := bot.Start(); err != nil {
				log.Printf("[secretarybird] Discord bot start failed: %v", err)
				bot = nil
			} else {
				log.Printf("[secretarybird] Discord bot started")
			}
		}
	} else {
		log.Printf("[secretarybird] DISCORD_TOKEN not set, bot disabled")
	}

	// Create API server
	srv := api.NewServer(bridge, fuseki, hub, bot, pipeline)
	router := srv.NewRouter()

	// Start HTTP server
	addr := fmt.Sprintf("%s:%d", cfg.Host, cfg.Port)
	httpServer := &http.Server{
		Addr:         addr,
		Handler:      router,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 120 * time.Second, // Long for streaming responses
		IdleTimeout:  60 * time.Second,
	}

	// Graceful shutdown
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt, syscall.SIGTERM)

	go func() {
		log.Printf("[secretarybird] server listening on %s", addr)
		if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("[secretarybird] server error: %v", err)
		}
	}()

	// Wait for shutdown signal
	<-stop
	log.Println("[secretarybird] shutting down...")

	// Graceful shutdown with 10s timeout
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if bot != nil {
		bot.Stop()
	}
	bridge.StopGateway()
	httpServer.Shutdown(shutdownCtx)

	log.Println("[secretarybird] shutdown complete")
}
