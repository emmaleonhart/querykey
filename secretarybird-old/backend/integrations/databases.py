"""
Database Connector Module

Supports PostgreSQL, MySQL, SQLite, and MongoDB with:
- Connection string builder with UI-friendly parameters
- Query execution returning pandas DataFrames
- Schema introspection (list tables, columns, types)
- Connection pooling via SQLAlchemy (for SQL databases)
- MongoDB query support via pymongo
"""

import logging
from enum import Enum
from typing import Any, Optional

import pandas as pd
from pydantic import BaseModel

logger = logging.getLogger("tojo.databases")


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

class DatabaseType(str, Enum):
    """Supported database types."""
    POSTGRESQL = "postgresql"
    MYSQL = "mysql"
    SQLITE = "sqlite"
    MONGODB = "mongodb"


class DatabaseConfig(BaseModel):
    """Database connection configuration with UI-friendly fields."""
    db_type: DatabaseType
    host: str = "localhost"
    port: Optional[int] = None
    database: str = ""
    username: Optional[str] = None
    password: Optional[str] = None
    # SQLite-specific
    file_path: Optional[str] = None
    # Advanced
    connection_string: Optional[str] = None  # overrides other fields
    pool_size: int = 5
    max_overflow: int = 10
    # MongoDB-specific
    auth_source: Optional[str] = None
    replica_set: Optional[str] = None


# ---------------------------------------------------------------------------
# Connection string builder
# ---------------------------------------------------------------------------

def build_connection_string(config: DatabaseConfig) -> str:
    """
    Build a database connection string from the config.

    If ``config.connection_string`` is set, it is returned as-is.

    Args:
        config: DatabaseConfig with connection parameters.

    Returns:
        Connection string / URI.
    """
    if config.connection_string:
        return config.connection_string

    db_type = config.db_type

    if db_type == DatabaseType.SQLITE:
        path = config.file_path or config.database or ":memory:"
        return f"sqlite:///{path}"

    # Default ports
    default_ports = {
        DatabaseType.POSTGRESQL: 5432,
        DatabaseType.MYSQL: 3306,
        DatabaseType.MONGODB: 27017,
    }
    port = config.port or default_ports.get(db_type, 5432)

    # Credential segment
    cred = ""
    if config.username:
        cred = config.username
        if config.password:
            cred += f":{config.password}"
        cred += "@"

    if db_type == DatabaseType.POSTGRESQL:
        return f"postgresql+psycopg2://{cred}{config.host}:{port}/{config.database}"
    elif db_type == DatabaseType.MYSQL:
        return f"mysql+pymysql://{cred}{config.host}:{port}/{config.database}"
    elif db_type == DatabaseType.MONGODB:
        uri = f"mongodb://{cred}{config.host}:{port}/{config.database}"
        params: list[str] = []
        if config.auth_source:
            params.append(f"authSource={config.auth_source}")
        if config.replica_set:
            params.append(f"replicaSet={config.replica_set}")
        if params:
            uri += "?" + "&".join(params)
        return uri
    else:
        raise ValueError(f"Unsupported database type: {db_type}")


# ---------------------------------------------------------------------------
# DatabaseConnector
# ---------------------------------------------------------------------------

class DatabaseConnector:
    """
    Unified database connector supporting SQL (PostgreSQL, MySQL, SQLite)
    and NoSQL (MongoDB) databases.
    """

    def __init__(self, config: DatabaseConfig):
        """
        Initialize the connector.

        Args:
            config: Database connection configuration.
        """
        self._config = config
        self._engine: Any = None  # SQLAlchemy engine (for SQL dbs)
        self._mongo_client: Any = None  # pymongo.MongoClient (for MongoDB)
        self._mongo_db: Any = None

    # ------------------------------------------------------------------
    # Connection management
    # ------------------------------------------------------------------
    def _get_engine(self) -> Any:
        """Return (and lazily create) a SQLAlchemy engine."""
        if self._config.db_type == DatabaseType.MONGODB:
            raise RuntimeError("Use _get_mongo_db() for MongoDB connections.")

        if self._engine is None:
            from sqlalchemy import create_engine

            conn_str = build_connection_string(self._config)
            if self._config.db_type == DatabaseType.SQLITE:
                from sqlalchemy.pool import StaticPool
                self._engine = create_engine(
                    conn_str,
                    connect_args={"check_same_thread": False},
                    poolclass=StaticPool,
                    pool_pre_ping=True,
                )
            else:
                self._engine = create_engine(
                    conn_str,
                    pool_size=self._config.pool_size,
                    max_overflow=self._config.max_overflow,
                    pool_pre_ping=True,
                )
            logger.info("Created SQLAlchemy engine for %s", self._config.db_type.value)

        return self._engine

    def _get_mongo_db(self) -> Any:
        """Return (and lazily create) a pymongo database handle."""
        if self._config.db_type != DatabaseType.MONGODB:
            raise RuntimeError("This method is only for MongoDB connections.")

        if self._mongo_client is None:
            try:
                from pymongo import MongoClient  # type: ignore[import-untyped]
            except ImportError as exc:
                raise RuntimeError("pymongo is not installed. Run: pip install pymongo") from exc

            conn_str = build_connection_string(self._config)
            self._mongo_client = MongoClient(conn_str)
            self._mongo_db = self._mongo_client[self._config.database]
            logger.info("Connected to MongoDB: %s", self._config.database)

        return self._mongo_db

    def close(self) -> None:
        """Close database connections and dispose of the engine pool."""
        if self._engine is not None:
            self._engine.dispose()
            self._engine = None
            logger.info("SQLAlchemy engine disposed.")
        if self._mongo_client is not None:
            self._mongo_client.close()
            self._mongo_client = None
            self._mongo_db = None
            logger.info("MongoDB client closed.")

    # ------------------------------------------------------------------
    # Test connection
    # ------------------------------------------------------------------
    def test_connection(self) -> bool:
        """
        Test whether the database is reachable.

        Returns:
            True if the connection succeeds, False otherwise.
        """
        try:
            if self._config.db_type == DatabaseType.MONGODB:
                db = self._get_mongo_db()
                db.command("ping")
            else:
                engine = self._get_engine()
                with engine.connect() as conn:
                    conn.execute(self._text("SELECT 1"))
            logger.info("Connection test successful for %s", self._config.db_type.value)
            return True
        except Exception as exc:
            logger.error("Connection test failed: %s", exc)
            return False

    # ------------------------------------------------------------------
    # Query execution
    # ------------------------------------------------------------------
    def execute_query(
        self,
        query: str,
        params: Optional[dict[str, Any]] = None,
    ) -> pd.DataFrame:
        """
        Execute a query and return results as a DataFrame.

        For SQL databases, this executes the SQL query.
        For MongoDB, ``query`` is interpreted as a JSON string representing
        a find filter on a collection. Use the format:
            {"collection": "name", "filter": {...}, "projection": {...}}

        Args:
            query: SQL query string or MongoDB query JSON.
            params: Optional bind parameters (SQL only).

        Returns:
            pandas DataFrame with query results.
        """
        if self._config.db_type == DatabaseType.MONGODB:
            return self._execute_mongo_query(query)
        else:
            return self._execute_sql_query(query, params)

    def _execute_sql_query(
        self,
        query: str,
        params: Optional[dict[str, Any]] = None,
    ) -> pd.DataFrame:
        """Execute a SQL query via SQLAlchemy."""
        engine = self._get_engine()
        sql = self._text(query)
        with engine.connect() as conn:
            result = conn.execute(sql, params or {})
            # For SELECT-like queries return a DataFrame
            if result.returns_rows:
                columns = list(result.keys())
                rows = result.fetchall()
                return pd.DataFrame(rows, columns=columns)
            else:
                conn.commit()
                return pd.DataFrame({"affected_rows": [result.rowcount]})

    def _execute_mongo_query(self, query_json: str) -> pd.DataFrame:
        """
        Execute a MongoDB find query.

        Expected JSON format:
            {"collection": "...", "filter": {...}, "projection": {...}, "limit": 1000}
        """
        import json

        db = self._get_mongo_db()
        try:
            spec = json.loads(query_json)
        except json.JSONDecodeError as exc:
            raise ValueError(f"Invalid MongoDB query JSON: {exc}") from exc

        collection_name = spec.get("collection")
        if not collection_name:
            raise ValueError("MongoDB query must include a 'collection' field.")

        collection = db[collection_name]
        cursor = collection.find(
            filter=spec.get("filter", {}),
            projection=spec.get("projection"),
        )
        limit = spec.get("limit", 10000)
        cursor = cursor.limit(limit)

        records = list(cursor)
        # Convert ObjectId to string for serialization
        for r in records:
            if "_id" in r:
                r["_id"] = str(r["_id"])

        return pd.DataFrame(records) if records else pd.DataFrame()

    # ------------------------------------------------------------------
    # Schema introspection
    # ------------------------------------------------------------------
    def get_schema(self) -> dict[str, Any]:
        """
        Retrieve database schema information.

        For SQL databases: lists tables and their columns with types.
        For MongoDB: lists collections and sample field names.

        Returns:
            Dictionary mapping table/collection names to column/field info.
        """
        if self._config.db_type == DatabaseType.MONGODB:
            return self._get_mongo_schema()
        else:
            return self._get_sql_schema()

    def _get_sql_schema(self) -> dict[str, Any]:
        """Introspect SQL database schema via SQLAlchemy."""
        from sqlalchemy import inspect as sa_inspect

        engine = self._get_engine()
        inspector = sa_inspect(engine)
        schema: dict[str, Any] = {}

        for table_name in inspector.get_table_names():
            columns = []
            for col in inspector.get_columns(table_name):
                columns.append({
                    "name": col["name"],
                    "type": str(col["type"]),
                    "nullable": col.get("nullable", True),
                    "default": str(col.get("default")) if col.get("default") is not None else None,
                    "primary_key": col.get("autoincrement", False) or False,
                })

            # Primary key info
            pk = inspector.get_pk_constraint(table_name)
            pk_columns = pk.get("constrained_columns", []) if pk else []

            # Foreign keys
            fks = inspector.get_foreign_keys(table_name)
            foreign_keys = [
                {
                    "columns": fk["constrained_columns"],
                    "referred_table": fk["referred_table"],
                    "referred_columns": fk["referred_columns"],
                }
                for fk in fks
            ]

            schema[table_name] = {
                "columns": columns,
                "primary_key": pk_columns,
                "foreign_keys": foreign_keys,
                "column_count": len(columns),
            }

        logger.info("Retrieved SQL schema: %d tables.", len(schema))
        return schema

    def _get_mongo_schema(self) -> dict[str, Any]:
        """Introspect MongoDB collections by sampling documents."""
        db = self._get_mongo_db()
        schema: dict[str, Any] = {}

        for collection_name in db.list_collection_names():
            collection = db[collection_name]
            sample = list(collection.find().limit(10))
            field_names: set[str] = set()
            field_types: dict[str, set[str]] = {}

            for doc in sample:
                for key, value in doc.items():
                    field_names.add(key)
                    field_types.setdefault(key, set()).add(type(value).__name__)

            schema[collection_name] = {
                "fields": [
                    {
                        "name": name,
                        "sample_types": list(field_types.get(name, set())),
                    }
                    for name in sorted(field_names)
                ],
                "document_count": collection.estimated_document_count(),
                "field_count": len(field_names),
            }

        logger.info("Retrieved MongoDB schema: %d collections.", len(schema))
        return schema

    def list_tables(self) -> list[str]:
        """Return a flat list of table (SQL) or collection (MongoDB) names."""
        if self._config.db_type == DatabaseType.MONGODB:
            db = self._get_mongo_db()
            return db.list_collection_names()
        else:
            from sqlalchemy import inspect as sa_inspect

            engine = self._get_engine()
            inspector = sa_inspect(engine)
            return inspector.get_table_names()

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------
    @staticmethod
    def _text(sql: str) -> Any:
        """Wrap a raw SQL string in SQLAlchemy's text() construct."""
        from sqlalchemy import text

        return text(sql)
