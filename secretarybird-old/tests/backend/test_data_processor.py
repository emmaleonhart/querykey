"""Tests for the data processor module."""
import os
import json
import pytest
import pandas as pd

from backend.core.data_processor import (
    DataProcessor,
    TransformRequest,
    FilterSpec,
    SortSpec,
    AggregateSpec,
    MergeSpec,
)


@pytest.fixture
def processor():
    """Create a DataProcessor instance."""
    return DataProcessor()


@pytest.fixture
def sample_csv(tmp_path):
    """Create a sample CSV file."""
    filepath = tmp_path / "sample.csv"
    df = pd.DataFrame({
        "name": ["Alice", "Bob", "Charlie", "Diana", "Eve"],
        "age": [25, 30, 35, 28, 42],
        "department": ["Engineering", "Sales", "Engineering", "Marketing", "Sales"],
        "salary": [75000, 65000, 80000, 60000, 70000],
    })
    df.to_csv(filepath, index=False)
    return str(filepath)


@pytest.fixture
def sample_json(tmp_path):
    """Create a sample JSON file."""
    filepath = tmp_path / "sample.json"
    data = [
        {"name": "Alice", "score": 95},
        {"name": "Bob", "score": 87},
        {"name": "Charlie", "score": 92},
    ]
    filepath.write_text(json.dumps(data))
    return str(filepath)


@pytest.fixture
def sample_excel(tmp_path):
    """Create a sample Excel file."""
    filepath = tmp_path / "sample.xlsx"
    df = pd.DataFrame({
        "product": ["Widget", "Gadget", "Doohickey"],
        "price": [9.99, 19.99, 4.99],
        "quantity": [100, 50, 200],
    })
    df.to_excel(filepath, index=False)
    return str(filepath)


class TestDataProcessor:
    """Test suite for DataProcessor."""

    def test_init(self, processor):
        """Test processor initialization."""
        assert processor is not None

    def test_load_csv(self, processor, sample_csv):
        """Test loading a CSV file."""
        df = processor.load(sample_csv)
        assert isinstance(df, pd.DataFrame)
        assert len(df) == 5
        assert "name" in df.columns

    def test_load_json(self, processor, sample_json):
        """Test loading a JSON file."""
        df = processor.load(sample_json)
        assert isinstance(df, pd.DataFrame)
        assert len(df) == 3

    def test_load_excel(self, processor, sample_excel):
        """Test loading an Excel file."""
        df = processor.load(sample_excel)
        assert isinstance(df, pd.DataFrame)
        assert len(df) == 3

    def test_load_nonexistent(self, processor):
        """Test loading a nonexistent file."""
        with pytest.raises(FileNotFoundError):
            processor.load("/nonexistent/file.csv")

    def test_load_unsupported(self, processor, tmp_path):
        """Test loading an unsupported format."""
        bad_file = tmp_path / "file.xyz"
        bad_file.write_text("data")
        with pytest.raises(ValueError):
            processor.load(str(bad_file))

    def test_filter_data(self, processor, sample_csv):
        """Test filtering data."""
        df = processor.load(sample_csv)
        request = TransformRequest(
            file_path="dummy",
            filters=[FilterSpec(column="department", operator="eq", value="Engineering")],
        )
        result = processor.transform(df, request)
        assert len(result) == 2
        assert all(result["department"] == "Engineering")

    def test_sort_data(self, processor, sample_csv):
        """Test sorting data."""
        df = processor.load(sample_csv)
        request = TransformRequest(
            file_path="dummy",
            sort=[SortSpec(column="salary", ascending=False)],
        )
        result = processor.transform(df, request)
        assert result.iloc[0]["salary"] == 80000

    def test_aggregate_data(self, processor, sample_csv):
        """Test aggregating data."""
        df = processor.load(sample_csv)
        request = TransformRequest(
            file_path="dummy",
            aggregate=AggregateSpec(
                group_by=["department"],
                aggregations={"salary": ["mean"]},
            ),
        )
        result = processor.transform(df, request)
        assert isinstance(result, pd.DataFrame)
        assert len(result) == 3  # Engineering, Sales, Marketing

    def test_aggregate_sum(self, processor, sample_csv):
        """Test sum aggregation."""
        df = processor.load(sample_csv)
        request = TransformRequest(
            file_path="dummy",
            aggregate=AggregateSpec(
                group_by=["department"],
                aggregations={"salary": ["sum"]},
            ),
        )
        result = processor.transform(df, request)
        eng_row = result[result["department"] == "Engineering"]
        eng_sum = eng_row["salary_sum"].values[0]
        assert eng_sum == 155000  # 75000 + 80000

    def test_profile_data(self, processor, sample_csv):
        """Test data profiling."""
        df = processor.load(sample_csv)
        profile = processor.profile(df)
        assert "shape" in profile
        assert "columns" in profile
        assert profile["shape"]["rows"] == 5
        assert profile["shape"]["columns"] == 4

    def test_profile_column_stats(self, processor, sample_csv):
        """Test column-level statistics in profile."""
        df = processor.load(sample_csv)
        profile = processor.profile(df)
        salary_profile = profile["columns"]["salary"]
        assert "dtype" in salary_profile
        assert "null_count" in salary_profile
        assert salary_profile["null_count"] == 0

    def test_export_csv(self, processor, sample_csv, tmp_path):
        """Test exporting to CSV."""
        df = processor.load(sample_csv)
        output = str(tmp_path / "output.csv")
        processor.export(df, output, format="csv")
        assert os.path.exists(output)
        reloaded = pd.read_csv(output)
        assert len(reloaded) == 5

    def test_export_json(self, processor, sample_csv, tmp_path):
        """Test exporting to JSON."""
        df = processor.load(sample_csv)
        output = str(tmp_path / "output.json")
        processor.export(df, output, format="json")
        assert os.path.exists(output)

    def test_export_excel(self, processor, sample_csv, tmp_path):
        """Test exporting to Excel."""
        df = processor.load(sample_csv)
        output = str(tmp_path / "output.xlsx")
        processor.export(df, output, format="xlsx")
        assert os.path.exists(output)

    def test_schema_inference(self, processor, sample_csv):
        """Test schema inference."""
        df = processor.load(sample_csv)
        schema = processor.infer_schema(df)
        assert isinstance(schema, dict)
        assert "name" in schema
        assert "salary" in schema

    def test_merge_dataframes(self, processor, tmp_path):
        """Test merging two DataFrames."""
        df1 = pd.DataFrame({"id": [1, 2, 3], "name": ["A", "B", "C"]})
        df2 = pd.DataFrame({"id": [1, 2, 3], "score": [90, 85, 95]})

        # Save the right DataFrame to a temp file since MergeSpec requires a file path
        right_file = tmp_path / "right.csv"
        df2.to_csv(right_file, index=False)

        request = TransformRequest(
            file_path="dummy",
            merge=MergeSpec(right_file_path=str(right_file), on=["id"]),
        )
        result = processor.transform(df1, request)
        assert len(result) == 3
        assert "name" in result.columns
        assert "score" in result.columns
