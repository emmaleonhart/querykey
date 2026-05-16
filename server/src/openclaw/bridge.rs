//! Port of server-go-old/internal/openclaw/bridge.go.
//! HTTP calls to the local agent gateway (OpenAI-compatible API).

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;

use super::WslManager;

/// Secretary tone (CLAUDE.md "Agent tone: secretary, not consultant").
/// Port of bridge.go's systemPrompt (kept concise).
const SYSTEM_PROMPT: &str = "You are QueryKey's local secretary agent. \
Be a secretary, not a consultant: short, direct, never wordy. Ask one \
clear question when unsure rather than guessing. Surface confidence and \
say when you don't know.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Status {
    pub available: bool,
    pub gateway_url: String,
    pub agent_id: String,
    pub error: String,
}

/// Managed-gateway lifecycle state (port of bridge.go's mutex-guarded
/// gatewayCmd / retries / health ticker).
#[derive(Default)]
struct GatewayState {
    retries: u32,
    stop_requested: bool,
    supervisor: Option<JoinHandle<()>>,
    health_task: Option<JoinHandle<()>>,
}

pub struct Bridge {
    gateway_url: String,
    agent_id: String,
    auth_token: String,
    http: reqwest::Client,
    wsl: WslManager,
    state: Mutex<GatewayState>,
    max_retries: u32,
    retry_delay: Duration,
}

impl Bridge {
    pub fn new(gateway_url: &str, agent_id: &str, auth_token: &str) -> Self {
        Self {
            gateway_url: gateway_url.trim_end_matches('/').to_string(),
            agent_id: agent_id.to_string(),
            auth_token: auth_token.to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
            wsl: WslManager::new(),
            state: Mutex::new(GatewayState::default()),
            max_retries: 5,
            retry_delay: Duration::from_secs(3),
        }
    }

    /// Port of bridge.go setHeaders(): agent-id header + bearer auth.
    fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let rb = rb.header("x-openclaw-agent-id", &self.agent_id);
        if self.auth_token.is_empty() {
            rb
        } else {
            rb.bearer_auth(&self.auth_token)
        }
    }

    /// Port of bridge.go buildMessages(): system prompt + history + user.
    fn build_messages(&self, user: &str, history: &[ChatMessage]) -> Vec<ChatMessage> {
        let mut msgs = Vec::with_capacity(history.len() + 2);
        msgs.push(ChatMessage {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
        });
        msgs.extend_from_slice(history);
        msgs.push(ChatMessage {
            role: "user".to_string(),
            content: user.to_string(),
        });
        msgs
    }

    /// Probe the gateway. Mirrors bridge.go Detect().
    pub async fn detect(&self) -> Status {
        let url = format!("{}/health", self.gateway_url);
        match self
            .auth(self.http.get(&url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => Status {
                available: true,
                gateway_url: self.gateway_url.clone(),
                agent_id: self.agent_id.clone(),
                error: String::new(),
            },
            Ok(resp) => Status {
                available: false,
                gateway_url: self.gateway_url.clone(),
                agent_id: self.agent_id.clone(),
                error: format!("gateway returned {}", resp.status()),
            },
            Err(e) => Status {
                available: false,
                gateway_url: self.gateway_url.clone(),
                agent_id: self.agent_id.clone(),
                error: e.to_string(),
            },
        }
    }

    /// Non-streaming chat completion (OpenAI-compatible).
    pub async fn chat(
        &self,
        message: &str,
        history: &[ChatMessage],
    ) -> anyhow::Result<String> {
        let msgs = self.build_messages(message, history);
        let body = serde_json::json!({
            "model": self.agent_id,
            "messages": msgs,
            "stream": false,
        });
        let resp = self
            .auth(self.http.post(format!("{}/v1/chat/completions", self.gateway_url)))
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let v: serde_json::Value = resp.json().await?;
        let content = v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        Ok(content)
    }

    /// Streaming chat over SSE. Port of bridge.go ChatStream(): POST
    /// with `stream: true`, parse `data: {...}` lines, deliver each
    /// `choices[0].delta.content` to `on_chunk` as it arrives, stop on
    /// `data: [DONE]`. Handles SSE lines split across network chunks.
    pub async fn chat_stream<F>(
        &self,
        message: &str,
        history: &[ChatMessage],
        mut on_chunk: F,
    ) -> anyhow::Result<()>
    where
        F: FnMut(&str),
    {
        let msgs = self.build_messages(message, history);
        let body = serde_json::json!({
            "model": self.agent_id,
            "messages": msgs,
            "stream": true,
        });
        let resp = self
            .auth(self.http.post(format!("{}/v1/chat/completions", self.gateway_url)))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let code = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("agent gateway returned {code}: {text}");
        }

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            buf.push_str(&String::from_utf8_lossy(bytes.as_ref()));
            // Drain complete lines; keep any partial trailing line.
            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].trim_end_matches('\r').trim().to_string();
                buf.drain(..=nl);
                let data = match line.strip_prefix("data: ") {
                    Some(d) => d,
                    None => continue,
                };
                if data == "[DONE]" {
                    return Ok(());
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(c) = v["choices"][0]["delta"]["content"].as_str() {
                        if !c.is_empty() {
                            on_chunk(c);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Analyze unstructured content; expects a JSON AnalysisResult-ish
    /// string back. Mirrors bridge.go Analyze().
    pub async fn analyze(
        &self,
        content: &str,
        existing_context: &str,
    ) -> anyhow::Result<String> {
        let prompt = format!(
            "Extract tasks, events, instructions, conflicts and messages \
             as JSON from the following. Context: {existing_context}\n\n{content}"
        );
        self.chat(&prompt, &[]).await
    }

    /// Port of bridge.go EnsureGateway(): detect first; if reachable,
    /// nothing to do; else (WSL available) start the supervised
    /// retry+health loop. Takes Arc<Self> so the background tasks can
    /// hold the bridge.
    pub async fn ensure_gateway(self: Arc<Self>) {
        if self.detect().await.available {
            tracing::info!("[openclaw] gateway already running at {}", self.gateway_url);
            return;
        }
        if !self.wsl.is_available() {
            tracing::warn!("[openclaw] WSL not available, cannot auto-start gateway");
            return;
        }
        {
            let mut s = self.state.lock().unwrap();
            s.stop_requested = false;
            if s.supervisor.is_some() {
                return; // already supervising
            }
        }
        let me = self.clone();
        let supervisor = tokio::spawn(async move { me.gateway_supervisor().await });
        self.state.lock().unwrap().supervisor = Some(supervisor);
        self.clone().start_health_check();
    }

    /// Port of bridge.go startGatewayWithRetry()'s loop: start → wait
    /// for the gateway to exit → backoff → retry, capped by
    /// `max_retries` (the health check resets the counter while it's
    /// reachable, so a healthy gateway keeps being respawned).
    async fn gateway_supervisor(self: Arc<Self>) {
        loop {
            {
                let mut s = self.state.lock().unwrap();
                if s.stop_requested {
                    return;
                }
                s.retries += 1;
                if s.retries > self.max_retries {
                    tracing::warn!(
                        "[openclaw] gave up after {} attempts",
                        self.max_retries
                    );
                    return;
                }
                tracing::info!(
                    "[openclaw] starting gateway (attempt {}/{})",
                    s.retries,
                    self.max_retries
                );
            }
            match self.wsl.start_gateway() {
                Ok(mut child) => {
                    let _ = child.wait().await;
                    tracing::warn!("[openclaw] gateway exited");
                }
                Err(e) => tracing::warn!("[openclaw] failed to start: {e}"),
            }
            if self.state.lock().unwrap().stop_requested {
                return;
            }
            tokio::time::sleep(self.retry_delay).await;
        }
    }

    /// Port of bridge.go startHealthCheck(): every 10s, if the gateway
    /// is reachable, reset the retry counter.
    fn start_health_check(self: Arc<Self>) {
        let me = self.clone();
        let task = tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(10));
            loop {
                tick.tick().await;
                if me.state.lock().unwrap().stop_requested {
                    return;
                }
                if me.detect().await.available {
                    me.state.lock().unwrap().retries = 0;
                }
            }
        });
        self.state.lock().unwrap().health_task = Some(task);
    }

    /// Port of bridge.go StopGateway(): stop supervising + kill the
    /// gateway. Killing it in WSL unblocks the supervisor's wait().
    pub fn stop_gateway(&self) {
        let (sup, health) = {
            let mut s = self.state.lock().unwrap();
            s.stop_requested = true;
            (s.supervisor.take(), s.health_task.take())
        };
        if let Some(h) = health {
            h.abort();
        }
        let _ = self.wsl.force_kill_openclaw();
        if let Some(h) = sup {
            h.abort();
        }
        tracing::info!("[openclaw] gateway stopped");
    }

    /// Port of bridge.go ForceKill(): StopGateway + kill everything.
    pub fn force_kill(&self) -> anyhow::Result<()> {
        self.stop_gateway();
        self.wsl.force_kill_openclaw()
    }
}
