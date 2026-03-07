"""
OpenClaw Bridge Module

Manages the OpenClaw CLI subprocess lifecycle:
- Detects and manages WSL availability
- Translates Windows <-> WSL paths
- Spawns and communicates with OpenClaw processes
- Session management with unique IDs

OpenClaw is LLM-agnostic and can work with any LLM backend
(cloud APIs, local models, etc.). The CLI command and arguments
are fully configurable.
"""

import logging
import os
import shutil
import subprocess
import uuid
from typing import Any, Optional

logger = logging.getLogger("tojo.openclaw_bridge")

# Default command to invoke OpenClaw. Can be overridden via environment
# variable OPENCLAW_CMD or by passing `command` to OpenClawBridge.
DEFAULT_OPENCLAW_CMD = os.environ.get("OPENCLAW_CMD", "openclaw")


# ---------------------------------------------------------------------------
# WSL Manager
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
        # Normalize backslashes
        path = windows_path.replace("\\", "/")

        # Check for drive letter pattern (e.g., C:/)
        if len(path) >= 2 and path[1] == ":":
            drive_letter = path[0].lower()
            rest = path[2:]  # skip "C:"
            return f"/mnt/{drive_letter}{rest}"

        return path

    def wsl_to_windows_path(self, wsl_path: str) -> str:
        """
        Convert a WSL path back to Windows format.

        Example: /mnt/c/Users/test/file.txt -> C:\\Users\\test\\file.txt
        """
        if wsl_path.startswith("/mnt/") and len(wsl_path) >= 6:
            drive_letter = wsl_path[5].upper()
            rest = wsl_path[6:]  # skip "/mnt/c"
            # Convert forward slashes to backslashes
            return f"{drive_letter}:{rest}".replace("/", "\\")

        return wsl_path

    def run_command(self, command: str, timeout: int = 60) -> dict[str, Any]:
        """
        Execute a command inside WSL.

        Args:
            command: Shell command to run in WSL.
            timeout: Maximum seconds to wait.

        Returns:
            Dictionary with stdout, stderr, and return code.
        """
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
            return {
                "stdout": "",
                "stderr": f"Command timed out after {timeout}s",
                "returncode": -1,
            }
        except FileNotFoundError:
            return {
                "stdout": "",
                "stderr": "WSL not found",
                "returncode": -1,
            }


# ---------------------------------------------------------------------------
# OpenClaw Bridge
# ---------------------------------------------------------------------------

class OpenClawBridge:
    """
    Bridge to the OpenClaw CLI process.

    OpenClaw is LLM-agnostic — it works with any LLM backend.
    The command and arguments are configurable via constructor
    or the OPENCLAW_CMD environment variable.

    Manages subprocess lifecycle, input/output streaming,
    and session tracking.
    """

    def __init__(self, command: Optional[str] = None):
        """
        Args:
            command: CLI command to invoke OpenClaw (default: env OPENCLAW_CMD or 'openclaw').
        """
        self.command = command or DEFAULT_OPENCLAW_CMD
        self.process: Optional[subprocess.Popen] = None
        self.wsl_manager = WSLManager()
        self._session_id: Optional[str] = None
        self._output_buffer: list[str] = []

    def _generate_session_id(self) -> str:
        """Generate a unique session identifier."""
        return str(uuid.uuid4())

    def is_running(self) -> bool:
        """Check whether the OpenClaw process is currently running."""
        if self.process is None:
            return False
        return self.process.poll() is None

    def _build_command(self, prompt: str) -> list[str]:
        """
        Build the command to launch OpenClaw.

        Args:
            prompt: The initial prompt/message to send.

        Returns:
            Command as a list of strings.
        """
        # Base command - uses WSL if available on Windows and command
        # is not found natively
        cmd = self.command
        if os.name == "nt" and not shutil.which(cmd) and self.wsl_manager.is_available():
            return ["wsl", cmd, "--print", prompt]
        else:
            return [cmd, "--print", prompt]

    def detect(self) -> dict[str, Any]:
        """
        Detect whether OpenClaw is available on this system.

        Returns:
            Dictionary with 'available' bool, 'command' string, and 'via_wsl' bool.
        """
        cmd = self.command

        # Check native availability first
        if shutil.which(cmd):
            return {"available": True, "command": cmd, "via_wsl": False}

        # On Windows, check WSL
        if os.name == "nt" and self.wsl_manager.is_available():
            result = self.wsl_manager.run_command(f"which {cmd}", timeout=10)
            if result["returncode"] == 0:
                return {"available": True, "command": cmd, "via_wsl": True}

        return {"available": False, "command": cmd, "via_wsl": False}

    def start(self, prompt: str) -> str:
        """
        Start an OpenClaw session with an initial prompt.

        Args:
            prompt: The initial message to send to OpenClaw.

        Returns:
            Session ID for tracking this session.
        """
        if self.is_running():
            self.stop()

        self._session_id = self._generate_session_id()
        self._output_buffer = []

        cmd = self._build_command(prompt)
        logger.info("Starting OpenClaw session %s: %s", self._session_id, " ".join(cmd))

        try:
            self.process = subprocess.Popen(
                cmd,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
        except FileNotFoundError as exc:
            logger.error("Failed to start OpenClaw: %s", exc)
            raise RuntimeError(
                f"OpenClaw ('{cmd[0]}') not found. Ensure it is installed and "
                "accessible, or set the OPENCLAW_CMD environment variable."
            ) from exc

        return self._session_id

    def stop(self) -> None:
        """Stop the current OpenClaw session."""
        if self.process is not None:
            logger.info("Stopping OpenClaw session %s", self._session_id)
            try:
                self.process.terminate()
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
            finally:
                self.process = None

    def send(self, message: str) -> None:
        """
        Send a message to the running OpenClaw process.

        Args:
            message: Text to send to stdin.
        """
        if not self.is_running():
            raise RuntimeError("OpenClaw is not running.")
        if self.process and self.process.stdin:
            self.process.stdin.write(message + "\n")
            self.process.stdin.flush()

    def read_output(self, timeout: float = 5.0) -> str:
        """
        Read available output from the OpenClaw process.

        Args:
            timeout: Maximum seconds to wait for output.

        Returns:
            Output text from OpenClaw.
        """
        if not self.is_running():
            # Process finished, read remaining output
            if self.process and self.process.stdout:
                remaining = self.process.stdout.read()
                if remaining:
                    self._output_buffer.append(remaining)

        if self.process and self.process.stdout:
            import select
            import sys

            if sys.platform != "win32":
                # Unix: use select for non-blocking read
                ready, _, _ = select.select([self.process.stdout], [], [], timeout)
                if ready:
                    line = self.process.stdout.readline()
                    if line:
                        self._output_buffer.append(line)
                        return line
            else:
                # Windows: blocking read with timeout via thread
                line = self.process.stdout.readline()
                if line:
                    self._output_buffer.append(line)
                    return line

        return ""

    def get_full_output(self) -> str:
        """Return all captured output from this session."""
        return "".join(self._output_buffer)

    @property
    def session_id(self) -> Optional[str]:
        """Current session ID."""
        return self._session_id
