"""Tests for the pipeline builder module."""
import json
import os
import pytest
import pandas as pd

from backend.pipeline.builder import PipelineBuilder, PipelineStep, PipelineDefinition


@pytest.fixture
def builder():
    """Create a PipelineBuilder instance."""
    return PipelineBuilder()


@pytest.fixture
def sample_csv(tmp_path):
    """Create a sample CSV for pipeline testing."""
    filepath = tmp_path / "input.csv"
    df = pd.DataFrame({
        "name": ["Alice", "Bob", "Charlie", "Diana"],
        "department": ["Eng", "Sales", "Eng", "Marketing"],
        "salary": [75000, 65000, 80000, 60000],
    })
    df.to_csv(filepath, index=False)
    return str(filepath)


class TestPipelineStep:
    """Test PipelineStep creation."""

    def test_create_step(self):
        """Test creating a pipeline step."""
        step = PipelineStep(
            name="load_data",
            type="source",
            config={"path": "data.csv", "format": "csv"},
        )
        assert step.name == "load_data"
        assert step.type == "source"

    def test_step_to_dict(self):
        """Test converting step to dictionary."""
        step = PipelineStep(
            name="filter",
            type="transform",
            config={"column": "status", "value": "active"},
        )
        d = step.to_dict()
        assert d["name"] == "filter"
        assert d["type"] == "transform"
        assert d["config"]["column"] == "status"


class TestPipelineDefinition:
    """Test PipelineDefinition creation and serialization."""

    def test_create_definition(self):
        """Test creating a pipeline definition."""
        pipeline = PipelineDefinition(
            name="test_pipeline",
            description="A test pipeline",
            steps=[
                PipelineStep("load", "source", {"path": "data.csv"}),
                PipelineStep("filter", "transform", {"column": "x", "value": "y"}),
                PipelineStep("save", "destination", {"path": "output.csv"}),
            ],
        )
        assert pipeline.name == "test_pipeline"
        assert len(pipeline.steps) == 3

    def test_definition_to_json(self):
        """Test serializing pipeline to JSON."""
        pipeline = PipelineDefinition(
            name="json_test",
            description="Testing JSON serialization",
            steps=[
                PipelineStep("load", "source", {"path": "data.csv"}),
            ],
        )
        json_str = pipeline.to_json()
        parsed = json.loads(json_str)
        assert parsed["name"] == "json_test"
        assert len(parsed["steps"]) == 1

    def test_definition_from_json(self):
        """Test deserializing pipeline from JSON."""
        json_str = json.dumps({
            "name": "loaded_pipeline",
            "description": "Loaded from JSON",
            "steps": [
                {"name": "load", "type": "source", "config": {"path": "data.csv"}},
            ],
        })
        pipeline = PipelineDefinition.from_json(json_str)
        assert pipeline.name == "loaded_pipeline"
        assert len(pipeline.steps) == 1


class TestPipelineBuilder:
    """Test the PipelineBuilder."""

    def test_init(self, builder):
        """Test builder initialization."""
        assert builder is not None

    def test_add_source_step(self, builder):
        """Test adding a source step."""
        builder.add_step("load", "source", {"path": "data.csv", "format": "csv"})
        assert len(builder.steps) == 1
        assert builder.steps[0].type == "source"

    def test_add_transform_step(self, builder):
        """Test adding a transform step."""
        builder.add_step("load", "source", {"path": "data.csv"})
        builder.add_step("filter", "transform", {"column": "status", "value": "active"})
        assert len(builder.steps) == 2

    def test_add_destination_step(self, builder):
        """Test adding a destination step."""
        builder.add_step("load", "source", {"path": "data.csv"})
        builder.add_step("save", "destination", {"path": "output.csv", "format": "csv"})
        assert len(builder.steps) == 2

    def test_build_definition(self, builder):
        """Test building a pipeline definition."""
        builder.add_step("load", "source", {"path": "data.csv"})
        builder.add_step("filter", "transform", {"column": "x", "value": "y"})
        builder.add_step("save", "destination", {"path": "out.csv"})
        definition = builder.build("my_pipeline", "A test pipeline")
        assert isinstance(definition, PipelineDefinition)
        assert definition.name == "my_pipeline"

    def test_execute_pipeline(self, builder, sample_csv, tmp_path):
        """Test executing a simple pipeline."""
        output_path = str(tmp_path / "output.csv")
        builder.add_step("load", "source", {"path": sample_csv, "format": "csv"})
        builder.add_step("filter", "transform", {"column": "department", "value": "Eng"})
        builder.add_step("save", "destination", {"path": output_path, "format": "csv"})

        result = builder.execute()
        assert result["success"] is True
        assert os.path.exists(output_path)

        # Verify filtered output
        df = pd.read_csv(output_path)
        assert len(df) == 2
        assert all(df["department"] == "Eng")

    def test_execute_empty_pipeline(self, builder):
        """Test executing an empty pipeline raises error."""
        with pytest.raises(ValueError):
            builder.execute()

    def test_clear_steps(self, builder):
        """Test clearing pipeline steps."""
        builder.add_step("load", "source", {"path": "data.csv"})
        builder.clear()
        assert len(builder.steps) == 0

    def test_save_pipeline(self, builder, tmp_path):
        """Test saving a pipeline definition to file."""
        builder.add_step("load", "source", {"path": "data.csv"})
        definition = builder.build("save_test", "Testing save")
        filepath = str(tmp_path / "pipeline.json")
        builder.save_definition(definition, filepath)
        assert os.path.exists(filepath)

    def test_load_pipeline(self, builder, tmp_path):
        """Test loading a pipeline definition from file."""
        # Save first
        builder.add_step("load", "source", {"path": "data.csv"})
        definition = builder.build("load_test", "Testing load")
        filepath = str(tmp_path / "pipeline.json")
        builder.save_definition(definition, filepath)

        # Load
        loaded = builder.load_definition(filepath)
        assert loaded.name == "load_test"
        assert len(loaded.steps) == 1
