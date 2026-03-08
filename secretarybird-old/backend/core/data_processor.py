"""
Data Processing Module

Provides a unified interface for:
- Loading data from CSV, Excel, JSON, and Parquet formats
- Applying transformations: filter, sort, aggregate, pivot, merge
- Data profiling: statistics, distributions, missing values
- Schema inference
- Exporting to multiple formats
"""

import logging
from pathlib import Path
from typing import Any, Optional

import numpy as np
import pandas as pd
from pydantic import BaseModel

logger = logging.getLogger("tojo.data_processor")


# ---------------------------------------------------------------------------
# Request / config models
# ---------------------------------------------------------------------------

class FilterSpec(BaseModel):
    """A single filter condition."""
    column: str
    operator: str  # "eq", "ne", "gt", "lt", "gte", "lte", "contains", "in", "not_in", "is_null", "not_null"
    value: Any = None


class SortSpec(BaseModel):
    """A single sort directive."""
    column: str
    ascending: bool = True


class AggregateSpec(BaseModel):
    """Aggregation specification."""
    group_by: list[str]
    aggregations: dict[str, list[str]]  # column -> list of agg funcs ("sum", "mean", "count", ...)


class PivotSpec(BaseModel):
    """Pivot table specification."""
    index: list[str]
    columns: str
    values: str
    aggfunc: str = "sum"


class MergeSpec(BaseModel):
    """Merge (join) specification."""
    right_file_path: str
    on: Optional[list[str]] = None
    left_on: Optional[list[str]] = None
    right_on: Optional[list[str]] = None
    how: str = "inner"  # "inner", "left", "right", "outer"


class TransformRequest(BaseModel):
    """Full transformation request combining multiple operations."""
    file_path: str
    filters: Optional[list[FilterSpec]] = None
    sort: Optional[list[SortSpec]] = None
    aggregate: Optional[AggregateSpec] = None
    pivot: Optional[PivotSpec] = None
    merge: Optional[MergeSpec] = None
    select_columns: Optional[list[str]] = None
    rename_columns: Optional[dict[str, str]] = None
    drop_duplicates: Optional[bool] = None
    fill_na: Optional[dict[str, Any]] = None


class ProfileRequest(BaseModel):
    """Profiling request."""
    file_path: str
    sample_size: Optional[int] = None


# ---------------------------------------------------------------------------
# DataProcessor
# ---------------------------------------------------------------------------

class DataProcessor:
    """
    Loads, transforms, profiles, and exports tabular data.
    Wraps pandas with a clean, typed interface.
    """

    # ------------------------------------------------------------------
    # Loading
    # ------------------------------------------------------------------
    def load(self, file_path: str, **kwargs: Any) -> pd.DataFrame:
        """
        Load data from a file into a pandas DataFrame.

        Supported formats: .csv, .tsv, .xlsx, .xls, .json, .parquet

        Args:
            file_path: Path to the data file.
            **kwargs: Additional keyword arguments forwarded to the pandas reader.

        Returns:
            pandas DataFrame with the loaded data.
        """
        path = Path(file_path).resolve()
        if not path.is_file():
            raise FileNotFoundError(f"File not found: {file_path}")

        ext = path.suffix.lower()
        logger.info("Loading %s (format: %s)", path.name, ext)

        if ext == ".csv":
            return pd.read_csv(path, **kwargs)
        elif ext == ".tsv":
            return pd.read_csv(path, sep="\t", **kwargs)
        elif ext in (".xlsx", ".xls", ".xlsm"):
            return pd.read_excel(path, **kwargs)
        elif ext == ".json":
            return pd.read_json(path, **kwargs)
        elif ext == ".parquet":
            return pd.read_parquet(path, **kwargs)
        else:
            raise ValueError(f"Unsupported file format: {ext}")

    # ------------------------------------------------------------------
    # Transformations
    # ------------------------------------------------------------------
    def transform(self, df: pd.DataFrame, request: TransformRequest) -> pd.DataFrame:
        """
        Apply a sequence of transformations to a DataFrame.

        Transformations are applied in this order:
        1. Select columns
        2. Rename columns
        3. Filter rows
        4. Drop duplicates
        5. Fill NA
        6. Sort
        7. Aggregate
        8. Pivot
        9. Merge

        Args:
            df: Input DataFrame.
            request: TransformRequest specifying operations.

        Returns:
            Transformed DataFrame.
        """
        result = df.copy()

        # 1. Select columns
        if request.select_columns:
            missing = [c for c in request.select_columns if c not in result.columns]
            if missing:
                raise ValueError(f"Columns not found: {missing}")
            result = result[request.select_columns]

        # 2. Rename columns
        if request.rename_columns:
            result = result.rename(columns=request.rename_columns)

        # 3. Filters
        if request.filters:
            for f in request.filters:
                result = self._apply_filter(result, f)

        # 4. Drop duplicates
        if request.drop_duplicates:
            result = result.drop_duplicates()

        # 5. Fill NA
        if request.fill_na:
            for col, fill_value in request.fill_na.items():
                if col in result.columns:
                    result[col] = result[col].fillna(fill_value)

        # 6. Sort
        if request.sort:
            result = result.sort_values(
                by=[s.column for s in request.sort],
                ascending=[s.ascending for s in request.sort],
            )

        # 7. Aggregate
        if request.aggregate:
            agg_spec = request.aggregate
            result = result.groupby(agg_spec.group_by).agg(agg_spec.aggregations)
            # Flatten multi-level columns
            if isinstance(result.columns, pd.MultiIndex):
                result.columns = ["_".join(col).strip("_") for col in result.columns]
            result = result.reset_index()

        # 8. Pivot
        if request.pivot:
            p = request.pivot
            result = pd.pivot_table(
                result,
                index=p.index,
                columns=p.columns,
                values=p.values,
                aggfunc=p.aggfunc,
            ).reset_index()

        # 9. Merge
        if request.merge:
            m = request.merge
            right_df = self.load(m.right_file_path)
            result = result.merge(
                right_df,
                on=m.on,
                left_on=m.left_on,
                right_on=m.right_on,
                how=m.how,
            )

        return result

    def _apply_filter(self, df: pd.DataFrame, f: FilterSpec) -> pd.DataFrame:
        """Apply a single filter condition to the DataFrame."""
        if f.column not in df.columns:
            raise ValueError(f"Column not found for filter: {f.column}")

        col = df[f.column]
        op = f.operator.lower()

        if op == "eq":
            return df[col == f.value]
        elif op == "ne":
            return df[col != f.value]
        elif op == "gt":
            return df[col > f.value]
        elif op == "lt":
            return df[col < f.value]
        elif op == "gte":
            return df[col >= f.value]
        elif op == "lte":
            return df[col <= f.value]
        elif op == "contains":
            return df[col.astype(str).str.contains(str(f.value), case=False, na=False)]
        elif op == "in":
            return df[col.isin(f.value)]
        elif op == "not_in":
            return df[~col.isin(f.value)]
        elif op == "is_null":
            return df[col.isna()]
        elif op == "not_null":
            return df[col.notna()]
        else:
            raise ValueError(f"Unknown filter operator: {op}")

    # ------------------------------------------------------------------
    # Profiling
    # ------------------------------------------------------------------
    def profile(self, df: pd.DataFrame) -> dict[str, Any]:
        """
        Generate a comprehensive profile of the DataFrame.

        Includes:
        - Shape and memory usage
        - Per-column statistics
        - Missing value counts
        - Type information
        - Value distributions for categorical columns
        - Numeric statistics for numeric columns

        Args:
            df: DataFrame to profile.

        Returns:
            Dictionary with profiling results.
        """
        profile: dict[str, Any] = {
            "shape": {"rows": len(df), "columns": len(df.columns)},
            "memory_usage_bytes": int(df.memory_usage(deep=True).sum()),
            "columns": {},
            "missing_summary": {},
        }

        for col_name in df.columns:
            col = df[col_name]
            col_profile = self._profile_column(col)
            profile["columns"][col_name] = col_profile

            missing_count = int(col.isna().sum())
            if missing_count > 0:
                profile["missing_summary"][col_name] = {
                    "count": missing_count,
                    "percentage": round(missing_count / len(df) * 100, 2),
                }

        # Duplicate rows
        dup_count = int(df.duplicated().sum())
        profile["duplicate_rows"] = {
            "count": dup_count,
            "percentage": round(dup_count / max(len(df), 1) * 100, 2),
        }

        return profile

    def _profile_column(self, col: pd.Series) -> dict[str, Any]:
        """Generate profile for a single column."""
        dtype_str = str(col.dtype)
        result: dict[str, Any] = {
            "dtype": dtype_str,
            "non_null_count": int(col.notna().sum()),
            "null_count": int(col.isna().sum()),
            "unique_count": int(col.nunique()),
        }

        if pd.api.types.is_numeric_dtype(col):
            desc = col.describe()
            result["statistics"] = {
                "mean": _safe_scalar(desc.get("mean")),
                "std": _safe_scalar(desc.get("std")),
                "min": _safe_scalar(desc.get("min")),
                "25%": _safe_scalar(desc.get("25%")),
                "50%": _safe_scalar(desc.get("50%")),
                "75%": _safe_scalar(desc.get("75%")),
                "max": _safe_scalar(desc.get("max")),
            }
            result["has_negatives"] = bool((col.dropna() < 0).any())
            result["has_zeros"] = bool((col.dropna() == 0).any())
        elif pd.api.types.is_datetime64_any_dtype(col):
            result["min_date"] = str(col.min())
            result["max_date"] = str(col.max())
        else:
            # Categorical / text
            top_values = col.value_counts().head(10)
            result["top_values"] = {str(k): int(v) for k, v in top_values.items()}
            if pd.api.types.is_string_dtype(col):
                lengths = col.dropna().astype(str).str.len()
                if len(lengths) > 0:
                    result["string_length"] = {
                        "min": int(lengths.min()),
                        "max": int(lengths.max()),
                        "mean": round(float(lengths.mean()), 1),
                    }

        return result

    # ------------------------------------------------------------------
    # Schema inference
    # ------------------------------------------------------------------
    def infer_schema(self, df: pd.DataFrame) -> dict[str, Any]:
        """
        Infer a schema from the DataFrame.

        Returns a dictionary mapping column names to type info and metadata.
        """
        schema: dict[str, Any] = {}
        for col_name in df.columns:
            col = df[col_name]
            dtype_str = str(col.dtype)

            # Try to infer more specific type
            inferred = dtype_str
            if dtype_str == "object":
                sample = col.dropna().head(100)
                if len(sample) > 0:
                    # Check if all values look numeric
                    try:
                        pd.to_numeric(sample)
                        inferred = "numeric_string"
                    except (ValueError, TypeError):
                        pass
                    # Check if all values look like dates
                    if inferred == dtype_str:
                        try:
                            pd.to_datetime(sample)
                            inferred = "datetime_string"
                        except (ValueError, TypeError):
                            inferred = "string"

            schema[col_name] = {
                "pandas_dtype": dtype_str,
                "inferred_type": inferred,
                "nullable": bool(col.isna().any()),
                "unique_count": int(col.nunique()),
                "sample_values": [_safe_scalar(v) for v in col.dropna().head(5).tolist()],
            }

        return schema

    # ------------------------------------------------------------------
    # Export
    # ------------------------------------------------------------------
    def export(
        self,
        df: pd.DataFrame,
        output_path: str,
        format: Optional[str] = None,
        **kwargs: Any,
    ) -> str:
        """
        Export a DataFrame to a file.

        Args:
            df: DataFrame to export.
            output_path: Destination file path.
            format: Explicit format override. If None, inferred from extension.
            **kwargs: Additional arguments forwarded to the pandas writer.

        Returns:
            Absolute path to the exported file.
        """
        path = Path(output_path).resolve()
        ext = format or path.suffix.lower()

        logger.info("Exporting %d rows to %s (format: %s)", len(df), path.name, ext)

        if ext in (".csv", "csv"):
            df.to_csv(path, index=False, **kwargs)
        elif ext in (".tsv", "tsv"):
            df.to_csv(path, sep="\t", index=False, **kwargs)
        elif ext in (".xlsx", "xlsx"):
            df.to_excel(path, index=False, **kwargs)
        elif ext in (".json", "json"):
            df.to_json(path, orient="records", indent=2, **kwargs)
        elif ext in (".parquet", "parquet"):
            df.to_parquet(path, index=False, **kwargs)
        else:
            raise ValueError(f"Unsupported export format: {ext}")

        return str(path)


# ---------------------------------------------------------------------------
# Utilities
# ---------------------------------------------------------------------------

def _safe_scalar(value: Any) -> Any:
    """Convert numpy/pandas scalar types to Python native types for JSON serialization."""
    if value is None:
        return None
    if isinstance(value, (np.integer,)):
        return int(value)
    if isinstance(value, (np.floating,)):
        v = float(value)
        if np.isnan(v) or np.isinf(v):
            return None
        return v
    if isinstance(value, np.bool_):
        return bool(value)
    if isinstance(value, pd.Timestamp):
        return value.isoformat()
    return value
