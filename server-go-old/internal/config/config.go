package config

import (
	"encoding/json"
	"log"
	"os"
	"os/exec"
	"strconv"
	"strings"
)

// Config holds all server configuration.
type Config struct {
	// Server
	Host string
	Port int

	// OpenClaw gateway (runs in WSL)
	OpenClawGatewayURL string
	OpenClawAgentID    string
	OpenClawToken      string

	// Apache Jena Fuseki
	FusekiURL     string
	FusekiDataset string

	// Discord bot
	DiscordToken         string
	DiscordGuildIDs      []string
	DiscordBatchInterval int // minutes between batch processing

	// General
	LogLevel string
}

// Load reads configuration from environment variables with sensible defaults.
func Load() *Config {
	cfg := &Config{
		Host:                 envOr("SB_HOST", "127.0.0.1"),
		Port:                 envInt("SB_PORT", 8000),
		OpenClawGatewayURL:   envOr("OPENCLAW_GATEWAY_URL", "http://127.0.0.1:18789"),
		OpenClawAgentID:      envOr("OPENCLAW_AGENT_ID", "main"),
		OpenClawToken:        os.Getenv("OPENCLAW_GATEWAY_TOKEN"),
		FusekiURL:            envOr("FUSEKI_URL", "http://127.0.0.1:3030"),
		FusekiDataset:        envOr("FUSEKI_DATASET", "secretarybird"),
		DiscordToken:         os.Getenv("DISCORD_TOKEN"),
		DiscordBatchInterval: envInt("DISCORD_BATCH_INTERVAL", 60),
		LogLevel:             envOr("SB_LOG_LEVEL", "info"),
	}

	if guilds := os.Getenv("DISCORD_GUILD_IDS"); guilds != "" {
		for _, g := range strings.Split(guilds, ",") {
			g = strings.TrimSpace(g)
			if g != "" {
				cfg.DiscordGuildIDs = append(cfg.DiscordGuildIDs, g)
			}
		}
	}

	if cfg.OpenClawToken == "" {
		cfg.OpenClawToken = readOpenClawTokenFromWSL()
	}

	return cfg
}

func envOr(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func envInt(key string, fallback int) int {
	if v := os.Getenv(key); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			return n
		}
	}
	return fallback
}

func readOpenClawTokenFromWSL() string {
	// On non-Windows, check the local filesystem directly
	if os.Getenv("OS") != "Windows_NT" {
		home, _ := os.UserHomeDir()
		return readTokenFromFile(home + "/.openclaw/openclaw.json")
	}

	// On Windows, read from WSL
	cmd := exec.Command("wsl", "-d", "Ubuntu", "--", "bash", "-lc", "cat ~/.openclaw/openclaw.json")
	out, err := cmd.Output()
	if err != nil {
		log.Printf("[config] could not read OpenClaw config from WSL: %v", err)
		return ""
	}
	return parseToken(out)
}

func readTokenFromFile(path string) string {
	data, err := os.ReadFile(path)
	if err != nil {
		return ""
	}
	return parseToken(data)
}

func parseToken(data []byte) string {
	var cfg struct {
		Gateway struct {
			Auth struct {
				Token string `json:"token"`
			} `json:"auth"`
		} `json:"gateway"`
	}
	if err := json.Unmarshal(data, &cfg); err != nil {
		return ""
	}
	return cfg.Gateway.Auth.Token
}
