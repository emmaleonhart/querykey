package ws

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"github.com/secretarybird/server/internal/models"
	"github.com/secretarybird/server/internal/openclaw"
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
	CheckOrigin: func(r *http.Request) bool {
		// Allow connections from Flutter app, Electron, localhost
		return true
	},
}

// Client represents a single WebSocket connection.
type Client struct {
	hub  *Hub
	conn *websocket.Conn
	send chan []byte
	id   string
}

// Hub manages all WebSocket clients and broadcasts.
// Mirrors the old Python WebSocket pattern: stream_start/stream_chunk/stream_end protocol.
type Hub struct {
	clients    map[*Client]bool
	broadcast  chan []byte
	register   chan *Client
	unregister chan *Client
	mu         sync.RWMutex

	bridge *openclaw.Bridge
}

// NewHub creates a WebSocket hub.
func NewHub(bridge *openclaw.Bridge) *Hub {
	return &Hub{
		clients:    make(map[*Client]bool),
		broadcast:  make(chan []byte, 256),
		register:   make(chan *Client),
		unregister: make(chan *Client),
		bridge:     bridge,
	}
}

// Run starts the hub's event loop. Call in a goroutine.
func (h *Hub) Run() {
	for {
		select {
		case client := <-h.register:
			h.mu.Lock()
			h.clients[client] = true
			h.mu.Unlock()
			log.Printf("[ws] client connected (%d total)", len(h.clients))

			// Notify client of OpenClaw status
			status := h.bridge.Detect()
			msg, _ := json.Marshal(models.WSMessage{
				Type: "status",
				Data: status,
			})
			client.send <- msg

		case client := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[client]; ok {
				delete(h.clients, client)
				close(client.send)
			}
			h.mu.Unlock()
			log.Printf("[ws] client disconnected (%d total)", len(h.clients))

		case message := <-h.broadcast:
			h.mu.RLock()
			for client := range h.clients {
				select {
				case client.send <- message:
				default:
					close(client.send)
					delete(h.clients, client)
				}
			}
			h.mu.RUnlock()
		}
	}
}

// BroadcastGraphDiff sends a graph diff to all connected clients.
func (h *Hub) BroadcastGraphDiff(diff *models.GraphDiff) {
	msg, err := json.Marshal(models.WSMessage{
		Type: "graph_diff",
		Data: diff,
	})
	if err != nil {
		log.Printf("[ws] error marshaling graph diff: %v", err)
		return
	}
	h.broadcast <- msg
}

// BroadcastMessage sends an arbitrary message to all connected clients.
func (h *Hub) BroadcastMessage(wsMsg models.WSMessage) {
	msg, err := json.Marshal(wsMsg)
	if err != nil {
		log.Printf("[ws] error marshaling message: %v", err)
		return
	}
	h.broadcast <- msg
}

// ClientCount returns the number of connected clients.
func (h *Hub) ClientCount() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.clients)
}

// HandleWebSocket is the HTTP handler for WebSocket upgrade requests.
// Mount this at /ws/chat to match the old protocol.
func (h *Hub) HandleWebSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("[ws] upgrade error: %v", err)
		return
	}

	client := &Client{
		hub:  h,
		conn: conn,
		send: make(chan []byte, 256),
		id:   r.RemoteAddr,
	}

	h.register <- client

	go client.writePump()
	go client.readPump()
}

// readPump reads messages from the WebSocket client and routes them.
func (c *Client) readPump() {
	defer func() {
		c.hub.unregister <- c
		c.conn.Close()
	}()

	c.conn.SetReadLimit(512 * 1024) // 512KB max message
	c.conn.SetReadDeadline(time.Now().Add(60 * time.Second))
	c.conn.SetPongHandler(func(string) error {
		c.conn.SetReadDeadline(time.Now().Add(60 * time.Second))
		return nil
	})

	for {
		_, raw, err := c.conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseNormalClosure) {
				log.Printf("[ws] read error: %v", err)
			}
			break
		}

		var req models.ChatRequest
		if err := json.Unmarshal(raw, &req); err != nil {
			c.sendJSON(models.WSMessage{Type: "error", Content: "Invalid JSON"})
			continue
		}

		// Get the message content (support both "content" and "message" fields)
		message := req.Content
		if message == "" {
			message = req.Message
		}
		if message == "" {
			c.sendJSON(models.WSMessage{Type: "error", Content: "Empty message"})
			continue
		}

		// Handle the chat message
		go c.handleChat(message, req)
	}
}

// handleChat processes a chat message, streaming the response from OpenClaw.
func (c *Client) handleChat(message string, req models.ChatRequest) {
	bridge := c.hub.bridge

	// Check OpenClaw availability
	status := bridge.Detect()
	if !status.Available {
		// Fallback response when OpenClaw is not available
		c.sendJSON(models.WSMessage{Type: "stream_start"})
		fallback := "OpenClaw gateway is not connected. Start it in WSL with: openclaw gateway\n\nI can help with task tracking, scheduling, and team coordination once connected."
		c.sendJSON(models.WSMessage{Type: "stream_chunk", Content: fallback})
		c.sendJSON(models.WSMessage{Type: "stream_end"})
		return
	}

	// Build history from request
	var history []openclaw.ChatMessage
	for _, h := range req.History {
		if h.Role == "user" || h.Role == "assistant" {
			history = append(history, openclaw.ChatMessage{
				Role:    h.Role,
				Content: h.Content,
			})
		}
	}

	// Stream response from OpenClaw
	c.sendJSON(models.WSMessage{Type: "stream_start"})

	err := bridge.ChatStream(context.Background(), message, history, func(chunk string) {
		c.sendJSON(models.WSMessage{Type: "stream_chunk", Content: chunk})
	})
	if err != nil {
		log.Printf("[ws] OpenClaw stream error: %v", err)
		c.sendJSON(models.WSMessage{Type: "error", Content: "OpenClaw streaming failed: " + err.Error()})
	}

	c.sendJSON(models.WSMessage{Type: "stream_end"})
	c.sendJSON(models.WSMessage{Type: "status", Data: openclaw.Status{Available: true}})
}

// writePump sends messages from the hub to the WebSocket client.
func (c *Client) writePump() {
	ticker := time.NewTicker(54 * time.Second)
	defer func() {
		ticker.Stop()
		c.conn.Close()
	}()

	for {
		select {
		case message, ok := <-c.send:
			c.conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if !ok {
				c.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			w, err := c.conn.NextWriter(websocket.TextMessage)
			if err != nil {
				return
			}
			w.Write(message)

			// Drain queued messages into the same write
			n := len(c.send)
			for i := 0; i < n; i++ {
				w.Write([]byte("\n"))
				w.Write(<-c.send)
			}

			if err := w.Close(); err != nil {
				return
			}

		case <-ticker.C:
			c.conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if err := c.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}

func (c *Client) sendJSON(msg models.WSMessage) {
	data, err := json.Marshal(msg)
	if err != nil {
		return
	}
	select {
	case c.send <- data:
	default:
		// Client send buffer full, drop message
	}
}
