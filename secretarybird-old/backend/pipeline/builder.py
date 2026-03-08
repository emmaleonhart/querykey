"""
Data Pipeline Builder

Provides:
- Step-by-step pipeline construction (source, transform, destination)
- Pipeline execution with pandas DataFrames
- Pipeline definition serialization (JSON)
- Save/load pipeline definitions to/from disk
"""

import json
import logging
from pathlib import Path
from typing import Any, Optional

import pandas as pd

logger = logging.getLogger("tojo.pipeline")

# Default directory for saved pipelines
_PIPELINES_DIR = Path.home() / ".tojo" / "pipelines"


# ---------------------------------------------------------------------------
# Pipeline data structures
# ---------------------------------------------------------------------------

class PipelineStep:
    """A single step in a data pipeline."""

    def __init__(self, name: str, type: str, config: Optional[dict[str, Any]] = None):
        self.name = name
        self.type = type  # "source", "transform", "destination"
        self.config = config or {}

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "type": self.type,
            "config": self.config,
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "PipelineStep":
        return cls(
            name=d["name"],
            type=d["type"],
            config=d.get("config", {}),
        )


class PipelineDefinition:
    """A complete pipeline definition with metadata and steps."""

    def __init__(
        self,
        name: str,
        description: str = "",
        steps: Optional[list[PipelineStep]] = None,
    ):
        self.name = name
        self.description = description
        self.steps = steps or []

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "description": self.description,
            "steps": [s.to_dict() for s in self.steps],
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), indent=2)

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "PipelineDefinition":
        steps = [PipelineStep.from_dict(s) for s in d.get("steps", [])]
        return cls(
            name=d["name"],
            description=d.get("description", ""),
            steps=steps,
        )

    @classmethod
    def from_json(cls, json_str: str) -> "PipelineDefinition":
        return cls.from_dict(json.loads(json_str))


# ---------------------------------------------------------------------------
# Pipeline Builder
# ---------------------------------------------------------------------------

class PipelineBuilder:
    """
    Builds and executes data pipelines from a sequence of steps.

    Steps are categorized as:
    - source: Load data from a file or connection
    - transform: Filter, sort, aggregate, etc.
    - destination: Save data to a file or connection
    """

    def __init__(self):
        self.steps: list[PipelineStep] = []

    # ------------------------------------------------------------------
    # Building
    # ------------------------------------------------------------------
    def add_step(self, name: str, type: str, config: dict[str, Any]) -> None:
        """Add a step to the pipeline."""
        self.steps.append(PipelineStep(name, type, config))
        logger.info("Added pipeline step: %s (%s)", name, type)

    def clear(self) -> None:
        """Clear all pipeline steps."""
        self.steps = []

    def build(self, name: str, description: str = "") -> PipelineDefinition:
        """Build a PipelineDefinition from current steps."""
        return PipelineDefinition(
            name=name,
            description=description,
            steps=list(self.steps),
        )

    # ------------------------------------------------------------------
    # Execution
    # ------------------------------------------------------------------
    def execute(self, definition: Optional[PipelineDefinition] = None) -> dict[str, Any]:
        """
        Execute a pipeline.

        Args:
            definition: Optional PipelineDefinition to execute.
                        If None, uses the internally built steps.

        Returns:
            Dictionary with execution results.
        """
        steps = definition.steps if definition else self.steps

        if not steps:
            raise ValueError("Pipeline has no steps to execute.")

        logger.info("Executing pipeline with %d steps", len(steps))
        df: Optional[pd.DataFrame] = None

        for i, step in enumerate(steps):
            logger.info("Step %d/%d: %s (%s)", i + 1, len(steps), step.name, step.type)

            if step.type == "source":
                df = self._execute_source(step)
            elif step.type == "transform":
                if df is None:
                    raise ValueError(f"Transform step '{step.name}' has no input data.")
                df = self._execute_transform(step, df)
            elif step.type == "destination":
                if df is None:
                    raise ValueError(f"Destination step '{step.name}' has no data to write.")
                self._execute_destination(step, df)
            else:
                raise ValueError(f"Unknown step type: {step.type}")

        rows = len(df) if df is not None else 0
        return {
            "success": True,
            "steps_executed": len(steps),
            "rows_processed": rows,
        }

    def _execute_source(self, step: PipelineStep) -> pd.DataFrame:
        """Load data from a source."""
        config = step.config
        file_path = config.get("path")
        fmt = config.get("format", "csv")

        if not file_path:
            raise ValueError(f"Source step '{step.name}' missing 'path' config.")

        if fmt == "csv":
            return pd.read_csv(file_path)
        elif fmt == "excel" or fmt == "xlsx":
            return pd.read_excel(file_path)
        elif fmt == "json":
            return pd.read_json(file_path)
        elif fmt == "parquet":
            return pd.read_parquet(file_path)
        else:
            # Default to CSV
            return pd.read_csv(file_path)

    def _execute_transform(self, step: PipelineStep, df: pd.DataFrame) -> pd.DataFrame:
        """Apply a transformation to the DataFrame."""
        config = step.config
        column = config.get("column")
        value = config.get("value")

        # Filter transform
        if column and value is not None:
            if column in df.columns:
                # Try numeric comparison first
                try:
                    numeric_value = float(value)
                    df = df[df[column] == numeric_value]
                except (ValueError, TypeError):
                    df = df[df[column] == value]
            else:
                raise ValueError(f"Column '{column}' not found in data.")

        # Sort transform
        sort_by = config.get("sort_by")
        if sort_by and sort_by in df.columns:
            ascending = config.get("ascending", True)
            df = df.sort_values(by=sort_by, ascending=ascending)

        # Select columns
        select = config.get("select")
        if select:
            missing = [c for c in select if c not in df.columns]
            if missing:
                raise ValueError(f"Columns not found: {missing}")
            df = df[select]

        # Drop duplicates
        if config.get("drop_duplicates"):
            df = df.drop_duplicates()

        return df

    def _execute_destination(self, step: PipelineStep, df: pd.DataFrame) -> None:
        """Write data to a destination."""
        config = step.config
        file_path = config.get("path")
        fmt = config.get("format", "csv")

        if not file_path:
            raise ValueError(f"Destination step '{step.name}' missing 'path' config.")

        # Ensure parent directory exists
        Path(file_path).parent.mkdir(parents=True, exist_ok=True)

        if fmt == "csv":
            df.to_csv(file_path, index=False)
        elif fmt == "excel" or fmt == "xlsx":
            df.to_excel(file_path, index=False)
        elif fmt == "json":
            df.to_json(file_path, orient="records", indent=2)
        elif fmt == "parquet":
            df.to_parquet(file_path, index=False)
        else:
            df.to_csv(file_path, index=False)

        logger.info("Wrote %d rows to %s", len(df), file_path)

    # ------------------------------------------------------------------
    # Validation
    # ------------------------------------------------------------------
    def validate(self, definition: PipelineDefinition) -> list[str]:
        """
        Validate a pipeline definition without executing it.

        Returns:
            List of error messages. Empty list means valid.
        """
        errors: list[str] = []

        if not definition.steps:
            errors.append("Pipeline has no steps.")
            return errors

        has_source = False
        for i, step in enumerate(definition.steps):
            if step.type == "source":
                has_source = True
                if not step.config.get("path"):
                    errors.append(f"Step {i + 1} ({step.name}): source missing 'path'.")
            elif step.type == "transform":
                if not has_source:
                    errors.append(f"Step {i + 1} ({step.name}): transform before any source.")
            elif step.type == "destination":
                if not has_source:
                    errors.append(f"Step {i + 1} ({step.name}): destination before any source.")
                if not step.config.get("path"):
                    errors.append(f"Step {i + 1} ({step.name}): destination missing 'path'.")
            else:
                errors.append(f"Step {i + 1} ({step.name}): unknown type '{step.type}'.")

        return errors

    # ------------------------------------------------------------------
    # Save / Load
    # ------------------------------------------------------------------
    def save_definition(self, definition: PipelineDefinition, filepath: str) -> None:
        """Save a pipeline definition to a file."""
        path = Path(filepath)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(definition.to_json(), encoding="utf-8")
        logger.info("Saved pipeline '%s' to %s", definition.name, filepath)

    def load_definition(self, filepath: str) -> PipelineDefinition:
        """Load a pipeline definition from a file."""
        path = Path(filepath)
        if not path.is_file():
            raise FileNotFoundError(f"Pipeline file not found: {filepath}")
        return PipelineDefinition.from_json(path.read_text(encoding="utf-8"))

    def save(self, definition: PipelineDefinition) -> Path:
        """Save a pipeline definition to the default pipelines directory."""
        _PIPELINES_DIR.mkdir(parents=True, exist_ok=True)
        filename = f"{definition.name}.json"
        filepath = _PIPELINES_DIR / filename
        filepath.write_text(definition.to_json(), encoding="utf-8")
        logger.info("Saved pipeline '%s' to %s", definition.name, filepath)
        return filepath

    def list_saved(self) -> list[dict[str, Any]]:
        """List all saved pipeline definitions."""
        if not _PIPELINES_DIR.is_dir():
            return []

        pipelines: list[dict[str, Any]] = []
        for f in _PIPELINES_DIR.glob("*.json"):
            try:
                definition = PipelineDefinition.from_json(f.read_text(encoding="utf-8"))
                pipelines.append({
                    "name": definition.name,
                    "description": definition.description,
                    "steps": len(definition.steps),
                    "path": str(f),
                })
            except Exception as exc:
                logger.warning("Failed to load pipeline %s: %s", f.name, exc)

        return pipelines
