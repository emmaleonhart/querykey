"""Tests for the file organizer module."""
import os
import tempfile
import pytest

from backend.core.file_organizer import FileOrganizer, FileCategory, OrganizeRequest


@pytest.fixture
def organizer():
    """Create a FileOrganizer instance."""
    return FileOrganizer()


@pytest.fixture
def messy_directory(tmp_path):
    """Create a temporary directory with various file types."""
    # Documents
    (tmp_path / "report.pdf").write_text("fake pdf")
    (tmp_path / "notes.docx").write_bytes(b"fake docx")
    (tmp_path / "readme.txt").write_text("readme content")

    # Spreadsheets
    (tmp_path / "budget.xlsx").write_bytes(b"fake xlsx")
    (tmp_path / "data.csv").write_text("a,b,c\n1,2,3")

    # Images
    (tmp_path / "photo.jpg").write_bytes(b"fake jpg")
    (tmp_path / "logo.png").write_bytes(b"fake png")

    # Code
    (tmp_path / "script.py").write_text("print('hello')")
    (tmp_path / "app.js").write_text("console.log('hello')")

    # Data
    (tmp_path / "config.json").write_text('{"key": "value"}')
    (tmp_path / "data.xml").write_text("<root/>")

    # Duplicates - same content
    (tmp_path / "report_copy.pdf").write_text("fake pdf")

    return tmp_path


class TestFileOrganizer:
    """Test suite for FileOrganizer."""

    def test_init(self, organizer):
        """Test organizer initialization."""
        assert organizer is not None

    def test_scan_directory(self, organizer, messy_directory):
        """Test scanning a directory returns categorized files."""
        result = organizer.scan_directory(str(messy_directory))
        assert result is not None
        assert "categories" in result
        assert "total_files" in result
        assert result["total_files"] > 0

    def test_categorize_documents(self, organizer, messy_directory):
        """Test that documents are correctly categorized."""
        result = organizer.scan_directory(str(messy_directory))
        categories = result["categories"]
        assert "Documents" in categories
        doc_files = [f["name"] for f in categories["Documents"]]
        assert "report.pdf" in doc_files
        assert "readme.txt" in doc_files

    def test_categorize_spreadsheets(self, organizer, messy_directory):
        """Test that spreadsheets are correctly categorized."""
        result = organizer.scan_directory(str(messy_directory))
        categories = result["categories"]
        assert "Spreadsheets" in categories
        sheet_files = [f["name"] for f in categories["Spreadsheets"]]
        assert "budget.xlsx" in sheet_files
        assert "data.csv" in sheet_files

    def test_categorize_images(self, organizer, messy_directory):
        """Test that images are correctly categorized."""
        result = organizer.scan_directory(str(messy_directory))
        categories = result["categories"]
        assert "Images" in categories
        img_files = [f["name"] for f in categories["Images"]]
        assert "photo.jpg" in img_files
        assert "logo.png" in img_files

    def test_categorize_code(self, organizer, messy_directory):
        """Test that code files are correctly categorized."""
        result = organizer.scan_directory(str(messy_directory))
        categories = result["categories"]
        assert "Code" in categories
        code_files = [f["name"] for f in categories["Code"]]
        assert "script.py" in code_files
        assert "app.js" in code_files

    def test_detect_duplicates(self, organizer, messy_directory):
        """Test duplicate file detection."""
        result = organizer.find_duplicates(str(messy_directory))
        assert "groups" in result
        # report.pdf and report_copy.pdf have the same content
        assert len(result["groups"]) > 0

    def test_suggest_structure(self, organizer, messy_directory):
        """Test organization structure suggestion."""
        suggestion = organizer.suggest_structure(str(messy_directory))
        assert suggestion is not None
        assert "proposed_folders" in suggestion

    def test_organize_files(self, organizer, messy_directory):
        """Test actually organizing files into folders."""
        request = OrganizeRequest(source_directory=str(messy_directory), dry_run=True)
        organized = organizer.organize(request)
        assert organized is not None
        assert "operations" in organized
        assert len(organized["operations"]) > 0

    def test_organize_dry_run_no_changes(self, organizer, messy_directory):
        """Test that dry run doesn't actually move files."""
        original_files = set(os.listdir(messy_directory))
        request = OrganizeRequest(source_directory=str(messy_directory), dry_run=True)
        organizer.organize(request)
        after_files = set(os.listdir(messy_directory))
        assert original_files == after_files

    def test_empty_directory(self, organizer, tmp_path):
        """Test scanning an empty directory."""
        result = organizer.scan_directory(str(tmp_path))
        assert result is not None
        assert result["total_files"] == 0

    def test_nonexistent_directory(self, organizer):
        """Test scanning a nonexistent directory."""
        with pytest.raises(ValueError):
            organizer.scan_directory("/nonexistent/directory")

    def test_file_category_enum(self):
        """Test FileCategory has expected values."""
        assert FileCategory.DOCUMENTS is not None
        assert FileCategory.SPREADSHEETS is not None
        assert FileCategory.IMAGES is not None
        assert FileCategory.CODE is not None
        assert FileCategory.DATA is not None


class TestFileCategoryClassification:
    """Test file extension to category mapping."""

    def test_pdf_is_document(self, organizer):
        """Test PDF files are classified as documents."""
        assert organizer.classify_extension(".pdf") == FileCategory.DOCUMENTS

    def test_xlsx_is_spreadsheet(self, organizer):
        """Test XLSX files are classified as spreadsheets."""
        assert organizer.classify_extension(".xlsx") == FileCategory.SPREADSHEETS

    def test_csv_is_spreadsheet(self, organizer):
        """Test CSV files are classified as spreadsheets."""
        assert organizer.classify_extension(".csv") == FileCategory.SPREADSHEETS

    def test_jpg_is_image(self, organizer):
        """Test JPG files are classified as images."""
        assert organizer.classify_extension(".jpg") == FileCategory.IMAGES

    def test_py_is_code(self, organizer):
        """Test Python files are classified as code."""
        assert organizer.classify_extension(".py") == FileCategory.CODE

    def test_json_is_data(self, organizer):
        """Test JSON files are classified as data."""
        assert organizer.classify_extension(".json") == FileCategory.DATA

    def test_unknown_extension(self, organizer):
        """Test unknown extensions get OTHER category."""
        assert organizer.classify_extension(".xyz123") == FileCategory.OTHER
