"""
Salesforce Integration Connector

Provides:
- Connection management via simple-salesforce
- SOQL query execution with DataFrame export
- Record CRUD (create, update, delete)
- Object metadata / describe
- Credential storage helpers
"""

import json
import logging
from pathlib import Path
from typing import Any, Optional

import pandas as pd
from pydantic import BaseModel

logger = logging.getLogger("tojo.salesforce")

# Default path for persisted credentials
_CRED_DIR = Path.home() / ".tojo" / "credentials"


# ---------------------------------------------------------------------------
# Configuration model
# ---------------------------------------------------------------------------

class SalesforceConfig(BaseModel):
    """Salesforce connection configuration."""
    username: str
    password: str
    security_token: str = ""
    domain: str = "login"  # "login" for production, "test" for sandbox
    instance_url: Optional[str] = None
    client_id: Optional[str] = None
    # If set, the connector will look up stored credentials by this name
    credential_name: Optional[str] = None


# ---------------------------------------------------------------------------
# Connector
# ---------------------------------------------------------------------------

class SalesforceConnector:
    """
    Manages a Salesforce connection and provides high-level operations.
    """

    def __init__(self, config: Optional[SalesforceConfig] = None):
        """
        Initialize the connector.

        Args:
            config: Salesforce connection configuration. Can be omitted if
                    loading saved credentials later via ``load_credentials``.
        """
        self._config = config
        self._sf: Any = None  # simple_salesforce.Salesforce instance

    # ------------------------------------------------------------------
    # Connection management
    # ------------------------------------------------------------------
    def connect(self) -> None:
        """
        Establish a connection to Salesforce.

        Raises:
            RuntimeError: If no config is available or authentication fails.
        """
        if self._config is None:
            raise RuntimeError("No Salesforce configuration provided.")

        try:
            from simple_salesforce import Salesforce  # type: ignore[import-untyped]
        except ImportError as exc:
            raise RuntimeError(
                "simple-salesforce is not installed. "
                "Run: pip install simple-salesforce"
            ) from exc

        connect_kwargs: dict[str, Any] = {
            "username": self._config.username,
            "password": self._config.password,
            "security_token": self._config.security_token,
            "domain": self._config.domain,
        }
        if self._config.instance_url:
            connect_kwargs["instance_url"] = self._config.instance_url
        if self._config.client_id:
            connect_kwargs["client_id"] = self._config.client_id

        self._sf = Salesforce(**connect_kwargs)
        logger.info(
            "Connected to Salesforce (%s) as %s",
            self._sf.sf_instance,
            self._config.username,
        )

    @property
    def connected(self) -> bool:
        """Check whether a connection is active."""
        return self._sf is not None

    def disconnect(self) -> None:
        """Close the Salesforce session."""
        if self._sf is not None:
            try:
                self._sf.session.close()
            except Exception:
                pass
            self._sf = None
            logger.info("Disconnected from Salesforce.")

    def _ensure_connected(self) -> Any:
        """Return the Salesforce client, raising if not connected."""
        if self._sf is None:
            raise RuntimeError("Not connected. Call connect() first.")
        return self._sf

    # ------------------------------------------------------------------
    # Querying
    # ------------------------------------------------------------------
    def query(self, soql: str) -> list[dict[str, Any]]:
        """
        Execute a SOQL query and return the raw records.

        Handles automatic pagination (queryMore) for large result sets.

        Args:
            soql: SOQL query string.

        Returns:
            List of record dictionaries.
        """
        sf = self._ensure_connected()
        result = sf.query(soql)
        records: list[dict[str, Any]] = result.get("records", [])

        while not result.get("done", True):
            result = sf.query_more(result["nextRecordsUrl"], identifier_is_url=True)
            records.extend(result.get("records", []))

        # Strip Salesforce metadata from each record
        for r in records:
            r.pop("attributes", None)

        logger.info("SOQL query returned %d records.", len(records))
        return records

    def query_to_dataframe(self, soql: str) -> pd.DataFrame:
        """
        Execute a SOQL query and return a pandas DataFrame.

        Args:
            soql: SOQL query string.

        Returns:
            pandas DataFrame with query results.
        """
        records = self.query(soql)
        if not records:
            return pd.DataFrame()
        return pd.json_normalize(records)

    # ------------------------------------------------------------------
    # Record operations
    # ------------------------------------------------------------------
    def create_record(self, object_name: str, data: dict[str, Any]) -> dict[str, Any]:
        """
        Create a new record.

        Args:
            object_name: Salesforce object API name (e.g. "Account").
            data: Field values for the new record.

        Returns:
            Result dictionary with 'id', 'success', etc.
        """
        sf = self._ensure_connected()
        sobject = getattr(sf, object_name)
        result = sobject.create(data)
        logger.info("Created %s record: %s", object_name, result.get("id"))
        return result

    def update_record(
        self,
        object_name: str,
        record_id: str,
        data: dict[str, Any],
    ) -> dict[str, Any]:
        """
        Update an existing record.

        Args:
            object_name: Salesforce object API name.
            record_id: The 15- or 18-character record ID.
            data: Field values to update.

        Returns:
            HTTP status code (204 indicates success).
        """
        sf = self._ensure_connected()
        sobject = getattr(sf, object_name)
        result = sobject.update(record_id, data)
        logger.info("Updated %s record %s", object_name, record_id)
        return {"status": result, "id": record_id}

    def delete_record(self, object_name: str, record_id: str) -> dict[str, Any]:
        """
        Delete a record.

        Args:
            object_name: Salesforce object API name.
            record_id: The 15- or 18-character record ID.

        Returns:
            HTTP status code (204 indicates success).
        """
        sf = self._ensure_connected()
        sobject = getattr(sf, object_name)
        result = sobject.delete(record_id)
        logger.info("Deleted %s record %s", object_name, record_id)
        return {"status": result, "id": record_id}

    # ------------------------------------------------------------------
    # Metadata
    # ------------------------------------------------------------------
    def describe_object(self, object_name: str) -> dict[str, Any]:
        """
        Get metadata (describe) for a Salesforce object.

        Args:
            object_name: Salesforce object API name (e.g. "Account").

        Returns:
            Dictionary with fields, label, relationships, etc.
        """
        sf = self._ensure_connected()
        sobject = getattr(sf, object_name)
        desc = sobject.describe()

        # Extract the most useful parts
        fields = [
            {
                "name": f["name"],
                "label": f["label"],
                "type": f["type"],
                "length": f.get("length"),
                "nillable": f.get("nillable"),
                "updateable": f.get("updateable"),
                "createable": f.get("createable"),
                "picklistValues": [
                    {"label": pv["label"], "value": pv["value"], "active": pv["active"]}
                    for pv in f.get("picklistValues", [])
                ] if f.get("picklistValues") else [],
            }
            for f in desc.get("fields", [])
        ]

        return {
            "name": desc.get("name"),
            "label": desc.get("label"),
            "labelPlural": desc.get("labelPlural"),
            "keyPrefix": desc.get("keyPrefix"),
            "queryable": desc.get("queryable"),
            "createable": desc.get("createable"),
            "updateable": desc.get("updateable"),
            "deletable": desc.get("deletable"),
            "field_count": len(fields),
            "fields": fields,
        }

    def list_objects(self) -> list[dict[str, str]]:
        """
        List available Salesforce objects.

        Returns:
            List of dictionaries with 'name' and 'label' keys.
        """
        sf = self._ensure_connected()
        result = sf.describe()
        return [
            {"name": obj["name"], "label": obj["label"]}
            for obj in result.get("sobjects", [])
        ]

    # ------------------------------------------------------------------
    # Credential storage
    # ------------------------------------------------------------------
    def save_credentials(self, name: str) -> Path:
        """
        Save current configuration to disk (encrypted storage is recommended
        for production but out of scope for the hackathon).

        Args:
            name: Friendly name for this credential set.

        Returns:
            Path to the saved credential file.
        """
        if self._config is None:
            raise RuntimeError("No configuration to save.")

        _CRED_DIR.mkdir(parents=True, exist_ok=True)
        cred_path = _CRED_DIR / f"salesforce_{name}.json"
        cred_path.write_text(
            self._config.model_dump_json(indent=2),
            encoding="utf-8",
        )
        logger.info("Saved Salesforce credentials to %s", cred_path)
        return cred_path

    @classmethod
    def load_credentials(cls, name: str) -> "SalesforceConnector":
        """
        Load saved credentials and return a new connector.

        Args:
            name: Friendly name used when saving.

        Returns:
            A new SalesforceConnector with the loaded config.
        """
        cred_path = _CRED_DIR / f"salesforce_{name}.json"
        if not cred_path.is_file():
            raise FileNotFoundError(f"No saved credentials found: {cred_path}")

        data = json.loads(cred_path.read_text(encoding="utf-8"))
        config = SalesforceConfig(**data)
        logger.info("Loaded Salesforce credentials from %s", cred_path)
        return cls(config)
