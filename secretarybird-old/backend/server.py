"""
Secretary Bird Assistant - FastAPI Backend Server

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
from contextlib import asynccontextmanager
from datetime import datetime
from pathlib import Path
from typing import Any, Optional

from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException, UploadFile, File, Query
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from pydantic import BaseModel

from backend.core.file_organizer import FileOrganizer, OrganizeRequest
from backend.core.excel_checker import ExcelChecker
from backend.core.data_processor import DataProcessor, TransformRequest
from backend.integrations.salesforce import SalesforceConnector, SalesforceConfig
from backend.integrations.google_suite import GoogleSuiteConnector  # noqa: F401 - needed for future endpoints
from backend.integrations.databases import DatabaseConnector, DatabaseConfig
from backend.integrations.api_discovery import APIDiscovery
from backend.integrations.competitor_analysis import CompetitorAnalysis
from backend.integrations.social_feeds import SocialFeedMonitor
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
    logger.info("Secretary Bird Assistant backend starting up...")
    _state["start_time"] = datetime.utcnow().isoformat()
    _state["data_processor"] = DataProcessor()
    _state["pipeline_builder"] = PipelineBuilder()
    _state["api_discovery"] = APIDiscovery()
    _state["competitor_analysis"] = CompetitorAnalysis()
    _state["social_feed_monitor"] = SocialFeedMonitor("Accelerate Okanagan")
    _state["openclaw_bridge"] = OpenClawBridge()
    _state["active_websockets"] = set()

    # Detect OpenClaw gateway availability at startup
    openclaw_status = _state["openclaw_bridge"].detect()
    _state["openclaw_available"] = openclaw_status["available"]
    if openclaw_status["available"]:
        logger.info("OpenClaw gateway connected: %s (agent: %s)", openclaw_status["gateway_url"], openclaw_status["agent_id"])
    else:
        logger.warning("OpenClaw gateway not available: %s", openclaw_status.get("error", "unknown"))
    logger.info("Backend ready.")
    yield
    logger.info("Secretary Bird Assistant backend shutting down...")
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
    title="Secretary Bird Assistant",
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


class CompetitorAnalysisRequest(BaseModel):
    """Competitor analysis request."""
    industry: str = "default"
    competitors: list[dict[str, str]]  # [{"url": "...", "name": "..."}]
    your_company: Optional[str] = None
    custom_scores: Optional[dict[str, dict[str, int]]] = None


class CompetitorScrapeRequest(BaseModel):
    """Single competitor scrape request."""
    url: str
    name: Optional[str] = None


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
                elif handler == "competitor_analysis":
                    result = await _handle_competitor_chat(user_message, context)
                elif handler == "social_feeds":
                    result = await _handle_social_feeds_chat(user_message, context)
                else:
                    # Default handler uses OpenClaw streaming when available
                    bridge: OpenClawBridge = _state.get("openclaw_bridge")
                    if bridge:
                        # Always do a fresh detect — gateway may have started after backend
                        openclaw_status = bridge.detect()
                        _state["openclaw_available"] = openclaw_status["available"]
                        if openclaw_status["available"]:
                            try:
                                # Build history from payload
                                raw_history = payload.get("history", [])
                                history = [
                                    {"role": h.get("role", "user"), "content": h.get("content", "")}
                                    for h in raw_history
                                    if h.get("role") in ("user", "assistant") and h.get("content")
                                ]
                                await ws.send_json({"type": "stream_start"})
                                async for chunk in bridge.chat_stream(user_message, history):
                                    await ws.send_json({"type": "stream_chunk", "content": chunk})
                                await ws.send_json({"type": "stream_end"})
                                # Update frontend status
                                await ws.send_json({"type": "status", "openclaw": True})
                                continue
                            except Exception as exc:
                                logger.warning("OpenClaw streaming failed, using fallback: %s", exc)
                    result = _handle_default_chat(user_message)

                # Stream the response in chunks
                chunk_size = 80
                text = result if isinstance(result, str) else json.dumps(result)

                await ws.send_json({"type": "stream_start"})

                for i in range(0, len(text), chunk_size):
                    await ws.send_json({
                        "type": "stream_chunk",
                        "content": text[i:i + chunk_size],
                    })
                    await asyncio.sleep(0.02)

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
    Fallback chat handler when OpenClaw is not available.

    Returns capability hints so the user knows what Secretary Bird can do.
    """
    capabilities = [
        "file organization (scan, categorize, deduplicate)",
        "Excel/CSV error checking",
        "data processing (load, transform, profile, export)",
        "Salesforce integration",
        "Google Suite integration",
        "database queries",
        "API discovery",
        "data pipeline building",
        "competitor analysis & Blue Ocean Strategy",
        "social feed monitoring (Twitter & Google Reviews)",
    ]
    return (
        f"I received your message: \"{message}\"\n\n"
        "I can help you with:\n"
        + "\n".join(f"  - {c}" for c in capabilities)
        + "\n\nOpenClaw gateway is not connected. "
        "Start it in WSL with: openclaw gateway"
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


async def _handle_competitor_chat(message: str, context: dict) -> str:
    """Handle competitor analysis requests from chat."""
    analysis: CompetitorAnalysis = _state["competitor_analysis"]

    competitors = context.get("competitors", [])
    industry = context.get("industry", "default")
    your_company = context.get("your_company")

    if not competitors:
        return (
            "I can help you analyze your competitors using Blue Ocean Strategy. "
            "Please provide competitor details using the sidebar or REST API.\n\n"
            "**What I need:**\n"
            "- Competitor website URLs (I'll scrape them for intelligence)\n"
            "- Your industry type (saas, ecommerce, consulting, or general)\n"
            "- Optionally, your company name\n\n"
            f"**Supported industries:** {', '.join(analysis.list_industries())}\n\n"
            "Use the **Analyze Competitors** button in the sidebar to get started, "
            "or send competitor URLs directly in the chat."
        )

    report = await analysis.analyze(
        industry=industry,
        competitors=competitors,
        your_company=your_company,
    )
    return analysis.format_report_text(report)


async def _handle_social_feeds_chat(message: str, context: dict) -> str:
    """Handle social feed monitoring requests from chat."""
    monitor: SocialFeedMonitor = _state["social_feed_monitor"]
    return await monitor.handle_chat(message, context)


# ---------------------------------------------------------------------------
# Social Feed Monitoring Endpoints
# ---------------------------------------------------------------------------
class SocialFeedReportRequest(BaseModel):
    """Request body for report generation."""
    cadence: str = "daily"  # hourly | daily | weekly


class SocialFeedHeartbeatRequest(BaseModel):
    """Request body for heartbeat control."""
    cadence: str = "daily"  # hourly | daily | weekly


@app.post("/api/social-feeds/fetch", tags=["social-feeds"])
async def fetch_social_feeds() -> JSONResponse:
    """Fetch latest Twitter and Google Reviews for the monitored company."""
    try:
        monitor: SocialFeedMonitor = _state["social_feed_monitor"]
        snapshot = await monitor.fetch_feeds()
        return JSONResponse(content=snapshot.to_dict())
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc))


@app.get("/api/social-feeds/data", tags=["social-feeds"])
async def get_social_feed_data() -> JSONResponse:
    """Get the most recently stored feed data."""
    try:
        monitor: SocialFeedMonitor = _state["social_feed_monitor"]
        snapshot = monitor.load_feeds()
        return JSONResponse(content=snapshot.to_dict())
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc))


@app.post("/api/social-feeds/report", tags=["social-feeds"])
async def generate_social_feed_report(req: SocialFeedReportRequest) -> JSONResponse:
    """Generate a market analysis report at the specified cadence (hourly/daily/weekly)."""
    try:
        monitor: SocialFeedMonitor = _state["social_feed_monitor"]
        snapshot = await monitor.fetch_feeds()
        report = monitor.generate_report(req.cadence, snapshot)
        return JSONResponse(content=report.to_dict())
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc))


@app.get("/api/social-feeds/reports", tags=["social-feeds"])
async def list_social_feed_reports() -> JSONResponse:
    """List all saved daily reports."""
    monitor: SocialFeedMonitor = _state["social_feed_monitor"]
    return JSONResponse(content={"reports": monitor.list_reports()})


@app.post("/api/social-feeds/heartbeat/start", tags=["social-feeds"])
async def start_heartbeat(req: SocialFeedHeartbeatRequest) -> JSONResponse:
    """Start a heartbeat scheduler for the specified cadence (hourly/daily/weekly)."""
    monitor: SocialFeedMonitor = _state["social_feed_monitor"]
    try:
        started = monitor.start_heartbeat(req.cadence)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    return JSONResponse(content={
        "started": started,
        "cadence": req.cadence,
        "message": f"Heartbeat [{req.cadence}] started" if started else f"Heartbeat [{req.cadence}] already running",
        "all_heartbeats": monitor.heartbeat_status(),
    })


@app.post("/api/social-feeds/heartbeat/stop", tags=["social-feeds"])
async def stop_heartbeat(req: Optional[SocialFeedHeartbeatRequest] = None) -> JSONResponse:
    """Stop heartbeat scheduler(s). Pass cadence to stop one, or omit to stop all."""
    monitor: SocialFeedMonitor = _state["social_feed_monitor"]
    cadence = req.cadence if req else None
    stopped = monitor.stop_heartbeat(cadence)
    return JSONResponse(content={
        "stopped": stopped,
        "cadence": cadence or "all",
        "message": "Heartbeat stopped" if stopped else "Heartbeat was not running",
        "all_heartbeats": monitor.heartbeat_status(),
    })


@app.get("/api/social-feeds/heartbeat/status", tags=["social-feeds"])
async def heartbeat_status() -> JSONResponse:
    """Check heartbeat scheduler status for all cadences."""
    monitor: SocialFeedMonitor = _state["social_feed_monitor"]
    return JSONResponse(content={
        "active": monitor.heartbeat_active,
        "company": monitor.company,
        "heartbeats": monitor.heartbeat_status(),
    })


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
# Competitor Analysis Endpoints
# ---------------------------------------------------------------------------
@app.post("/api/competitors/analyze", tags=["competitor-analysis"])
async def analyze_competitors(req: CompetitorAnalysisRequest) -> JSONResponse:
    """
    Run a full competitor analysis with Blue Ocean Strategy.

    Scrapes competitor websites, builds a Strategy Canvas, applies the
    Four Actions Framework, and identifies Blue Ocean opportunities.
    """
    try:
        analysis: CompetitorAnalysis = _state["competitor_analysis"]
        report = await analysis.analyze(
            industry=req.industry,
            competitors=req.competitors,
            your_company=req.your_company,
            custom_scores=req.custom_scores,
        )
        return JSONResponse(content=report.to_dict())
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.post("/api/competitors/scrape", tags=["competitor-analysis"])
async def scrape_competitor(req: CompetitorScrapeRequest) -> JSONResponse:
    """Scrape a single competitor website for a quick preview."""
    try:
        analysis: CompetitorAnalysis = _state["competitor_analysis"]
        profile = await analysis.scrape_single(req.url, req.name)
        return JSONResponse(content=profile.to_dict())
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))


@app.get("/api/competitors/industries", tags=["competitor-analysis"])
async def list_industries() -> JSONResponse:
    """List supported industry types for competitor analysis."""
    analysis: CompetitorAnalysis = _state["competitor_analysis"]
    industries = analysis.list_industries()
    factors_by_industry = {
        ind: analysis.get_industry_factors(ind) for ind in industries
    }
    return JSONResponse(content={
        "industries": industries,
        "factors": factors_by_industry,
    })


@app.get("/api/competitors/reports", tags=["competitor-analysis"])
async def list_competitor_reports() -> JSONResponse:
    """List previously saved competitor analysis reports."""
    analysis: CompetitorAnalysis = _state["competitor_analysis"]
    return JSONResponse(content={"reports": analysis.list_reports()})


@app.post("/api/competitors/save", tags=["competitor-analysis"])
async def save_competitor_report(req: CompetitorAnalysisRequest) -> JSONResponse:
    """Run analysis and save the report to disk."""
    try:
        analysis: CompetitorAnalysis = _state["competitor_analysis"]
        report = await analysis.analyze(
            industry=req.industry,
            competitors=req.competitors,
            your_company=req.your_company,
            custom_scores=req.custom_scores,
        )
        path = analysis.save_report(report)
        return JSONResponse(content={"saved": True, "path": str(path)})
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
        app,
        host="127.0.0.1",
        port=8000,
        log_level="info",
    )


if __name__ == "__main__":
    main()
