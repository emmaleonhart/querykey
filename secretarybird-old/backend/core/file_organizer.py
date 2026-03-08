"""
File Organization Module

Provides directory scanning, file categorization, duplicate detection,
and automated file organization into structured folder hierarchies.
Designed for common business file types encountered in office environments.
"""

import hashlib
import logging
import shutil
from collections import defaultdict
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Optional

from pydantic import BaseModel


# ---------------------------------------------------------------------------
# FileCategory enum
# ---------------------------------------------------------------------------

class FileCategory(str, Enum):
    """Categories for file classification."""
    DOCUMENTS = "Documents"
    SPREADSHEETS = "Spreadsheets"
    PRESENTATIONS = "Presentations"
    IMAGES = "Images"
    DATA = "Data"
    CODE = "Code"
    ARCHIVES = "Archives"
    AUDIO = "Audio"
    VIDEO = "Video"
    EMAIL = "Email"
    OTHER = "Other"


logger = logging.getLogger("tojo.file_organizer")

# ---------------------------------------------------------------------------
# Category definitions
# ---------------------------------------------------------------------------

FILE_CATEGORIES: dict[str, set[str]] = {
    "Documents": {
        ".doc", ".docx", ".odt", ".rtf", ".txt", ".pdf",
        ".pages", ".tex", ".wpd",
    },
    "Spreadsheets": {
        ".xls", ".xlsx", ".xlsm", ".xlsb", ".ods", ".csv", ".tsv",
        ".numbers",
    },
    "Presentations": {
        ".ppt", ".pptx", ".odp", ".key",
    },
    "Images": {
        ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tiff", ".tif",
        ".svg", ".webp", ".ico", ".heic", ".heif", ".raw",
    },
    "Data": {
        ".json", ".xml", ".yaml", ".yml", ".toml", ".parquet",
        ".avro", ".feather", ".hdf5", ".h5", ".sqlite", ".db",
    },
    "Code": {
        ".py", ".js", ".ts", ".jsx", ".tsx", ".java", ".c", ".cpp",
        ".h", ".hpp", ".cs", ".go", ".rs", ".rb", ".php", ".swift",
        ".kt", ".scala", ".r", ".m", ".sh", ".bat", ".ps1",
    },
    "Archives": {
        ".zip", ".tar", ".gz", ".bz2", ".7z", ".rar", ".xz",
        ".tgz", ".tar.gz",
    },
    "Audio": {
        ".mp3", ".wav", ".flac", ".aac", ".ogg", ".wma", ".m4a",
    },
    "Video": {
        ".mp4", ".avi", ".mkv", ".mov", ".wmv", ".flv", ".webm",
        ".m4v",
    },
    "Email": {
        ".eml", ".msg", ".mbox", ".pst",
    },
}

# Reverse lookup: extension -> category
_EXT_TO_CATEGORY: dict[str, str] = {}
for _cat, _exts in FILE_CATEGORIES.items():
    for _ext in _exts:
        _EXT_TO_CATEGORY[_ext] = _cat


def _categorize_extension(ext: str) -> str:
    """Return the category name for a given file extension."""
    return _EXT_TO_CATEGORY.get(ext.lower(), "Other")


# ---------------------------------------------------------------------------
# Models
# ---------------------------------------------------------------------------

class OrganizeRequest(BaseModel):
    """Parameters for a file organization operation."""
    source_directory: str
    target_directory: Optional[str] = None
    dry_run: bool = True
    copy_instead_of_move: bool = False


class FileInfo(BaseModel):
    """Metadata about a single file."""
    path: str
    name: str
    extension: str
    category: str
    size_bytes: int
    modified: str
    created: str


# ---------------------------------------------------------------------------
# FileOrganizer
# ---------------------------------------------------------------------------

class FileOrganizer:
    """
    Scans directories, categorizes files by type, detects duplicates,
    and can reorganize files into a clean folder structure.
    """

    # Hash buffer size for duplicate detection (8 KB)
    _HASH_CHUNK_SIZE = 8192

    def classify_extension(self, ext: str) -> FileCategory:
        """Classify a file extension into a FileCategory enum value."""
        category_name = _categorize_extension(ext)
        try:
            return FileCategory(category_name)
        except ValueError:
            return FileCategory.OTHER

    def scan_directory(self, directory: str) -> dict[str, Any]:
        """
        Scan a directory and return categorized file information.

        Args:
            directory: Path to the directory to scan.

        Returns:
            Dictionary with keys:
                - total_files: int
                - total_size_bytes: int
                - categories: dict mapping category name to list of FileInfo dicts
                - summary: dict mapping category name to file count
        """
        dir_path = Path(directory).resolve()
        if not dir_path.is_dir():
            raise ValueError(f"Not a valid directory: {directory}")

        categories: dict[str, list[dict[str, Any]]] = defaultdict(list)
        total_size = 0
        total_files = 0

        for item in dir_path.rglob("*"):
            if not item.is_file():
                continue

            ext = item.suffix.lower()
            category = _categorize_extension(ext)
            stat = item.stat()
            total_size += stat.st_size
            total_files += 1

            info = {
                "path": str(item),
                "name": item.name,
                "extension": ext,
                "category": category,
                "size_bytes": stat.st_size,
                "modified": datetime.fromtimestamp(stat.st_mtime).isoformat(),
                "created": datetime.fromtimestamp(stat.st_ctime).isoformat(),
            }
            categories[category].append(info)

        summary = {cat: len(files) for cat, files in categories.items()}

        return {
            "total_files": total_files,
            "total_size_bytes": total_size,
            "categories": dict(categories),
            "summary": summary,
        }

    def suggest_structure(self, directory: str) -> dict[str, Any]:
        """
        Suggest an organized folder structure based on current contents.

        Args:
            directory: Path to the directory to analyze.

        Returns:
            Dictionary with proposed folder hierarchy and file moves.
        """
        scan = self.scan_directory(directory)
        suggestions: list[dict[str, str]] = []
        proposed_folders: list[str] = []

        target = Path(directory).resolve()

        for category, files in scan["categories"].items():
            if not files:
                continue
            folder = target / category
            proposed_folders.append(str(folder))
            for f in files:
                current_path = f["path"]
                new_path = str(folder / Path(current_path).name)
                if current_path != new_path:
                    suggestions.append({
                        "from": current_path,
                        "to": new_path,
                        "category": category,
                    })

        return {
            "proposed_folders": proposed_folders,
            "moves": suggestions,
            "move_count": len(suggestions),
        }

    def organize(self, request: OrganizeRequest) -> dict[str, Any]:
        """
        Organize files by moving or copying them into category folders.

        Args:
            request: An OrganizeRequest with source/target/options.

        Returns:
            Dictionary with operation results (moved/copied files, errors).
        """
        source = Path(request.source_directory).resolve()
        target = Path(request.target_directory or request.source_directory).resolve()

        if not source.is_dir():
            raise ValueError(f"Source directory does not exist: {source}")

        scan = self.scan_directory(str(source))
        moved: list[dict[str, str]] = []
        errors: list[dict[str, str]] = []

        for category, files in scan["categories"].items():
            if not files:
                continue

            dest_folder = target / category

            if not request.dry_run:
                dest_folder.mkdir(parents=True, exist_ok=True)

            for f in files:
                src_path = Path(f["path"])
                dst_path = dest_folder / src_path.name

                # Avoid moving a file onto itself
                if src_path.resolve() == dst_path.resolve():
                    continue

                # Handle name collisions
                if dst_path.exists():
                    stem = dst_path.stem
                    suffix = dst_path.suffix
                    counter = 1
                    while dst_path.exists():
                        dst_path = dest_folder / f"{stem}_{counter}{suffix}"
                        counter += 1

                if request.dry_run:
                    moved.append({"from": str(src_path), "to": str(dst_path), "action": "would_move"})
                else:
                    try:
                        if request.copy_instead_of_move:
                            shutil.copy2(str(src_path), str(dst_path))
                            action = "copied"
                        else:
                            shutil.move(str(src_path), str(dst_path))
                            action = "moved"
                        moved.append({"from": str(src_path), "to": str(dst_path), "action": action})
                    except Exception as exc:
                        errors.append({"file": str(src_path), "error": str(exc)})
                        logger.error("Failed to process %s: %s", src_path, exc)

        return {
            "dry_run": request.dry_run,
            "processed": len(moved),
            "errors": len(errors),
            "operations": moved,
            "error_details": errors,
        }

    def find_duplicates(self, directory: str) -> dict[str, Any]:
        """
        Find duplicate files by comparing SHA-256 hashes.

        First groups files by size (cheap), then hashes only same-size files.

        Args:
            directory: Path to the directory to search.

        Returns:
            Dictionary with duplicate groups and potential space savings.
        """
        dir_path = Path(directory).resolve()
        if not dir_path.is_dir():
            raise ValueError(f"Not a valid directory: {directory}")

        # Phase 1: group by size
        size_groups: dict[int, list[Path]] = defaultdict(list)
        for item in dir_path.rglob("*"):
            if item.is_file():
                size_groups[item.stat().st_size].append(item)

        # Phase 2: hash only groups with >1 file of same size
        hash_groups: dict[str, list[str]] = defaultdict(list)
        for size, paths in size_groups.items():
            if len(paths) < 2:
                continue
            for p in paths:
                file_hash = self._hash_file(p)
                hash_groups[file_hash].append(str(p))

        # Filter to actual duplicates
        duplicates: list[dict[str, Any]] = []
        wasted_bytes = 0
        for file_hash, paths in hash_groups.items():
            if len(paths) < 2:
                continue
            file_size = Path(paths[0]).stat().st_size
            wasted = file_size * (len(paths) - 1)
            wasted_bytes += wasted
            duplicates.append({
                "hash": file_hash,
                "files": paths,
                "count": len(paths),
                "file_size_bytes": file_size,
                "wasted_bytes": wasted,
            })

        return {
            "duplicate_groups": len(duplicates),
            "total_duplicate_files": sum(d["count"] - 1 for d in duplicates),
            "wasted_bytes": wasted_bytes,
            "groups": duplicates,
        }

    def _hash_file(self, path: Path) -> str:
        """Compute SHA-256 hash of a file."""
        sha = hashlib.sha256()
        with open(path, "rb") as fh:
            while True:
                chunk = fh.read(self._HASH_CHUNK_SIZE)
                if not chunk:
                    break
                sha.update(chunk)
        return sha.hexdigest()
