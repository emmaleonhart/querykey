//! Port of server-go-old/internal/openclaw/bridge.go.
//! HTTP calls to the local agent gateway (OpenAI-compatible API).

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::WslManager;

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

pub struct Bridge {
    gateway_url: String,
    agent_id: String,
    auth_token: String,
    http: reqwest::Client,
    wsl: WslManager,
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
        }
    }

    fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.auth_token.is_empty() {
            rb
        } else {
            rb.bearer_auth(&self.auth_token)
        }
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
        let mut msgs = history.to_vec();
        msgs.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });
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
        let mut msgs = history.to_vec();
        msgs.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });
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

    /// Attempt to auto-start the gateway in WSL. TODO(port): retry loop
    /// + health polling — see bridge.go startGatewayWithRetry().
    pub async fn ensure_gateway(&self) {
        if self.wsl.is_available() {
            tracing::info!("[openclaw] attempting WSL gateway start (best-effort)");
            let _ = self.wsl.start_gateway();
        } else {
            tracing::warn!("[openclaw] WSL not available; gateway not started");
        }
    }

    pub fn stop_gateway(&self) {
        // TODO(port): graceful gateway stop — see bridge.go StopGateway()
    }

    pub fn force_kill(&self) -> anyhow::Result<()> {
        self.wsl.force_kill_openclaw()
    }
}
