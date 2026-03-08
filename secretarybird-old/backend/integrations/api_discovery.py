"""
API Discovery Module

Provides:
- Dynamic API endpoint discovery from OpenAPI/Swagger specs
- API configuration management (store, load, serialize)
- Endpoint testing with configurable auth
- Support for API key, Bearer token, OAuth2, and Basic auth
"""

import json
import logging
from enum import Enum
from pathlib import Path
from typing import Any, Optional

logger = logging.getLogger("tojo.api_discovery")


# ---------------------------------------------------------------------------
# Auth types
# ---------------------------------------------------------------------------

class AuthType(str, Enum):
    """Supported authentication types."""
    NONE = "none"
    API_KEY = "api_key"
    BEARER = "bearer"
    OAUTH2 = "oauth2"
    BASIC = "basic"


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------

class APIEndpoint:
    """Represents a single API endpoint."""

    def __init__(
        self,
        path: str,
        method: str,
        description: str = "",
        parameters: Optional[list[dict[str, Any]]] = None,
    ):
        self.path = path
        self.method = method.upper()
        self.description = description
        self.parameters = parameters or []

    def to_dict(self) -> dict[str, Any]:
        return {
            "path": self.path,
            "method": self.method,
            "description": self.description,
            "parameters": self.parameters,
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "APIEndpoint":
        return cls(
            path=d["path"],
            method=d["method"],
            description=d.get("description", ""),
            parameters=d.get("parameters", []),
        )


class APIConfig:
    """Configuration for an API, including auth and discovered endpoints."""

    def __init__(
        self,
        name: str,
        base_url: str,
        auth_type: AuthType = AuthType.NONE,
        auth_config: Optional[dict[str, str]] = None,
        endpoints: Optional[list[APIEndpoint]] = None,
    ):
        self.name = name
        self.base_url = base_url.rstrip("/")
        self.auth_type = auth_type
        self.auth_config = auth_config or {}
        self.endpoints = endpoints or []

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "base_url": self.base_url,
            "auth_type": self.auth_type.value,
            "auth_config": self.auth_config,
            "endpoints": [ep.to_dict() for ep in self.endpoints],
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "APIConfig":
        auth_type_str = d.get("auth_type", "none")
        auth_type = AuthType(auth_type_str)
        endpoints = [
            APIEndpoint.from_dict(ep) for ep in d.get("endpoints", [])
        ]
        return cls(
            name=d["name"],
            base_url=d["base_url"],
            auth_type=auth_type,
            auth_config=d.get("auth_config", {}),
            endpoints=endpoints,
        )


# ---------------------------------------------------------------------------
# APIDiscovery
# ---------------------------------------------------------------------------

class APIDiscovery:
    """
    Discovers, stores, and manages API configurations.
    Can parse OpenAPI/Swagger specs and test endpoints.
    """

    def __init__(self):
        self.configs: dict[str, APIConfig] = {}

    # ------------------------------------------------------------------
    # Config management
    # ------------------------------------------------------------------
    def store_config(self, config: APIConfig) -> None:
        """Store an API configuration by name."""
        self.configs[config.name] = config
        logger.info("Stored API config: %s", config.name)

    def get_config(self, name: str) -> Optional[APIConfig]:
        """Retrieve a stored config by name."""
        return self.configs.get(name)

    def list_configs(self) -> list[APIConfig]:
        """List all stored API configurations."""
        return list(self.configs.values())

    def remove_config(self, name: str) -> None:
        """Remove a stored config by name."""
        self.configs.pop(name, None)
        logger.info("Removed API config: %s", name)

    # ------------------------------------------------------------------
    # Persistence
    # ------------------------------------------------------------------
    def save_configs(self, filepath: str) -> None:
        """Save all configs to a JSON file."""
        path = Path(filepath)
        path.parent.mkdir(parents=True, exist_ok=True)
        data = {name: config.to_dict() for name, config in self.configs.items()}
        path.write_text(json.dumps(data, indent=2), encoding="utf-8")
        logger.info("Saved %d API configs to %s", len(data), filepath)

    def load_configs(self, filepath: str) -> None:
        """Load configs from a JSON file."""
        path = Path(filepath)
        if not path.is_file():
            raise FileNotFoundError(f"Config file not found: {filepath}")
        data = json.loads(path.read_text(encoding="utf-8"))
        for name, config_dict in data.items():
            self.configs[name] = APIConfig.from_dict(config_dict)
        logger.info("Loaded %d API configs from %s", len(data), filepath)

    # ------------------------------------------------------------------
    # Discovery
    # ------------------------------------------------------------------
    async def discover(self, url: str) -> dict[str, Any]:
        """
        Discover API endpoints from an OpenAPI/Swagger specification URL.

        Args:
            url: URL pointing to an OpenAPI spec (JSON or YAML).

        Returns:
            Dictionary with discovered endpoints and metadata.
        """
        import httpx

        async with httpx.AsyncClient(timeout=30.0) as client:
            response = await client.get(url)
            response.raise_for_status()

        try:
            spec = response.json()
        except Exception:
            import yaml
            spec = yaml.safe_load(response.text)

        endpoints: list[dict[str, Any]] = []
        paths = spec.get("paths", {})

        for path, methods in paths.items():
            for method, details in methods.items():
                if method.upper() in ("GET", "POST", "PUT", "PATCH", "DELETE"):
                    params = []
                    for p in details.get("parameters", []):
                        params.append({
                            "name": p.get("name"),
                            "type": p.get("schema", {}).get("type", "string"),
                            "required": p.get("required", False),
                            "in": p.get("in", "query"),
                        })
                    endpoints.append({
                        "path": path,
                        "method": method.upper(),
                        "description": details.get("summary", details.get("description", "")),
                        "parameters": params,
                    })

        info = spec.get("info", {})
        base_url = ""
        servers = spec.get("servers", [])
        if servers:
            base_url = servers[0].get("url", "")

        result = {
            "title": info.get("title", "Unknown API"),
            "version": info.get("version", ""),
            "description": info.get("description", ""),
            "base_url": base_url,
            "endpoint_count": len(endpoints),
            "endpoints": endpoints,
        }

        logger.info("Discovered %d endpoints from %s", len(endpoints), url)
        return result

    # ------------------------------------------------------------------
    # Endpoint testing
    # ------------------------------------------------------------------
    async def test_endpoint(
        self,
        url: str,
        method: str = "GET",
        headers: Optional[dict[str, str]] = None,
        body: Any = None,
        auth_type: Optional[str] = None,
        auth_credentials: Optional[dict[str, str]] = None,
    ) -> dict[str, Any]:
        """
        Test a specific API endpoint.

        Args:
            url: Full URL of the endpoint.
            method: HTTP method.
            headers: Optional request headers.
            body: Optional request body.
            auth_type: Authentication type.
            auth_credentials: Authentication credentials.

        Returns:
            Dictionary with status code, headers, and body preview.
        """
        import httpx

        req_headers = dict(headers or {})

        # Apply auth
        if auth_type and auth_credentials:
            if auth_type == "bearer":
                req_headers["Authorization"] = f"Bearer {auth_credentials.get('token', '')}"
            elif auth_type == "api_key":
                header_name = auth_credentials.get("header", "X-API-Key")
                req_headers[header_name] = auth_credentials.get("key", "")
            elif auth_type == "basic":
                import base64
                cred_str = f"{auth_credentials.get('username', '')}:{auth_credentials.get('password', '')}"
                encoded = base64.b64encode(cred_str.encode()).decode()
                req_headers["Authorization"] = f"Basic {encoded}"

        async with httpx.AsyncClient(timeout=30.0) as client:
            response = await client.request(
                method=method.upper(),
                url=url,
                headers=req_headers,
                json=body if body and method.upper() != "GET" else None,
            )

        # Build response summary
        try:
            response_body = response.json()
        except Exception:
            response_body = response.text[:2000]

        return {
            "status_code": response.status_code,
            "headers": dict(response.headers),
            "body": response_body,
            "elapsed_ms": int(response.elapsed.total_seconds() * 1000),
            "url": str(response.url),
        }
