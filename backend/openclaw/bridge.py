"""
OpenClaw Bridge Module

Connects to the OpenClaw Gateway via its OpenAI-compatible HTTP API:
- POST /v1/chat/completions (streaming and non-streaming)
- Gateway runs at ws://127.0.0.1:18789 by default (configurable)
- Authentication via bearer token from OpenClaw config

OpenClaw is LLM-agnostic and can work with any LLM backend
(cloud APIs, local models, etc.). The gateway is the control plane;
Tojo Assistant is just another client.

Also includes WSL path translation utilities.
"""

import json
import logging
import os
import subprocess
from typing import Any, AsyncIterator, Optional

import httpx

logger = logging.getLogger("tojo.openclaw_bridge")

# Gateway defaults — can be overridden via environment variables
DEFAULT_GATEWAY_URL = os.environ.get("OPENCLAW_GATEWAY_URL", "http://127.0.0.1:18789")
DEFAULT_AGENT_ID = os.environ.get("OPENCLAW_AGENT_ID", "main")


# ---------------------------------------------------------------------------
# WSL Manager (kept for path translation utilities)
# ---------------------------------------------------------------------------

class WSLManager:
    """
    Utility for managing WSL (Windows Subsystem for Linux) interactions,
    including path translation and availability detection.
    """

    def __init__(self):
        self._available: Optional[bool] = None

    def is_available(self) -> bool:
        """Check whether WSL is available on this system."""
        if self._available is not None:
            return self._available

        try:
            result = subprocess.run(
                ["wsl", "--status"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            self._available = result.returncode == 0
        except (FileNotFoundError, subprocess.TimeoutExpired):
            self._available = False

        logger.info("WSL available: %s", self._available)
        return self._available

    def windows_to_wsl_path(self, windows_path: str) -> str:
        """
        Convert a Windows path to its WSL equivalent.

        Example: C:\\Users\\test\\file.txt -> /mnt/c/Users/test/file.txt
        """
        path = windows_path.replace("\\", "/")
        if len(path) >= 2 and path[1] == ":":
            drive_letter = path[0].lower()
            rest = path[2:]
            return f"/mnt/{drive_letter}{rest}"
        return path

    def wsl_to_windows_path(self, wsl_path: str) -> str:
        """
        Convert a WSL path back to Windows format.

        Example: /mnt/c/Users/test/file.txt -> C:\\Users\\test\\file.txt
        """
        if wsl_path.startswith("/mnt/") and len(wsl_path) >= 6:
            drive_letter = wsl_path[5].upper()
            rest = wsl_path[6:]
            return f"{drive_letter}:{rest}".replace("/", "\\")
        return wsl_path

    def run_command(self, command: str, timeout: int = 60) -> dict[str, Any]:
        """Execute a command inside WSL."""
        try:
            result = subprocess.run(
                ["wsl", "bash", "-c", command],
                capture_output=True,
                text=True,
                timeout=timeout,
            )
            return {
                "stdout": result.stdout,
                "stderr": result.stderr,
                "returncode": result.returncode,
            }
        except subprocess.TimeoutExpired:
            return {"stdout": "", "stderr": f"Command timed out after {timeout}s", "returncode": -1}
        except FileNotFoundError:
            return {"stdout": "", "stderr": "WSL not found", "returncode": -1}


# ---------------------------------------------------------------------------
# OpenClaw config reader
# ---------------------------------------------------------------------------

def _read_openclaw_config() -> dict[str, Any]:
    """
    Read the OpenClaw gateway config to get port and auth token.

    Checks ~/.openclaw/openclaw.json (via WSL if on Windows).
    """
    config_paths = []

    if os.name == "nt":
        # On Windows, OpenClaw config lives inside WSL
        wsl_home = os.environ.get("WSL_HOME", "")
        if wsl_home:
            config_paths.append(os.path.join(wsl_home, ".openclaw", "openclaw.json"))
        # Try reading via wsl command
        try:
            result = subprocess.run(
                ["wsl", "-d", "Ubuntu", "--", "bash", "-lc", "cat ~/.openclaw/openclaw.json"],
                capture_output=True, text=True, timeout=10,
            )
            if result.returncode == 0 and result.stdout.strip():
                return json.loads(result.stdout)
        except (FileNotFoundError, subprocess.TimeoutExpired, json.JSONDecodeError):
            pass
    else:
        home = os.path.expanduser("~")
        config_path = os.path.join(home, ".openclaw", "openclaw.json")
        config_paths.append(config_path)

    for path in config_paths:
        if os.path.isfile(path):
            with open(path, "r") as f:
                return json.load(f)

    return {}


# ---------------------------------------------------------------------------
# OpenClaw Bridge
# ---------------------------------------------------------------------------

class OpenClawBridge:
    """
    Bridge to the OpenClaw Gateway via its OpenAI-compatible HTTP API.

    The gateway exposes POST /v1/chat/completions which accepts standard
    OpenAI Chat Completions format. Tojo Assistant sends user messages
    through this endpoint and receives LLM responses.

    This is much more reliable than subprocess management — the gateway
    handles all the LLM communication, session management, and tool execution.
    """

    def __init__(
        self,
        gateway_url: Optional[str] = None,
        auth_token: Optional[str] = None,
        agent_id: Optional[str] = None,
    ):
        """
        Args:
            gateway_url: Gateway base URL (default: env OPENCLAW_GATEWAY_URL or http://127.0.0.1:18789).
            auth_token: Bearer token for gateway auth (auto-read from OpenClaw config if not provided).
            agent_id: OpenClaw agent ID to use (default: env OPENCLAW_AGENT_ID or 'main').
        """
        self._gateway_url = gateway_url or DEFAULT_GATEWAY_URL
        self._agent_id = agent_id or DEFAULT_AGENT_ID
        self._auth_token = auth_token
        self._config: Optional[dict[str, Any]] = None
        self.wsl_manager = WSLManager()

    def _get_config(self) -> dict[str, Any]:
        """Lazily load OpenClaw config."""
        if self._config is None:
            self._config = _read_openclaw_config()
        return self._config

    def _get_auth_token(self) -> Optional[str]:
        """Get the gateway auth token from config or constructor."""
        if self._auth_token:
            return self._auth_token

        # Try environment variable
        env_token = os.environ.get("OPENCLAW_GATEWAY_TOKEN")
        if env_token:
            self._auth_token = env_token
            return self._auth_token

        # Read from OpenClaw config
        config = self._get_config()
        gateway_config = config.get("gateway", {})
        auth_config = gateway_config.get("auth", {})
        self._auth_token = auth_config.get("token")
        return self._auth_token

    def _get_gateway_url(self) -> str:
        """Get the gateway URL, potentially from config."""
        if self._gateway_url != DEFAULT_GATEWAY_URL:
            return self._gateway_url

        config = self._get_config()
        gateway_config = config.get("gateway", {})
        port = gateway_config.get("port", 18789)
        bind = gateway_config.get("bind", "loopback")

        host = "127.0.0.1" if bind == "loopback" else "0.0.0.0"
        self._gateway_url = f"http://{host}:{port}"
        return self._gateway_url

    def _build_headers(self) -> dict[str, str]:
        """Build HTTP headers including auth."""
        headers = {
            "Content-Type": "application/json",
            "x-openclaw-agent-id": self._agent_id,
        }
        token = self._get_auth_token()
        if token:
            headers["Authorization"] = f"Bearer {token}"
        return headers

    def detect(self) -> dict[str, Any]:
        """
        Detect whether the OpenClaw gateway is reachable and the
        chat completions endpoint is enabled.

        Returns:
            Dictionary with 'available', 'gateway_url', 'agent_id', and optional 'error'.
        """
        url = self._get_gateway_url()
        try:
            with httpx.Client(timeout=5.0) as client:
                # First check if gateway is up at all
                resp = client.get(url)
                if resp.status_code != 200:
                    return {
                        "available": False,
                        "gateway_url": url,
                        "agent_id": self._agent_id,
                        "error": f"Gateway returned status {resp.status_code}",
                    }

                # Test the chat completions endpoint with a minimal request
                chat_url = f"{url}/v1/chat/completions"
                chat_resp = client.post(
                    chat_url,
                    headers=self._build_headers(),
                    json={
                        "model": f"openclaw:{self._agent_id}",
                        "messages": [{"role": "user", "content": "ping"}],
                    },
                )

                if chat_resp.status_code == 405:
                    return {
                        "available": False,
                        "gateway_url": url,
                        "agent_id": self._agent_id,
                        "error": "Chat completions endpoint is disabled. "
                                 "Enable it in ~/.openclaw/openclaw.json: "
                                 "gateway.http.endpoints.chatCompletions.enabled = true",
                    }

                if chat_resp.status_code == 401:
                    return {
                        "available": False,
                        "gateway_url": url,
                        "agent_id": self._agent_id,
                        "error": "Authentication failed. Check gateway token.",
                    }

                return {
                    "available": chat_resp.status_code == 200,
                    "gateway_url": url,
                    "agent_id": self._agent_id,
                }

        except httpx.ConnectError:
            return {
                "available": False,
                "gateway_url": url,
                "agent_id": self._agent_id,
                "error": "Cannot connect to OpenClaw gateway. "
                         "Start it with: openclaw gateway (in WSL/Ubuntu)",
            }
        except Exception as exc:
            return {
                "available": False,
                "gateway_url": url,
                "agent_id": self._agent_id,
                "error": str(exc),
            }

    def chat(
        self,
        message: str,
        history: Optional[list[dict[str, str]]] = None,
    ) -> str:
        """
        Send a message to OpenClaw and get a response (non-streaming).

        Args:
            message: User message text.
            history: Optional prior conversation messages
                     (list of {"role": "user"|"assistant", "content": "..."}).

        Returns:
            Assistant response text.
        """
        url = f"{self._get_gateway_url()}/v1/chat/completions"
        messages = list(history or [])
        messages.append({"role": "user", "content": message})

        with httpx.Client(timeout=120.0) as client:
            resp = client.post(
                url,
                headers=self._build_headers(),
                json={
                    "model": f"openclaw:{self._agent_id}",
                    "messages": messages,
                },
            )
            resp.raise_for_status()
            data = resp.json()

        choices = data.get("choices", [])
        if choices:
            return choices[0].get("message", {}).get("content", "")
        return ""

    async def chat_stream(
        self,
        message: str,
        history: Optional[list[dict[str, str]]] = None,
    ) -> AsyncIterator[str]:
        """
        Send a message to OpenClaw and stream the response via SSE.

        Args:
            message: User message text.
            history: Optional prior conversation messages.

        Yields:
            Response text chunks as they arrive.
        """
        url = f"{self._get_gateway_url()}/v1/chat/completions"
        messages = list(history or [])
        messages.append({"role": "user", "content": message})

        async with httpx.AsyncClient(timeout=120.0) as client:
            async with client.stream(
                "POST",
                url,
                headers=self._build_headers(),
                json={
                    "model": f"openclaw:{self._agent_id}",
                    "messages": messages,
                    "stream": True,
                },
            ) as resp:
                resp.raise_for_status()
                async for line in resp.aiter_lines():
                    if not line.startswith("data: "):
                        continue
                    data_str = line[6:]
                    if data_str == "[DONE]":
                        return
                    try:
                        chunk = json.loads(data_str)
                        delta = chunk.get("choices", [{}])[0].get("delta", {})
                        content = delta.get("content", "")
                        if content:
                            yield content
                    except (json.JSONDecodeError, IndexError, KeyError):
                        continue
