"""
Tojo Assistant - FastAPI Backend Server

Main server module providing:
- WebSocket endpoint for streaming chat interactions
- REST endpoints for file organization, Excel checking, data processing,
  integrations (Salesforce, Google Suite, Databases, APIs), and pipelines
- CORS middleware configured for Electron desktop client
- Health check and lifecycle management
"""

import asyncio
import json
import logging
import traceback
from contextlib import asynccontextmanager
from datetime import datetime
from pathlib import Path
from typing import Any, Optional

from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException, UploadFile, File, Query
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, FileResponse
from pydantic import BaseModel, Field

from backend.core.file_organizer import FileOrganizer, OrganizeRequest
from backend.core.excel_checker import ExcelChecker
from backend.core.data_processor import DataProcessor, TransformRequest, ProfileRequest
from backend.integrations.salesforce import SalesforceConnector, SalesforceConfig
from backend.integrations.google_suite import GoogleSuiteConnector, GoogleSheetsConfig
from backend.integrations.databases import DatabaseConnector, DatabaseConfig
from backend.integrations.api_discovery import APIDiscovery, APIConfig
from backend.pipeline.builder import PipelineBuilder, PipelineDefinition
from backend.openclaw.bridge import OpenClawBridge

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("tojo.server")

# ---------------------------------------------------------------------------
# Shared state (lives for the lifetime of the process)
# ---------------------------------------------------------------------------
_state: dict[str, Any] = {}


# ---------------------------------------------------------------------------
# Lifespan
# ---------------------------------------------------------------------------
@asynccontextmanager
async def lifespan(app: FastAPI):
    """Startup and shutdown lifecycle for the FastAPI application."""
    logger.info("Tojo Assistant backend starting up...")
    _state["start_time"] = datetime.utcnow().isoformat()
    _state["data_processor"] = DataProcessor()
    _state["pipeline_builder"] = PipelineBuilder()
    _state["api_discovery"] = APIDiscovery()
    _state["openclaw_bridge"] = OpenClawBridge()
    _state["active_websockets"] = set()

    # Detect OpenClaw availability at startup
    openclaw_status = _state["openclaw_bridge"].detect()
    _state["openclaw_available"] = openclaw_status["available"]
    if openclaw_status["available"]:
        logger.info("OpenClaw detected: %s (WSL: %s)", openclaw_status["command"], openclaw_status["via_wsl"])
    else:
        logger.warning("OpenClaw not found. Chat will use built-in handlers only.")
    logger.info("Backend ready.")
    yield
    logger.info("Tojo Assistant backend shutting down...")
    # Close any active WebSocket connections
    for ws in list(_state.get("active_websockets", set())):
        try:
            await ws.close()
        except Exception:
            pass
    _state.clear()
    logger.info("Backend shutdown complete.")


# ---------------------------------------------------------------------------
# Application
# ---------------------------------------------------------------------------
app = FastAPI(
    title="Tojo Assistant",
    description="Business data assistant backend",
    version="0.1.0",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=[
        "http://localhost:3000",
        "http://localhost:5173",
        "http://localhost:8080",
        "app://.",                 # Electron custom protocol
        "file://",                 # Electron file protocol
    ],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# ---------------------------------------------------------------------------
# Pydantic request/response models
# ---------------------------------------------------------------------------
class HealthResponse(BaseModel):
    """Health check response."""
    status: str = "ok"
    uptime_since: Optional[str] = None
    version: str = "0.1.0"


class ErrorResponse(BaseModel):
    """Standard error wrapper."""
    error: str
    detail: Optional[str] = None


class FileOrgRequest(BaseModel):
    """Request body for file organization endpoints."""
    directory: str
    dry_run: bool = True
    target_directory: Optional[str] = None


class ExcelCheckRequest(BaseModel):
    """Request body for Excel checking (when sending a path rather than upload)."""
    file_path: str


class QueryRequest(BaseModel):
    """Generic query request for integrations."""
    query: str
    params: Optional[dict[str, Any]] = None


class SalesforceQueryRequest(BaseModel):
    """Salesforce SOQL query request."""
    config: SalesforceConfig
    soql: str


class SalesforceRecordRequest(BaseModel):
    """Salesforce record create/update/delete request."""
    config: SalesforceConfig
    object_name: str
    action: str  # "create" | "update" | "delete"
    record_id: Optional[str] = None
    data: Optional[dict[str, Any]] = None


class DatabaseQueryRequest(BaseModel):
    """Database query request."""
    config: DatabaseConfig
    query: str
    params: Optional[dict[str, Any]] = None


class APIDiscoverRequest(BaseModel):
    """API discovery request."""
    url: str
    auth_type: Optional[str] = None
    auth_credentials: Optional[dict[str, str]] = None


class APITestRequest(BaseModel):
    """API endpoint test request."""
    url: str
    method: str = "GET"
    headers: Optional[dict[str, str]] = None
    body: Optional[Any] = None
    auth_type: Optional[str] = None
    auth_credentials: Optional[dict[str, str]] = None


class PipelineRunRequest(BaseModel):
    """Pipeline execution request."""
    definition: dict[str, Any]


class ChatMessage(BaseModel):
    """A single chat message for WebSocket communication."""
    role: str  # "user" | "assistant" | "system"
    content: str
    metadata: Optional[dict[str, Any]] = None


# ---------------------------------------------------------------------------
# Health
# ---------------------------------------------------------------------------
@app.get("/health", response_model=HealthResponse, tags=["system"])
async def health_check() -> HealthResponse:
    """Return server health status and uptime."""
    return HealthResponse(
        status="ok",
        uptime_since=_state.get("start_time"),
    )


@app.get("/api/openclaw/status", tags=["openclaw"])
async def openclaw_status() -> JSONResponse:
    """Check OpenClaw CLI availability."""
    bridge: OpenClawBridge = _state["openclaw_bridge"]
    status = bridge.detect()
    return JSONResponse(content=status)


# ---------------------------------------------------------------------------
# WebSocket - Streaming Chat
# ---------------------------------------------------------------------------
@app.websocket("/ws/chat")
async def websocket_chat(ws: WebSocket):
    """
    WebSocket endpoint for streaming chat interactions.

    Protocol:
    - Client sends JSON: {"message": "...", "context": {...}}
    - Server streams back JSON chunks: {"type": "chunk"|"done"|"error", "content": "..."}
    """
    await ws.accept()
    _state.setdefault("active_websockets", set()).add(ws)
    logger.info("WebSocket client connected.")

    # Notify client of OpenClaw status on connect
    await ws.send_json({
        "type": "status",
        "openclaw": _state.get("openclaw_available", False),
    })

    try:
        while True:
            raw = await ws.receive_text()
            try:
                payload = json.loads(raw)
            except json.JSONDecodeError:
                await ws.send_json({"type": "error", "content": "Invalid JSON"})
                continue

            user_message = payload.get("content", payload.get("message", ""))
            context = payload.get("context", {})

            # Route the message to the appropriate handler based on context
            handler = context.get("handler", "default")

            try:
                if handler == "file_organizer":
                    result = await _handle_file_org_chat(user_message, context)
                elif handler == "excel_checker":
                    result = await _handle_excel_chat(user_message, context)
                elif handler == "data_processor":
                    result = await _handle_data_chat(user_message, context)
                elif handler == "pipeline":
                    result = await _handle_pipeline_chat(user_message, context)
                else:
                    result = _handle_default_chat(user_message)

                # Stream the response in chunks to simulate real-time output
                chunk_size = 80
                text = result if isinstance(result, str) else json.dumps(result)

                await ws.send_json({"type": "stream_start"})

                for i in range(0, len(text), chunk_size):
                    await ws.send_json({
                        "type": "stream_chunk",
                        "content": text[i : i + chunk_size],
                    })
                    await asyncio.sleep(0.02)  # Small delay for streaming feel

                await ws.send_json({"type": "stream_end"})

            except Exception as exc:
                logger.exception("Error processing chat message")
                await ws.send_json({
                    "type": "error",
                    "message": str(exc),
                })

    except WebSocketDisconnect:
        logger.info("WebSocket client disconnected.")
    finally:
        _state.get("active_websockets", set()).discard(ws)


def _handle_default_chat(message: str) -> str:
    """
    Default chat handler.

    If OpenClaw is available, routes the message through it.
    Otherwise, responds with capability hints.
    """
    bridge: OpenClawBridge = _state.get("openclaw_bridge")
    if bridge and _state.get("openclaw_available"):
        try:
            session_id = bridge.start(message)
            # Wait for the process to finish and collect output
            if bridge.process:
                stdout, stderr = bridge.process.communicate(timeout=120)
                output = stdout.strip() if stdout else ""
                if output:
                    bridge._output_buffer.append(output)
                    return output
                # If no stdout, check stderr for useful info
                if stderr and stderr.strip():
                    return f"OpenClaw: {stderr.strip()}"
        except Exception as exc:
            logger.warning("OpenClaw failed, falling back to built-in handler: %s", exc)

    # Fallback: capability list
    capabilities = [
        "file organization (scan, categorize, deduplicate)",
        "Excel/CSV error checking",
        "data processing (load, transform, profile, export)",
        "Salesforce integration",
        "Google Suite integration",
        "database queries",
        "API discovery",
        "data pipeline building",
    ]
    return (
        f"I received your message: \"{message}\"\n\n"
        "I can help you with:\n"
        + "\n".join(f"  - {c}" for c in capabilities)
        + "\n\nOpenClaw is not currently available. "
        "Install it or set the OPENCLAW_CMD environment variable to enable LLM-powered responses."
    )


async def _handle_file_org_chat(message: str, context: dict) -> str:
    """Handle file organization requests from chat."""
    directory = context.get("directory", ".")
    organizer = FileOrganizer()
    scan = organizer.scan_directory(directory)
    return json.dumps(scan, default=str)


async def _handle_excel_chat(message: str, context: dict) -> str:
    """Handle Excel checking requests from chat."""
    file_path = context.get("file_path")
    if not file_path:
        return "Please provide a file_path in the context."
    checker = ExcelChecker()
    report = checker.check_file(file_path)
    return json.dumps(report, default=str)


async def _handle_data_chat(message: str, context: dict) -> str:
    """Handle data processing requests from chat."""
    return "Data processing via chat: please use the REST endpoints for structured operations."


async def _handle_pipeline_chat(message: str, context: dict) -> str:
    """Handle pipeline requests from chat."""
    return "Pipeline building via chat: please use the REST endpoints for structured operations."


# ---------------------------------------------------------------------------
# File Organization Endpoints
# ---------------------------------------------------------------------------
@app.post("/api/files/scan", tags=["files"])
async def scan_directory(req: FileOrgRequest) -> JSONResponse:
    """Scan a directory and return file categorization."""
    try:
        organizer = FileOrganizer()
        result = organizer.scan_directory(req.directory)
        return JSONResponse(content=result)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/files/organize", tags=["files"])
async def organize_directory(req: FileOrgRequest) -> JSONResponse:
    """Organize files in a directory into categorized folders."""
    try:
        organizer = FileOrganizer()
        organize_req = OrganizeRequest(
            source_directory=req.directory,
            target_directory=req.target_directory or req.directory,
            dry_run=req.dry_run,
        )
        result = organizer.organize(organize_req)
        return JSONResponse(content=result)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/files/duplicates", tags=["files"])
async def find_duplicates(req: FileOrgRequest) -> JSONResponse:
    """Find duplicate files in a directory."""
    try:
        organizer = FileOrganizer()
        result = organizer.find_duplicates(req.directory)
        return JSONResponse(content=result)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


# ---------------------------------------------------------------------------
# Excel / Spreadsheet Checking Endpoints
# ---------------------------------------------------------------------------
@app.post("/api/excel/check", tags=["excel"])
async def check_excel_file(req: ExcelCheckRequest) -> JSONResponse:
    """Check an Excel or CSV file for common errors."""
    try:
        checker = ExcelChecker()
        report = checker.check_file(req.file_path)
        return JSONResponse(content=report)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/excel/upload-check", tags=["excel"])
async def check_uploaded_excel(file: UploadFile = File(...)) -> JSONResponse:
    """Upload and check an Excel or CSV file for errors."""
    import tempfile
    import shutil

    suffix = Path(file.filename or "upload.xlsx").suffix
    try:
        with tempfile.NamedTemporaryFile(delete=False, suffix=suffix) as tmp:
            shutil.copyfileobj(file.file, tmp)
            tmp_path = tmp.name

        checker = ExcelChecker()
        report = checker.check_file(tmp_path)
        return JSONResponse(content=report)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    finally:
        Path(tmp_path).unlink(missing_ok=True)


# ---------------------------------------------------------------------------
# Data Processing Endpoints
# ---------------------------------------------------------------------------
@app.post("/api/data/load", tags=["data"])
async def load_data(file: UploadFile = File(...)) -> JSONResponse:
    """Load a data file and return a preview with schema info."""
    import tempfile
    import shutil

    suffix = Path(file.filename or "data.csv").suffix
    try:
        with tempfile.NamedTemporaryFile(delete=False, suffix=suffix) as tmp:
            shutil.copyfileobj(file.file, tmp)
            tmp_path = tmp.name

        processor: DataProcessor = _state["data_processor"]
        df = processor.load(tmp_path)
        schema = processor.infer_schema(df)
        preview = df.head(20).to_dict(orient="records")
        return JSONResponse(content={
            "rows": len(df),
            "columns": list(df.columns),
            "schema": schema,
            "preview": preview,
        })
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    finally:
        Path(tmp_path).unlink(missing_ok=True)


@app.post("/api/data/profile", tags=["data"])
async def profile_data(file: UploadFile = File(...)) -> JSONResponse:
    """Profile a data file: statistics, distributions, missing values."""
    import tempfile
    import shutil

    suffix = Path(file.filename or "data.csv").suffix
    try:
        with tempfile.NamedTemporaryFile(delete=False, suffix=suffix) as tmp:
            shutil.copyfileobj(file.file, tmp)
            tmp_path = tmp.name

        processor: DataProcessor = _state["data_processor"]
        df = processor.load(tmp_path)
        profile = processor.profile(df)
        return JSONResponse(content=profile)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    finally:
        Path(tmp_path).unlink(missing_ok=True)


@app.post("/api/data/transform", tags=["data"])
async def transform_data(req: TransformRequest) -> JSONResponse:
    """Apply transformations to a previously loaded dataset."""
    try:
        processor: DataProcessor = _state["data_processor"]
        df = processor.load(req.file_path)
        result_df = processor.transform(df, req)
        return JSONResponse(content={
            "rows": len(result_df),
            "columns": list(result_df.columns),
            "preview": result_df.head(50).to_dict(orient="records"),
        })
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


# ---------------------------------------------------------------------------
# Salesforce Endpoints
# ---------------------------------------------------------------------------
@app.post("/api/salesforce/query", tags=["salesforce"])
async def salesforce_query(req: SalesforceQueryRequest) -> JSONResponse:
    """Execute a SOQL query against Salesforce."""
    try:
        connector = SalesforceConnector(req.config)
        connector.connect()
        df = connector.query_to_dataframe(req.soql)
        return JSONResponse(content={
            "rows": len(df),
            "columns": list(df.columns),
            "data": df.to_dict(orient="records"),
        })
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/salesforce/describe", tags=["salesforce"])
async def salesforce_describe(config: SalesforceConfig, object_name: str = Query(...)) -> JSONResponse:
    """Describe a Salesforce object (get metadata)."""
    try:
        connector = SalesforceConnector(config)
        connector.connect()
        metadata = connector.describe_object(object_name)
        return JSONResponse(content=metadata)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/salesforce/record", tags=["salesforce"])
async def salesforce_record(req: SalesforceRecordRequest) -> JSONResponse:
    """Create, update, or delete a Salesforce record."""
    try:
        connector = SalesforceConnector(req.config)
        connector.connect()
        if req.action == "create":
            result = connector.create_record(req.object_name, req.data or {})
        elif req.action == "update":
            if not req.record_id:
                raise ValueError("record_id required for update")
            result = connector.update_record(req.object_name, req.record_id, req.data or {})
        elif req.action == "delete":
            if not req.record_id:
                raise ValueError("record_id required for delete")
            result = connector.delete_record(req.object_name, req.record_id)
        else:
            raise ValueError(f"Unknown action: {req.action}")
        return JSONResponse(content={"result": result})
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


# ---------------------------------------------------------------------------
# Database Endpoints
# ---------------------------------------------------------------------------
@app.post("/api/database/query", tags=["database"])
async def database_query(req: DatabaseQueryRequest) -> JSONResponse:
    """Execute a SQL query and return results as JSON."""
    try:
        connector = DatabaseConnector(req.config)
        df = connector.execute_query(req.query, req.params)
        return JSONResponse(content={
            "rows": len(df),
            "columns": list(df.columns),
            "data": df.to_dict(orient="records"),
        })
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/database/schema", tags=["database"])
async def database_schema(config: DatabaseConfig) -> JSONResponse:
    """Get database schema: tables and their columns."""
    try:
        connector = DatabaseConnector(config)
        schema = connector.get_schema()
        return JSONResponse(content=schema)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/database/test-connection", tags=["database"])
async def database_test_connection(config: DatabaseConfig) -> JSONResponse:
    """Test a database connection."""
    try:
        connector = DatabaseConnector(config)
        success = connector.test_connection()
        return JSONResponse(content={"connected": success})
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


# ---------------------------------------------------------------------------
# API Discovery Endpoints
# ---------------------------------------------------------------------------
@app.post("/api/discover", tags=["api-discovery"])
async def discover_api(req: APIDiscoverRequest) -> JSONResponse:
    """Discover API endpoints from an OpenAPI/Swagger spec URL."""
    try:
        discovery: APIDiscovery = _state["api_discovery"]
        result = await discovery.discover(req.url)
        return JSONResponse(content=result)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/discover/test", tags=["api-discovery"])
async def test_api_endpoint(req: APITestRequest) -> JSONResponse:
    """Test a specific API endpoint."""
    try:
        discovery: APIDiscovery = _state["api_discovery"]
        result = await discovery.test_endpoint(
            url=req.url,
            method=req.method,
            headers=req.headers,
            body=req.body,
            auth_type=req.auth_type,
            auth_credentials=req.auth_credentials,
        )
        return JSONResponse(content=result)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


# ---------------------------------------------------------------------------
# Pipeline Endpoints
# ---------------------------------------------------------------------------
@app.post("/api/pipeline/run", tags=["pipeline"])
async def run_pipeline(req: PipelineRunRequest) -> JSONResponse:
    """Execute a data pipeline."""
    try:
        builder: PipelineBuilder = _state["pipeline_builder"]
        definition = PipelineDefinition.from_dict(req.definition)
        result = builder.execute(definition)
        return JSONResponse(content=result)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/pipeline/validate", tags=["pipeline"])
async def validate_pipeline(req: PipelineRunRequest) -> JSONResponse:
    """Validate a pipeline definition without executing it."""
    try:
        builder: PipelineBuilder = _state["pipeline_builder"]
        definition = PipelineDefinition.from_dict(req.definition)
        errors = builder.validate(definition)
        return JSONResponse(content={"valid": len(errors) == 0, "errors": errors})
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.get("/api/pipeline/list", tags=["pipeline"])
async def list_pipelines() -> JSONResponse:
    """List saved pipeline definitions."""
    try:
        builder: PipelineBuilder = _state["pipeline_builder"]
        pipelines = builder.list_saved()
        return JSONResponse(content={"pipelines": pipelines})
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/pipeline/save", tags=["pipeline"])
async def save_pipeline(req: PipelineRunRequest) -> JSONResponse:
    """Save a pipeline definition for later reuse."""
    try:
        builder: PipelineBuilder = _state["pipeline_builder"]
        definition = PipelineDefinition.from_dict(req.definition)
        path = builder.save(definition)
        return JSONResponse(content={"saved": True, "path": str(path)})
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------
def main() -> None:
    """Run the server with uvicorn."""
    import uvicorn

    uvicorn.run(
        "backend.server:app",
        host="127.0.0.1",
        port=8000,
        reload=True,
        log_level="info",
    )


if __name__ == "__main__":
    main()
