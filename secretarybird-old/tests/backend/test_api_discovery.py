"""Tests for the API discovery module."""
import json
import pytest

from backend.integrations.api_discovery import (
    APIDiscovery,
    APIConfig,
    APIEndpoint,
    AuthType,
)


@pytest.fixture
def discovery():
    """Create an APIDiscovery instance."""
    return APIDiscovery()


class TestAuthType:
    """Test AuthType enum."""

    def test_auth_types_exist(self):
        """Test that expected auth types exist."""
        assert AuthType.NONE is not None
        assert AuthType.API_KEY is not None
        assert AuthType.BEARER is not None
        assert AuthType.OAUTH2 is not None
        assert AuthType.BASIC is not None


class TestAPIEndpoint:
    """Test APIEndpoint dataclass."""

    def test_create_endpoint(self):
        """Test creating an API endpoint."""
        endpoint = APIEndpoint(
            path="/users",
            method="GET",
            description="List all users",
            parameters=[{"name": "limit", "type": "integer", "required": False}],
        )
        assert endpoint.path == "/users"
        assert endpoint.method == "GET"

    def test_endpoint_to_dict(self):
        """Test converting endpoint to dictionary."""
        endpoint = APIEndpoint(
            path="/users/{id}",
            method="GET",
            description="Get user by ID",
            parameters=[{"name": "id", "type": "string", "required": True}],
        )
        d = endpoint.to_dict()
        assert d["path"] == "/users/{id}"
        assert d["method"] == "GET"
        assert len(d["parameters"]) == 1


class TestAPIConfig:
    """Test APIConfig."""

    def test_create_config(self):
        """Test creating an API config."""
        config = APIConfig(
            name="Test API",
            base_url="https://api.example.com",
            auth_type=AuthType.API_KEY,
            auth_config={"header": "X-API-Key", "key": "test123"},
        )
        assert config.name == "Test API"
        assert config.base_url == "https://api.example.com"
        assert config.auth_type == AuthType.API_KEY

    def test_config_to_dict(self):
        """Test converting config to dictionary."""
        config = APIConfig(
            name="Test API",
            base_url="https://api.example.com",
            auth_type=AuthType.NONE,
        )
        d = config.to_dict()
        assert d["name"] == "Test API"
        assert d["auth_type"] == "none"

    def test_config_from_dict(self):
        """Test creating config from dictionary."""
        d = {
            "name": "Test API",
            "base_url": "https://api.example.com",
            "auth_type": "bearer",
            "auth_config": {"token": "abc123"},
        }
        config = APIConfig.from_dict(d)
        assert config.name == "Test API"
        assert config.auth_type == AuthType.BEARER

    def test_config_serialization_roundtrip(self):
        """Test that config survives JSON roundtrip."""
        original = APIConfig(
            name="Roundtrip Test",
            base_url="https://api.example.com",
            auth_type=AuthType.BASIC,
            auth_config={"username": "user", "password": "pass"},
            endpoints=[
                APIEndpoint("/health", "GET", "Health check", []),
            ],
        )
        json_str = json.dumps(original.to_dict())
        restored = APIConfig.from_dict(json.loads(json_str))
        assert restored.name == original.name
        assert restored.auth_type == original.auth_type
        assert len(restored.endpoints) == 1


class TestAPIDiscovery:
    """Test APIDiscovery functionality."""

    def test_init(self, discovery):
        """Test discovery initialization."""
        assert discovery is not None

    def test_store_api_config(self, discovery):
        """Test storing an API configuration."""
        config = APIConfig(
            name="Stored API",
            base_url="https://api.example.com",
            auth_type=AuthType.NONE,
        )
        discovery.store_config(config)
        assert "Stored API" in discovery.configs

    def test_get_stored_config(self, discovery):
        """Test retrieving a stored config."""
        config = APIConfig(
            name="Retrieval Test",
            base_url="https://api.example.com",
            auth_type=AuthType.NONE,
        )
        discovery.store_config(config)
        retrieved = discovery.get_config("Retrieval Test")
        assert retrieved is not None
        assert retrieved.base_url == "https://api.example.com"

    def test_get_nonexistent_config(self, discovery):
        """Test retrieving a nonexistent config returns None."""
        result = discovery.get_config("nonexistent")
        assert result is None

    def test_list_configs(self, discovery):
        """Test listing all stored configs."""
        discovery.store_config(APIConfig("API 1", "https://api1.com", AuthType.NONE))
        discovery.store_config(APIConfig("API 2", "https://api2.com", AuthType.NONE))
        configs = discovery.list_configs()
        assert len(configs) == 2

    def test_remove_config(self, discovery):
        """Test removing a stored config."""
        discovery.store_config(APIConfig("To Remove", "https://api.com", AuthType.NONE))
        discovery.remove_config("To Remove")
        assert discovery.get_config("To Remove") is None

    def test_save_and_load_configs(self, discovery, tmp_path):
        """Test saving and loading configs to/from file."""
        filepath = str(tmp_path / "apis.json")
        discovery.store_config(APIConfig("Saved API", "https://api.com", AuthType.API_KEY,
                                         auth_config={"key": "test"}))
        discovery.save_configs(filepath)

        new_discovery = APIDiscovery()
        new_discovery.load_configs(filepath)
        assert "Saved API" in new_discovery.configs
