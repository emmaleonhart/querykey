//! Local AI agent bridge. Port of server-go-old/internal/openclaw/.
//! The agent is model-agnostic (Gemma default); OpenClaw via a WSL
//! gateway is today's implementation detail. Deep retry/health/stream
//! loops are marked TODO against the Go reference.

mod bridge;
mod wsl;

pub use bridge::{Bridge, ChatMessage, Status};
pub use wsl::WslManager;
