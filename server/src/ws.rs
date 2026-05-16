//! Port of server-go-old/internal/ws/hub.go.
//! WebSocket hub: fan-out broadcast + per-connection chat streaming
//! through the local-agent bridge. Uses a tokio broadcast channel
//! instead of Go channels.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;

use crate::api::AppState;
use crate::models::{ChatRequest, GraphDiff, WsMessage};
use crate::openclaw::Bridge;

pub struct Hub {
    tx: broadcast::Sender<String>,
    clients: AtomicUsize,
    bridge: Arc<Bridge>,
}

impl Hub {
    pub fn new(bridge: Arc<Bridge>) -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self {
            tx,
            clients: AtomicUsize::new(0),
            bridge,
        }
    }

    pub fn client_count(&self) -> usize {
        self.clients.load(Ordering::Relaxed)
    }

    pub fn broadcast_message(&self, msg: WsMessage) {
        if let Ok(s) = serde_json::to_string(&msg) {
            let _ = self.tx.send(s);
        }
    }

    pub fn broadcast_graph_diff(&self, diff: &GraphDiff) {
        self.broadcast_message(WsMessage {
            msg_type: "graph_diff".to_string(),
            content: String::new(),
            data: serde_json::to_value(diff).ok(),
        });
    }
}

/// axum handler for GET /ws/chat (upgrade).
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let hub = &state.hub;
    hub.clients.fetch_add(1, Ordering::Relaxed);
    let mut rx = hub.tx.subscribe();
    let bridge = hub.bridge.clone();

    let (mut sink, mut stream) = socket.split();

    // Forward broadcast messages to this client.
    let broadcast_task = tokio::spawn(async move {
        while let Ok(s) = rx.recv().await {
            if sink.send(Message::Text(s)).await.is_err() {
                break;
            }
        }
    });

    // Read loop: treat each text frame as a ChatRequest.
    while let Some(Ok(msg)) = stream.next().await {
        if let Message::Text(txt) = msg {
            handle_chat(&state, &bridge, &txt).await;
        }
    }

    broadcast_task.abort();
    state.hub.clients.fetch_sub(1, Ordering::Relaxed);
}

/// Mirrors hub.go Client.handleChat: stream the agent reply back as
/// stream_start / stream_chunk / stream_end frames over the broadcast
/// channel (all connected clients share the conversation, matching the
/// old unified-inbox behavior).
async fn handle_chat(state: &Arc<AppState>, bridge: &Bridge, raw: &str) {
    let req: ChatRequest = serde_json::from_str(raw).unwrap_or_default();
    let content = if !req.content.is_empty() {
        req.content.clone()
    } else {
        req.message.clone()
    };
    if content.is_empty() {
        return;
    }
    let history = req
        .history
        .iter()
        .map(|h| crate::openclaw::ChatMessage {
            role: h.role.clone(),
            content: h.content.clone(),
        })
        .collect::<Vec<_>>();

    let hub = &state.hub;
    hub.broadcast_message(WsMessage {
        msg_type: "stream_start".to_string(),
        content: String::new(),
        data: None,
    });
    let mut acc = String::new();
    match bridge
        .chat_stream(&content, &history, |chunk| acc.push_str(chunk))
        .await
    {
        Ok(()) => {
            hub.broadcast_message(WsMessage {
                msg_type: "stream_chunk".to_string(),
                content: acc,
                data: None,
            });
            hub.broadcast_message(WsMessage {
                msg_type: "stream_end".to_string(),
                content: String::new(),
                data: None,
            });
        }
        Err(e) => {
            tracing::warn!("[ws] chat error: {e}");
            hub.broadcast_message(WsMessage {
                msg_type: "error".to_string(),
                content: e.to_string(),
                data: None,
            });
        }
    }
}
