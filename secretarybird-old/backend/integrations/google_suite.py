"""
Google Suite Integration Connector

Provides:
- Google Sheets: read, write, create spreadsheets
- Google Drive: list, download, upload files
- OAuth2 authentication flow and credential management
- Error checking for Google Sheets (similar to Excel checker)
"""

import logging
from pathlib import Path
from typing import Any, Optional

import pandas as pd
from pydantic import BaseModel

logger = logging.getLogger("tojo.google_suite")

# Default credential / token storage
_CRED_DIR = Path.home() / ".tojo" / "credentials"
_DEFAULT_TOKEN_PATH = _CRED_DIR / "google_token.json"
_DEFAULT_CREDENTIALS_PATH = _CRED_DIR / "google_credentials.json"

# Scopes required for Sheets + Drive
SCOPES = [
    "https://www.googleapis.com/auth/spreadsheets",
    "https://www.googleapis.com/auth/drive",
]


# ---------------------------------------------------------------------------
# Configuration models
# ---------------------------------------------------------------------------

class GoogleSheetsConfig(BaseModel):
    """Configuration for Google Sheets operations."""
    spreadsheet_id: str
    range: Optional[str] = None  # e.g. "Sheet1!A1:Z1000"
    sheet_name: Optional[str] = None


class GoogleDriveConfig(BaseModel):
    """Configuration for Google Drive operations."""
    folder_id: Optional[str] = None
    mime_type_filter: Optional[str] = None
    query: Optional[str] = None


# ---------------------------------------------------------------------------
# Google Suite Connector
# ---------------------------------------------------------------------------

class GoogleSuiteConnector:
    """
    Manages Google API authentication and provides helpers for
    Google Sheets and Google Drive operations.
    """

    def __init__(
        self,
        credentials_path: Optional[str] = None,
        token_path: Optional[str] = None,
    ):
        """
        Initialize the connector.

        Args:
            credentials_path: Path to the OAuth2 client secrets JSON file
                              downloaded from Google Cloud Console.
            token_path: Path where the refresh/access token is stored.
        """
        self._credentials_path = Path(credentials_path) if credentials_path else _DEFAULT_CREDENTIALS_PATH
        self._token_path = Path(token_path) if token_path else _DEFAULT_TOKEN_PATH
        self._creds: Any = None
        self._sheets_service: Any = None
        self._drive_service: Any = None

    # ------------------------------------------------------------------
    # Authentication
    # ------------------------------------------------------------------
    def authenticate(self, headless: bool = False) -> None:
        """
        Run the OAuth2 authentication flow.

        If a valid token exists on disk it is reused. Otherwise, the
        browser-based consent flow is launched (unless headless=True).

        Args:
            headless: If True, skip the browser flow and raise if no
                      valid token is available.
        """
        try:
            from google.auth.transport.requests import Request  # type: ignore[import-untyped]
            from google.oauth2.credentials import Credentials  # type: ignore[import-untyped]
            from google_auth_oauthlib.flow import InstalledAppFlow  # type: ignore[import-untyped]
        except ImportError as exc:
            raise RuntimeError(
                "Google auth libraries not installed. Run: "
                "pip install google-api-python-client google-auth google-auth-oauthlib"
            ) from exc

        creds = None

        # Attempt to load existing token
        if self._token_path.is_file():
            creds = Credentials.from_authorized_user_file(str(self._token_path), SCOPES)

        # Refresh or re-auth
        if creds and creds.expired and creds.refresh_token:
            logger.info("Refreshing expired Google token...")
            creds.refresh(Request())
        elif not creds or not creds.valid:
            if headless:
                raise RuntimeError(
                    "No valid Google token available and headless mode is on. "
                    "Run authenticate(headless=False) to complete the consent flow."
                )
            if not self._credentials_path.is_file():
                raise FileNotFoundError(
                    f"Google client secrets file not found: {self._credentials_path}. "
                    "Download it from the Google Cloud Console."
                )
            flow = InstalledAppFlow.from_client_secrets_file(
                str(self._credentials_path),
                SCOPES,
            )
            creds = flow.run_local_server(port=0)

        # Persist token
        self._token_path.parent.mkdir(parents=True, exist_ok=True)
        self._token_path.write_text(creds.to_json(), encoding="utf-8")
        self._creds = creds
        logger.info("Google authentication successful.")

    def _ensure_authenticated(self) -> Any:
        """Return credentials, raising if not authenticated."""
        if self._creds is None:
            raise RuntimeError("Not authenticated. Call authenticate() first.")
        return self._creds

    def _get_sheets_service(self) -> Any:
        """Lazy-build and return the Sheets API service."""
        if self._sheets_service is None:
            from googleapiclient.discovery import build  # type: ignore[import-untyped]

            creds = self._ensure_authenticated()
            self._sheets_service = build("sheets", "v4", credentials=creds)
        return self._sheets_service

    def _get_drive_service(self) -> Any:
        """Lazy-build and return the Drive API service."""
        if self._drive_service is None:
            from googleapiclient.discovery import build  # type: ignore[import-untyped]

            creds = self._ensure_authenticated()
            self._drive_service = build("drive", "v3", credentials=creds)
        return self._drive_service

    # ------------------------------------------------------------------
    # Google Sheets operations
    # ------------------------------------------------------------------
    def sheets_read(self, config: GoogleSheetsConfig) -> pd.DataFrame:
        """
        Read data from a Google Sheet into a DataFrame.

        Args:
            config: GoogleSheetsConfig with spreadsheet_id and optional range.

        Returns:
            pandas DataFrame with sheet data.
        """
        service = self._get_sheets_service()
        range_str = config.range or (f"{config.sheet_name}" if config.sheet_name else "Sheet1")

        result = (
            service.spreadsheets()
            .values()
            .get(spreadsheetId=config.spreadsheet_id, range=range_str)
            .execute()
        )
        values = result.get("values", [])
        if not values:
            return pd.DataFrame()

        # First row as header
        header = values[0]
        data = values[1:]
        # Pad short rows
        max_len = len(header)
        padded = [row + [""] * (max_len - len(row)) for row in data]
        return pd.DataFrame(padded, columns=header)

    def sheets_write(
        self,
        config: GoogleSheetsConfig,
        df: pd.DataFrame,
        append: bool = False,
    ) -> dict[str, Any]:
        """
        Write a DataFrame to a Google Sheet.

        Args:
            config: GoogleSheetsConfig with spreadsheet_id and optional range.
            df: Data to write.
            append: If True, append below existing data; otherwise overwrite.

        Returns:
            API response dict.
        """
        service = self._get_sheets_service()
        range_str = config.range or (f"{config.sheet_name}" if config.sheet_name else "Sheet1")

        # Prepare values (header + rows), converting everything to strings
        values = [df.columns.tolist()] + df.astype(str).values.tolist()
        body = {"values": values}

        if append:
            result = (
                service.spreadsheets()
                .values()
                .append(
                    spreadsheetId=config.spreadsheet_id,
                    range=range_str,
                    valueInputOption="USER_ENTERED",
                    insertDataOption="INSERT_ROWS",
                    body=body,
                )
                .execute()
            )
        else:
            # Clear then update
            service.spreadsheets().values().clear(
                spreadsheetId=config.spreadsheet_id,
                range=range_str,
            ).execute()
            result = (
                service.spreadsheets()
                .values()
                .update(
                    spreadsheetId=config.spreadsheet_id,
                    range=range_str,
                    valueInputOption="USER_ENTERED",
                    body=body,
                )
                .execute()
            )

        logger.info(
            "Wrote %d rows to Google Sheet %s",
            len(df),
            config.spreadsheet_id,
        )
        return result

    def sheets_create(self, title: str) -> dict[str, Any]:
        """
        Create a new Google Spreadsheet.

        Args:
            title: Title for the new spreadsheet.

        Returns:
            Dictionary with 'spreadsheet_id' and 'url'.
        """
        service = self._get_sheets_service()
        body = {"properties": {"title": title}}
        result = service.spreadsheets().create(body=body).execute()
        spreadsheet_id = result["spreadsheetId"]
        url = result.get("spreadsheetUrl", f"https://docs.google.com/spreadsheets/d/{spreadsheet_id}")
        logger.info("Created Google Sheet: %s (%s)", title, spreadsheet_id)
        return {"spreadsheet_id": spreadsheet_id, "url": url}

    def sheets_check_errors(self, config: GoogleSheetsConfig) -> dict[str, Any]:
        """
        Check a Google Sheet for common errors (similar to ExcelChecker).

        Looks for error strings (#REF!, #VALUE!, etc.), empty cells in
        populated columns, and mixed data types.

        Args:
            config: GoogleSheetsConfig identifying the sheet.

        Returns:
            Structured error report.
        """
        df = self.sheets_read(config)
        errors: list[dict[str, Any]] = []
        warnings: list[dict[str, Any]] = []

        error_values = {"#REF!", "#VALUE!", "#NAME?", "#NULL!", "#N/A", "#DIV/0!", "#NUM!", "#ERROR!"}

        for col_idx, col_name in enumerate(df.columns):
            col = df[col_name]
            type_counts: dict[str, int] = {}

            for row_idx, value in enumerate(col):
                cell_ref = f"{col_name}:{row_idx + 2}"  # +2 for 1-indexed + header

                # Error values
                if isinstance(value, str) and value.strip() in error_values:
                    errors.append({
                        "type": "formula_error",
                        "cell": cell_ref,
                        "error_value": value.strip(),
                        "severity": "error",
                    })

                # Type tracking
                if value is None or (isinstance(value, str) and value.strip() == ""):
                    type_counts["empty"] = type_counts.get("empty", 0) + 1
                else:
                    try:
                        float(str(value).replace(",", ""))
                        type_counts["number"] = type_counts.get("number", 0) + 1
                    except ValueError:
                        type_counts["text"] = type_counts.get("text", 0) + 1

            # Mixed types
            non_empty_types = {k: v for k, v in type_counts.items() if k != "empty"}
            if len(non_empty_types) > 1:
                total = sum(non_empty_types.values())
                dominant = max(non_empty_types, key=non_empty_types.get)  # type: ignore[arg-type]
                minority_pct = (1 - non_empty_types[dominant] / total) * 100
                if minority_pct > 2:
                    warnings.append({
                        "type": "mixed_data_types",
                        "column": col_name,
                        "type_distribution": non_empty_types,
                        "severity": "warning",
                    })

            # Empty cells in mostly-populated columns
            total_cells = len(col)
            empty_count = type_counts.get("empty", 0)
            if total_cells > 0 and 0 < empty_count < total_cells * 0.1 and empty_count > 0:
                non_empty_pct = ((total_cells - empty_count) / total_cells) * 100
                warnings.append({
                    "type": "empty_cells_in_range",
                    "column": col_name,
                    "empty_count": empty_count,
                    "fill_rate": round(non_empty_pct, 1),
                    "severity": "warning",
                })

        return {
            "spreadsheet_id": config.spreadsheet_id,
            "errors": errors,
            "warnings": warnings,
            "error_count": len(errors),
            "warning_count": len(warnings),
        }

    # ------------------------------------------------------------------
    # Google Drive operations
    # ------------------------------------------------------------------
    def drive_list_files(
        self,
        folder_id: Optional[str] = None,
        mime_type: Optional[str] = None,
        query: Optional[str] = None,
        page_size: int = 100,
    ) -> list[dict[str, Any]]:
        """
        List files in Google Drive.

        Args:
            folder_id: Optional folder ID to list contents of.
            mime_type: Optional MIME type filter.
            query: Optional raw Drive query string.
            page_size: Number of results per page.

        Returns:
            List of file metadata dictionaries.
        """
        service = self._get_drive_service()

        q_parts: list[str] = []
        if folder_id:
            q_parts.append(f"'{folder_id}' in parents")
        if mime_type:
            q_parts.append(f"mimeType='{mime_type}'")
        if query:
            q_parts.append(query)
        q_parts.append("trashed=false")
        q_string = " and ".join(q_parts)

        all_files: list[dict[str, Any]] = []
        page_token: Optional[str] = None

        while True:
            params: dict[str, Any] = {
                "q": q_string,
                "pageSize": page_size,
                "fields": "nextPageToken, files(id, name, mimeType, size, modifiedTime, parents)",
            }
            if page_token:
                params["pageToken"] = page_token

            result = service.files().list(**params).execute()
            all_files.extend(result.get("files", []))
            page_token = result.get("nextPageToken")
            if not page_token:
                break

        logger.info("Listed %d files from Google Drive.", len(all_files))
        return all_files

    def drive_download(self, file_id: str, output_path: str) -> str:
        """
        Download a file from Google Drive.

        Args:
            file_id: Google Drive file ID.
            output_path: Local path to save the file.

        Returns:
            Absolute path to the downloaded file.
        """
        from googleapiclient.http import MediaIoBaseDownload  # type: ignore[import-untyped]

        service = self._get_drive_service()
        request = service.files().get_media(fileId=file_id)

        out = Path(output_path).resolve()
        out.parent.mkdir(parents=True, exist_ok=True)

        with open(out, "wb") as fh:
            downloader = MediaIoBaseDownload(fh, request)
            done = False
            while not done:
                status, done = downloader.next_chunk()
                if status:
                    logger.info("Download progress: %d%%", int(status.progress() * 100))

        logger.info("Downloaded file %s to %s", file_id, out)
        return str(out)

    def drive_upload(
        self,
        file_path: str,
        name: Optional[str] = None,
        folder_id: Optional[str] = None,
        mime_type: Optional[str] = None,
    ) -> dict[str, Any]:
        """
        Upload a file to Google Drive.

        Args:
            file_path: Local path of the file to upload.
            name: Name for the file in Drive. Defaults to the local filename.
            folder_id: Optional folder ID to upload into.
            mime_type: Optional MIME type override.

        Returns:
            Dictionary with 'id' and 'name' of the uploaded file.
        """
        from googleapiclient.http import MediaFileUpload  # type: ignore[import-untyped]

        service = self._get_drive_service()
        path = Path(file_path).resolve()
        if not path.is_file():
            raise FileNotFoundError(f"File not found: {file_path}")

        file_metadata: dict[str, Any] = {"name": name or path.name}
        if folder_id:
            file_metadata["parents"] = [folder_id]

        media = MediaFileUpload(str(path), mimetype=mime_type, resumable=True)
        result = (
            service.files()
            .create(body=file_metadata, media_body=media, fields="id, name")
            .execute()
        )
        logger.info("Uploaded %s as %s (ID: %s)", path.name, result["name"], result["id"])
        return {"id": result["id"], "name": result["name"]}

    # ------------------------------------------------------------------
    # Credential management
    # ------------------------------------------------------------------
    def save_client_secrets(self, secrets_json: str) -> Path:
        """
        Save Google OAuth2 client secrets JSON to the credential directory.

        Args:
            secrets_json: JSON string of client secrets.

        Returns:
            Path to the saved file.
        """
        _CRED_DIR.mkdir(parents=True, exist_ok=True)
        self._credentials_path.write_text(secrets_json, encoding="utf-8")
        logger.info("Saved Google client secrets to %s", self._credentials_path)
        return self._credentials_path

    @property
    def is_authenticated(self) -> bool:
        """Check if the connector has valid credentials loaded."""
        return self._creds is not None and self._creds.valid
