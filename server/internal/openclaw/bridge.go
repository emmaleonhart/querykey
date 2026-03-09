package openclaw

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os/exec"
	"strings"
	"sync"
	"time"
)

const (
	systemPrompt = `You are Secretarybird, an AI secretary for team coordination.

You help with:
- Extracting tasks, events, and deadlines from conversations
- Detecting contradictions between instructions
- Following up with team members to clarify ambiguity
- Keeping everyone on the same page

Be concise, direct, and action-oriented. You are a secretary, not a consultant.
Short messages. Ask questions. Don't explain yourself.`
)

// Bridge connects to the OpenClaw Gateway via its OpenAI-compatible HTTP API.
// The gateway runs in WSL and exposes POST /v1/chat/completions.
// This is the same pattern as the old Python bridge but in Go.
type Bridge struct {
	gatewayURL string
	agentID    string
	authToken  string
	client     *http.Client
	wsl        *WSLManager

	// Gateway process management
	mu             sync.Mutex
	gatewayCmd     *exec.Cmd
	retries        int
	maxRetries     int
	retryDelay     time.Duration
	healthTicker   *time.Ticker
	stopHealth     chan struct{}
}

// NewBridge creates a new OpenClaw bridge.
func NewBridge(gatewayURL, agentID, authToken string) *Bridge {
	return &Bridge{
		gatewayURL: gatewayURL,
		agentID:    agentID,
		authToken:  authToken,
		client: &http.Client{
			Timeout: 120 * time.Second,
		},
		wsl:        NewWSLManager(),
		maxRetries: 5,
		retryDelay: 3 * time.Second,
		stopHealth: make(chan struct{}),
	}
}

// Status holds the result of a gateway availability check.
type Status struct {
	Available  bool   `json:"available"`
	GatewayURL string `json:"gateway_url"`
	AgentID    string `json:"agent_id"`
	HasAuth    bool   `json:"has_auth"`
	Error      string `json:"error,omitempty"`
}

// Detect checks whether the OpenClaw gateway is reachable.
// Only checks reachability (fast GET /), does NOT send a full chat request.
func (b *Bridge) Detect() Status {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	req, err := http.NewRequestWithContext(ctx, "GET", b.gatewayURL, nil)
	if err != nil {
		return Status{
			Available:  false,
			GatewayURL: b.gatewayURL,
			AgentID:    b.agentID,
			Error:      err.Error(),
		}
	}

	resp, err := b.client.Do(req)
	if err != nil {
		return Status{
			Available:  false,
			GatewayURL: b.gatewayURL,
			AgentID:    b.agentID,
			Error:      "Cannot connect to OpenClaw gateway. Start it in WSL with: openclaw gateway",
		}
	}
	defer resp.Body.Close()
	io.Copy(io.Discard, resp.Body)

	if resp.StatusCode != 200 {
		return Status{
			Available:  false,
			GatewayURL: b.gatewayURL,
			AgentID:    b.agentID,
			Error:      fmt.Sprintf("Gateway returned status %d", resp.StatusCode),
		}
	}

	return Status{
		Available:  true,
		GatewayURL: b.gatewayURL,
		AgentID:    b.agentID,
		HasAuth:    b.authToken != "",
	}
}

// ChatMessage is the OpenAI-compatible message format.
type ChatMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

// Chat sends a message to OpenClaw and returns the full response (non-streaming).
func (b *Bridge) Chat(ctx context.Context, message string, history []ChatMessage) (string, error) {
	messages := b.buildMessages(message, history)

	body, err := json.Marshal(map[string]any{
		"model":    fmt.Sprintf("openclaw:%s", b.agentID),
		"messages": messages,
	})
	if err != nil {
		return "", err
	}

	req, err := http.NewRequestWithContext(ctx, "POST",
		b.gatewayURL+"/v1/chat/completions",
		bytes.NewReader(body))
	if err != nil {
		return "", err
	}
	b.setHeaders(req)

	resp, err := b.client.Do(req)
	if err != nil {
		return "", fmt.Errorf("OpenClaw request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		respBody, _ := io.ReadAll(resp.Body)
		return "", fmt.Errorf("OpenClaw returned %d: %s", resp.StatusCode, string(respBody))
	}

	var result struct {
		Choices []struct {
			Message struct {
				Content string `json:"content"`
			} `json:"message"`
		} `json:"choices"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return "", err
	}

	if len(result.Choices) > 0 {
		return result.Choices[0].Message.Content, nil
	}
	return "", nil
}

// ChatStream sends a message to OpenClaw and streams the response via SSE.
// The callback is called for each text chunk as it arrives.
func (b *Bridge) ChatStream(ctx context.Context, message string, history []ChatMessage, onChunk func(string)) error {
	messages := b.buildMessages(message, history)

	body, err := json.Marshal(map[string]any{
		"model":    fmt.Sprintf("openclaw:%s", b.agentID),
		"messages": messages,
		"stream":   true,
	})
	if err != nil {
		return err
	}

	req, err := http.NewRequestWithContext(ctx, "POST",
		b.gatewayURL+"/v1/chat/completions",
		bytes.NewReader(body))
	if err != nil {
		return err
	}
	b.setHeaders(req)

	resp, err := b.client.Do(req)
	if err != nil {
		return fmt.Errorf("OpenClaw stream request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		respBody, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("OpenClaw returned %d: %s", resp.StatusCode, string(respBody))
	}

	// Parse SSE stream
	scanner := bufio.NewScanner(resp.Body)
	for scanner.Scan() {
		line := scanner.Text()
		if !strings.HasPrefix(line, "data: ") {
			continue
		}
		data := line[6:]
		if data == "[DONE]" {
			return nil
		}

		var chunk struct {
			Choices []struct {
				Delta struct {
					Content string `json:"content"`
				} `json:"delta"`
			} `json:"choices"`
		}
		if err := json.Unmarshal([]byte(data), &chunk); err != nil {
			continue
		}
		if len(chunk.Choices) > 0 && chunk.Choices[0].Delta.Content != "" {
			onChunk(chunk.Choices[0].Delta.Content)
		}
	}
	return scanner.Err()
}

// Analyze sends content to OpenClaw for entity extraction, task detection, etc.
// This uses a specialized system prompt for analysis rather than chat.
func (b *Bridge) Analyze(ctx context.Context, content string, existingContext string) (string, error) {
	analysisPrompt := `Analyze this input and extract structured data. Return JSON with:
- tasks: [{title, description, assigned_to, assigned_by, deadline, confidence}]
- events: [{title, description, start_time, end_time, participants, confidence}]
- conflicts: [{type, explanation}]
- follow_ups: [{target, question, urgency}]

Input context: ` + existingContext + `

Input to analyze:
` + content

	return b.Chat(ctx, analysisPrompt, nil)
}

// EnsureGateway starts the OpenClaw gateway in WSL if it's not already running.
// Implements retry logic with exponential backoff.
func (b *Bridge) EnsureGateway() {
	status := b.Detect()
	if status.Available {
		log.Printf("[openclaw] gateway already running at %s", b.gatewayURL)
		return
	}

	if !b.wsl.IsAvailable() {
		log.Printf("[openclaw] WSL not available, cannot auto-start gateway")
		return
	}

	b.startGatewayWithRetry()
}

// StopGateway stops the managed OpenClaw gateway process.
func (b *Bridge) StopGateway() {
	b.mu.Lock()
	defer b.mu.Unlock()

	if b.healthTicker != nil {
		b.healthTicker.Stop()
		close(b.stopHealth)
	}

	if b.gatewayCmd != nil && b.gatewayCmd.Process != nil {
		log.Printf("[openclaw] stopping gateway...")
		b.gatewayCmd.Process.Kill()
		b.gatewayCmd = nil
	}
}

// ForceKill kills all OpenClaw processes in WSL. Big red button.
func (b *Bridge) ForceKill() error {
	b.StopGateway()
	return b.wsl.ForceKillOpenClaw()
}

func (b *Bridge) startGatewayWithRetry() {
	b.mu.Lock()
	b.retries++
	if b.retries > b.maxRetries {
		b.mu.Unlock()
		log.Printf("[openclaw] gave up after %d attempts", b.maxRetries)
		return
	}
	attempt := b.retries
	b.mu.Unlock()

	log.Printf("[openclaw] starting gateway (attempt %d/%d)", attempt, b.maxRetries)

	cmd, err := b.wsl.StartGateway()
	if err != nil {
		log.Printf("[openclaw] failed to start: %v", err)
		time.AfterFunc(b.retryDelay, b.startGatewayWithRetry)
		return
	}

	b.mu.Lock()
	b.gatewayCmd = cmd
	b.mu.Unlock()

	// Monitor the process
	go func() {
		err := cmd.Wait()
		log.Printf("[openclaw] gateway exited: %v", err)
		b.mu.Lock()
		b.gatewayCmd = nil
		b.mu.Unlock()
		// Retry on unexpected exit
		time.AfterFunc(b.retryDelay, b.startGatewayWithRetry)
	}()

	// Start health checking
	b.startHealthCheck()
}

func (b *Bridge) startHealthCheck() {
	b.healthTicker = time.NewTicker(10 * time.Second)
	go func() {
		for {
			select {
			case <-b.healthTicker.C:
				status := b.Detect()
				if status.Available {
					b.mu.Lock()
					b.retries = 0
					b.mu.Unlock()
				}
			case <-b.stopHealth:
				return
			}
		}
	}()
}

func (b *Bridge) buildMessages(userMessage string, history []ChatMessage) []ChatMessage {
	messages := []ChatMessage{
		{Role: "system", Content: systemPrompt},
	}
	messages = append(messages, history...)
	messages = append(messages, ChatMessage{Role: "user", Content: userMessage})
	return messages
}

func (b *Bridge) setHeaders(req *http.Request) {
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("x-openclaw-agent-id", b.agentID)
	if b.authToken != "" {
		req.Header.Set("Authorization", "Bearer "+b.authToken)
	}
}
