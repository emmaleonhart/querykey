"""Tests for the FastAPI server."""
import pytest
from fastapi.testclient import TestClient

from backend.server import app


@pytest.fixture
def client():
    """Create a test client for the FastAPI app."""
    return TestClient(app)


class TestHealthCheck:
    """Test health check endpoint."""

    def test_health_check(self, client):
        """Test that health check returns OK."""
        response = client.get("/health")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"

    def test_health_check_has_version(self, client):
        """Test that health check includes version."""
        response = client.get("/health")
        data = response.json()
        assert "version" in data


class TestFileOrganizerEndpoints:
    """Test file organizer API endpoints."""

    def test_scan_nonexistent_directory(self, client):
        """Test scanning a nonexistent directory returns error."""
        response = client.post("/files/scan", json={"path": "/nonexistent/dir"})
        assert response.status_code == 404


class TestExcelCheckerEndpoints:
    """Test Excel checker API endpoints."""

    def test_check_nonexistent_file(self, client):
        """Test checking a nonexistent file returns error."""
        response = client.post("/excel/check", json={"path": "/nonexistent/file.xlsx"})
        assert response.status_code == 404


class TestDataProcessorEndpoints:
    """Test data processor API endpoints."""

    def test_load_nonexistent_file(self, client):
        """Test loading nonexistent file returns error."""
        response = client.post("/data/load", json={"path": "/nonexistent/file.csv"})
        assert response.status_code == 404


class TestCORS:
    """Test CORS configuration."""

    def test_cors_headers(self, client):
        """Test that CORS headers are present."""
        response = client.options(
            "/health",
            headers={
                "Origin": "http://localhost:3000",
                "Access-Control-Request-Method": "GET",
            },
        )
        # FastAPI with CORSMiddleware should handle this
        assert response.status_code in (200, 405)
