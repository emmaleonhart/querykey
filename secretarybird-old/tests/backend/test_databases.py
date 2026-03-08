"""Tests for the database connector module."""
import os
import pytest
import pandas as pd

from backend.integrations.databases import (
    DatabaseConnector,
    DatabaseConfig,
    DatabaseType,
    build_connection_string,
)


class TestDatabaseConfig:
    """Test DatabaseConfig creation and connection string building."""

    def test_sqlite_config(self):
        """Test creating SQLite connection config."""
        config = DatabaseConfig(
            db_type=DatabaseType.SQLITE,
            database="/path/to/db.sqlite",
        )
        assert config.db_type == DatabaseType.SQLITE
        conn_str = build_connection_string(config)
        assert "sqlite" in conn_str

    def test_postgres_config(self):
        """Test creating PostgreSQL connection config."""
        config = DatabaseConfig(
            db_type=DatabaseType.POSTGRESQL,
            host="localhost",
            port=5432,
            database="mydb",
            username="user",
            password="pass",
        )
        assert config.db_type == DatabaseType.POSTGRESQL
        conn_str = build_connection_string(config)
        assert "postgresql" in conn_str

    def test_mysql_config(self):
        """Test creating MySQL connection config."""
        config = DatabaseConfig(
            db_type=DatabaseType.MYSQL,
            host="localhost",
            port=3306,
            database="mydb",
            username="user",
            password="pass",
        )
        assert config.db_type == DatabaseType.MYSQL
        conn_str = build_connection_string(config)
        assert "mysql" in conn_str

    def test_custom_connection_string(self):
        """Test that explicit connection_string overrides other fields."""
        config = DatabaseConfig(
            db_type=DatabaseType.POSTGRESQL,
            connection_string="postgresql://custom@host/db",
        )
        conn_str = build_connection_string(config)
        assert conn_str == "postgresql://custom@host/db"


class TestDatabaseConnector:
    """Test DatabaseConnector with SQLite."""

    @pytest.fixture
    def sqlite_db(self, tmp_path):
        """Create a temporary SQLite database for testing."""
        import sqlite3
        db_path = str(tmp_path / "test.db")
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("""
            CREATE TABLE employees (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                department TEXT,
                salary REAL
            )
        """)
        cursor.executemany(
            "INSERT INTO employees (name, department, salary) VALUES (?, ?, ?)",
            [
                ("Alice", "Engineering", 75000),
                ("Bob", "Sales", 65000),
                ("Charlie", "Engineering", 80000),
                ("Diana", "Marketing", 60000),
            ],
        )
        conn.commit()
        conn.close()
        return db_path

    def test_init(self, sqlite_db):
        """Test connector initialization."""
        config = DatabaseConfig(db_type=DatabaseType.SQLITE, file_path=sqlite_db)
        connector = DatabaseConnector(config)
        assert connector is not None

    def test_test_connection(self, sqlite_db):
        """Test connection test with SQLite."""
        config = DatabaseConfig(db_type=DatabaseType.SQLITE, file_path=sqlite_db)
        connector = DatabaseConnector(config)
        assert connector.test_connection() is True

    def test_query_to_dataframe(self, sqlite_db):
        """Test querying data and returning a DataFrame."""
        config = DatabaseConfig(db_type=DatabaseType.SQLITE, file_path=sqlite_db)
        connector = DatabaseConnector(config)
        df = connector.execute_query("SELECT * FROM employees")
        assert isinstance(df, pd.DataFrame)
        assert len(df) == 4
        assert "name" in df.columns
        connector.close()

    def test_query_with_filter(self, sqlite_db):
        """Test querying with WHERE clause."""
        config = DatabaseConfig(db_type=DatabaseType.SQLITE, file_path=sqlite_db)
        connector = DatabaseConnector(config)
        df = connector.execute_query(
            "SELECT * FROM employees WHERE department = 'Engineering'"
        )
        assert len(df) == 2
        connector.close()

    def test_list_tables(self, sqlite_db):
        """Test listing tables in database."""
        config = DatabaseConfig(db_type=DatabaseType.SQLITE, file_path=sqlite_db)
        connector = DatabaseConnector(config)
        tables = connector.list_tables()
        assert "employees" in tables
        connector.close()

    def test_get_schema(self, sqlite_db):
        """Test getting database schema."""
        config = DatabaseConfig(db_type=DatabaseType.SQLITE, file_path=sqlite_db)
        connector = DatabaseConnector(config)
        schema = connector.get_schema()
        assert "employees" in schema
        assert "columns" in schema["employees"]
        col_names = [c["name"] for c in schema["employees"]["columns"]]
        assert "id" in col_names
        assert "name" in col_names
        assert "salary" in col_names
        connector.close()

    def test_insert_and_query(self, sqlite_db):
        """Test executing an insert statement then querying."""
        config = DatabaseConfig(db_type=DatabaseType.SQLITE, file_path=sqlite_db)
        connector = DatabaseConnector(config)
        connector.execute_query(
            "INSERT INTO employees (name, department, salary) VALUES ('Eve', 'Sales', 70000)"
        )
        df = connector.execute_query("SELECT * FROM employees")
        assert len(df) == 5
        connector.close()

    def test_close(self, sqlite_db):
        """Test disconnecting from database."""
        config = DatabaseConfig(db_type=DatabaseType.SQLITE, file_path=sqlite_db)
        connector = DatabaseConnector(config)
        connector.test_connection()
        connector.close()
        # After close, engine is disposed
        assert connector._engine is None
