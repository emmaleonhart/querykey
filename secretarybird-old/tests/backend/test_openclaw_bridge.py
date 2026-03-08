"""Tests for the OpenClaw bridge module."""
import os
import pytest

from backend.openclaw.bridge import OpenClawBridge, WSLManager


class TestWSLManager:
    """Test WSL management utilities."""

    def test_init(self):
        """Test WSLManager initialization."""
        manager = WSLManager()
        assert manager is not None

    def test_windows_to_wsl_path(self):
        """Test converting Windows paths to WSL paths."""
        manager = WSLManager()
        wsl_path = manager.windows_to_wsl_path(r"C:\Users\test\file.txt")
        assert wsl_path == "/mnt/c/Users/test/file.txt"

    def test_windows_to_wsl_path_drive_letter(self):
        """Test drive letter conversion."""
        manager = WSLManager()
        wsl_path = manager.windows_to_wsl_path(r"D:\Data\report.xlsx")
        assert wsl_path == "/mnt/d/Data/report.xlsx"

    def test_wsl_to_windows_path(self):
        """Test converting WSL paths to Windows paths."""
        manager = WSLManager()
        win_path = manager.wsl_to_windows_path("/mnt/c/Users/test/file.txt")
        assert win_path == r"C:\Users\test\file.txt"

    def test_wsl_to_windows_path_non_mnt(self):
        """Test that non-/mnt paths are returned as-is."""
        manager = WSLManager()
        path = "/home/user/file.txt"
        result = manager.wsl_to_windows_path(path)
        assert result == path

    def test_path_roundtrip(self):
        """Test that path conversion roundtrips correctly."""
        manager = WSLManager()
        original = r"C:\Users\test\Documents\file.xlsx"
        wsl = manager.windows_to_wsl_path(original)
        back = manager.wsl_to_windows_path(wsl)
        assert back == original


class TestOpenClawBridge:
    """Test OpenClaw bridge functionality."""

    def test_init_defaults(self):
        """Test bridge initialization with defaults."""
        bridge = OpenClawBridge()
        assert bridge is not None
        assert bridge._agent_id == "main"

    def test_init_custom(self):
        """Test bridge initialization with custom values."""
        bridge = OpenClawBridge(
            gateway_url="http://localhost:9999",
            auth_token="test-token",
            agent_id="beta",
        )
        assert bridge._gateway_url == "http://localhost:9999"
        assert bridge._auth_token == "test-token"
        assert bridge._agent_id == "beta"

    def test_build_headers_with_token(self):
        """Test that auth headers are built correctly."""
        bridge = OpenClawBridge(auth_token="my-secret-token")
        headers = bridge._build_headers()
        assert headers["Authorization"] == "Bearer my-secret-token"
        assert headers["Content-Type"] == "application/json"
        assert headers["x-openclaw-agent-id"] == "main"

    def test_build_headers_custom_agent(self):
        """Test headers include custom agent ID."""
        bridge = OpenClawBridge(agent_id="custom-agent")
        headers = bridge._build_headers()
        assert headers["x-openclaw-agent-id"] == "custom-agent"

    def test_detect_returns_dict(self):
        """Test that detect returns a properly structured dict."""
        bridge = OpenClawBridge(gateway_url="http://127.0.0.1:99999")
        result = bridge.detect()
        assert isinstance(result, dict)
        assert "available" in result
        assert "gateway_url" in result
        assert "agent_id" in result

    def test_detect_unreachable_gateway(self):
        """Test detect when gateway is not reachable."""
        bridge = OpenClawBridge(gateway_url="http://127.0.0.1:99999")
        result = bridge.detect()
        assert result["available"] is False
        assert "error" in result

    def test_env_override_gateway_url(self):
        """Test that OPENCLAW_GATEWAY_URL env var is respected."""
        bridge = OpenClawBridge(gateway_url="http://custom:1234")
        assert bridge._gateway_url == "http://custom:1234"
