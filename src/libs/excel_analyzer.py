"""
GradebookProcessor Module

A comprehensive solution for converting, parsing, and generating PDF reports
from educational gradebooks. Supports Excel (.xls/.xlsx) and CSV formats.

Classes:
    FileMetadata: Dataclass holding institution and subject metadata
    StudentGrades: Dataclass holding student grade information
    FileConverter: Handles file conversion and validation
    GradebookParser: Parses gradebook CSV to extract metadata and student data
    PDFGenerator: Generates French-styled PDF reports for each student
    GradebookProcessor: Main orchestrator for complete pipeline

Author: Educational Report System
"""

import os
import re
import logging
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from reportlab.lib import colors
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import mm
from reportlab.platypus import Paragraph, SimpleDocTemplate, Spacer, Table, TableStyle
import pandas as pd


# ==================== DATA MODELS ====================

@dataclass
class FileMetadata:
    """Metadata extracted from gradebook file header.
    
    Attributes:
        file_name: Original filename
        university_name: Institution name
        program_name: Program/degree name
        materia_name: Subject/course name
        group_name: Class group identifier
        professor_name: Instructor name
        period_name: Academic period (semester/year)
        first_twenty_grade_count: Number of first-block grades (before components)
        first_twenty_grade_numbers: List of first-block grade numbers as strings
        component_grade_numbers: List of component exam numbers as strings
        grade_labels: Dict mapping grade numbers to their labels
    """
    file_name: str
    university_name: str
    program_name: str
    materia_name: str
    group_name: str
    professor_name: str
    period_name: str
    first_twenty_grade_count: int
    first_twenty_grade_numbers: List[str]
    component_grade_numbers: List[str]
    grade_labels: Dict[str, str]
    first_block_weight_label: str = "20%"
    component_weight_labels: Dict[str, str] = field(default_factory=dict)


@dataclass
class StudentGrades:
    """Student grade information.
    
    Attributes:
        student_number: Sequential student number
        student_code: Unique student identifier/code
        student_name: Full student name
        abse: Absences count
        first_twenty_grades: Dict of first-block grades {label: score}
        component_grades: Dict of component exam grades {label: score}
        weighted_components: Dict of weighted component grades {label: weighted_score}
        definitive_grade: Final course grade
    """
    student_number: Optional[int]
    student_code: str
    student_name: str
    abse: Optional[float]
    first_twenty_grades: Dict[str, Optional[float]]
    component_grades: Dict[str, Optional[float]]
    weighted_components: Dict[str, Optional[float]]
    definitive_grade: Optional[float]


# ==================== UTILITY FUNCTIONS ====================

def clean_text(value: Any) -> str:
    """Clean and normalize text values.
    
    Removes whitespace normalization characters, strips leading/trailing spaces,
    and collapses multiple whitespaces to single space.
    
    Args:
        value: Input value (any type)
        
    Returns:
        Cleaned string, empty string if value is NaN
    """
    if pd.isna(value):
        return ""
    text = str(value).replace("\xa0", " ").strip()
    text = re.sub(r"\s+", " ", text)
    return text


def to_float(value: Any) -> Optional[float]:
    """Convert value to float, handling NaN and non-numeric strings.
    
    Args:
        value: Input value
        
    Returns:
        Float value or None if conversion fails
    """
    if pd.isna(value):
        return None
    text = clean_text(value)
    if not text:
        return None
    try:
        return float(text)
    except ValueError:
        return None


def extract_grade_num_from_key(key: str) -> Optional[str]:
    """Extract grade number from grade key string.
    
    Expected format: "123 - Label Text"
    
    Args:
        key: Grade key string
        
    Returns:
        Grade number as string or None
    """
    match = re.match(r"^(\d+)\s*-", key)
    return match.group(1) if match else None


def format_concept_label(label: str) -> str:
    """Format concept label with line breaks on sentence boundaries.
    
    Splits on ". " (period + space) and joins with &lt;br/&gt; so each
    sentence appears on its own line in the ReportLab Paragraph.
    
    Args:
        label: Raw concept label string
        
    Returns:
        Formatted label with per-sentence line breaks
    """
    parts = label.split(". ")
    formatted = []
    for i, part in enumerate(parts):
        part = part.strip()
        if not part:
            continue
        if i < len(parts) - 1:
            formatted.append(part + ".")
        else:
            formatted.append(part)
    return "<br/>".join(formatted)


def extract_label_from_key(key: str) -> str:
    """Extract label from grade key string.
    
    Expected format: "123 - Label Text"
    
    Args:
        key: Grade key string
        
    Returns:
        Label text or original key if parsing fails
    """
    parts = key.split(" - ", 1)
    label = parts[1].strip() if len(parts) == 2 else key
    return format_concept_label(label)


def safe_filename(value: str) -> str:
    """Create filesystem-safe filename from arbitrary string.
    
    Removes special characters, replaces spaces with underscores.
    
    Args:
        value: Input string
        
    Returns:
        Safe filename string (or "student" if empty)
    """
    cleaned = re.sub(r"[^\w\-\s]", "", value, flags=re.UNICODE).strip()
    cleaned = re.sub(r"\s+", "_", cleaned)
    return cleaned or "student"


def format_grade(value: Optional[float]) -> str:
    """Format numeric grade for display.
    
    Formats to 2 decimals, strips trailing zeros and decimal point if not needed.
    
    Args:
        value: Grade value
        
    Returns:
        Formatted grade string, empty string if None
    """
    if value is None:
        return ""
    return f"{value:.2f}".rstrip("0").rstrip(".")


def format_weight_label(value: Any, default: str = "20%") -> str:
    """Format weight values from the spreadsheet as percentage labels."""
    text = clean_text(value)
    if not text:
        return default

    has_percent = "%" in text
    normalized = text.replace("%", "").replace(",", ".").strip()
    try:
        number = float(normalized)
    except ValueError:
        return text

    percent = number if has_percent or number > 1 else number * 100
    return f"{percent:g}%"


# ==================== FILE CONVERTER ====================

class FileConverter:
    """Handles file format conversion and validation.
    
    Converts Excel files (.xls, .xlsx) to CSV format and validates file integrity.
    Collects audit information during processing.
    
    Args:
        logger: Optional logging.Logger instance for detailed logging
    
    Example:
        converter = FileConverter(logger=logging.getLogger(__name__))
        df, audit_report = converter.convert_and_audit('grades.xls')
        if audit_report['status'] == 'success':
            print(f"Converted to: {audit_report['conversion_path']}")
    """
    
    def __init__(self, logger: Optional[logging.Logger] = None):
        """Initialize converter with optional logger.
        
        Args:
            logger: Optional logging.Logger instance
        """
        self.logger = logger
    
    def log_conversion(self, audit_report: Dict[str, Any]) -> None:
        """Log conversion details (called internally, separates logging from main flow).
        
        Args:
            audit_report: Audit report dict with conversion results
        """
        if not self.logger:
            return
        
        status_icon = "✓" if audit_report['status'] == 'success' else "✗"
        self.logger.info(f"{status_icon} File conversion: {audit_report['file_name']}")
        self.logger.info(f"   Format: {audit_report['original_format']} → CSV")
        self.logger.info(f"   Status: {audit_report['status'].upper()}")
        if audit_report['converted']:
            self.logger.info(f"   Output: {audit_report.get('conversion_path', 'N/A')}")
        if audit_report['conversion_errors']:
            self.logger.error(f"   Errors: {audit_report['conversion_errors']}")
    
    @staticmethod
    def convert_and_audit(file_path: str, output_csv: Optional[str] = None) -> Tuple[Optional[pd.DataFrame], Dict[str, Any]]:
        """Convert and audit a gradebook file.
        
        Supports .xls, .xlsx, and .csv formats. Automatically converts Excel to CSV.
        Validates data integrity and reports empty columns/rows.
        
        Args:
            file_path: Path to input file (.xls, .xlsx, or .csv)
            output_csv: Optional output CSV path (auto-generated if not provided)
            
        Returns:
            Tuple of (DataFrame or None, audit_report dict).
            Audit report keys: 'file_name', 'original_format', 'converted',
            'conversion_path', 'row_count', 'column_count', 'status', etc.
        """
        file_path_obj = Path(file_path)
        audit_report = {
            "file_name": file_path_obj.name,
            "original_format": file_path_obj.suffix.lower(),
            "converted": False,
            "conversion_path": "",
            "conversion_errors": [],
            "is_empty": False,
            "is_corrupted": False,
            "corruption_details": [],
            "row_count": 0,
            "column_count": 0,
            "empty_cells_count": 0,
            "null_values_count": 0,
            "warnings": [],
            "status": "success",
        }

        try:
            suffix = file_path_obj.suffix.lower()
            if suffix in [".xls", ".xlsx"]:
                output_path = Path(output_csv) if output_csv else file_path_obj.with_suffix(".csv")
                try:
                    with pd.ExcelFile(file_path_obj) as workbook:
                        df_temp = pd.read_excel(workbook)
                    with open(output_path, "w", encoding="utf-8", newline="") as handle:
                        df_temp.to_csv(handle, index=False)
                    file_path_obj = output_path
                    audit_report["converted"] = True
                    audit_report["conversion_path"] = str(output_path)
                except Exception as exc:
                    audit_report["conversion_errors"].append(str(exc))
                    audit_report["status"] = "conversion_failed"
                    audit_report["is_corrupted"] = True
                    return None, audit_report
            elif suffix == ".csv":
                audit_report["conversion_path"] = str(file_path_obj)
            else:
                audit_report["conversion_errors"].append(f"Unsupported file format: {file_path_obj.suffix}")
                audit_report["status"] = "unsupported_format"
                audit_report["is_corrupted"] = True
                return None, audit_report

            with open(file_path_obj, "r", encoding="utf-8", newline="") as handle:
                df = pd.read_csv(handle)
            audit_report["row_count"] = len(df)
            audit_report["column_count"] = len(df.columns)

            if df.empty:
                audit_report["is_empty"] = True
                audit_report["warnings"].append("DataFrame is empty - contains no data rows")

            null_count = df.isnull().sum().sum()
            audit_report["null_values_count"] = int(null_count)

            empty_cells = (df == "").sum().sum() + df.isna().sum().sum()
            audit_report["empty_cells_count"] = int(empty_cells)

            all_null_cols = df.columns[df.isnull().all()].tolist()
            if all_null_cols:
                audit_report["warnings"].append(f"Columns with all null values: {all_null_cols}")
                df = df.drop(columns=all_null_cols)

            all_null_rows = df[df.isnull().all(axis=1)].index.tolist()
            if all_null_rows:
                audit_report["warnings"].append(f"Found {len(all_null_rows)} rows with all null values")
                df = df.dropna(how="all")

            if audit_report["is_empty"]:
                audit_report["status"] = "empty"

            return df, audit_report

        except Exception as exc:
            audit_report["is_corrupted"] = True
            audit_report["corruption_details"].append(f"Critical error reading file: {exc}")
            audit_report["status"] = "corrupted"
            return None, audit_report
    
    @staticmethod
    def print_audit_report(audit_report: Dict[str, Any]) -> None:
        """Print formatted audit report to console.
        
        Args:
            audit_report: Audit report dict from convert_and_audit()
        """
        print("\n" + "=" * 60)
        print("FILE AUDIT REPORT")
        print("=" * 60)
        print(f"File: {audit_report['file_name']}")
        print(f"Original Format: {audit_report['original_format']}")
        print(f"Status: {audit_report['status'].upper()}")
        if audit_report["converted"]:
            print(f"Converted to .csv: {audit_report.get('conversion_path', 'N/A')}")
        else:
            print(f"File path: {audit_report.get('conversion_path', 'N/A')}")
        if audit_report["conversion_errors"]:
            print(f"Conversion Errors: {audit_report['conversion_errors']}")
        print("=" * 60 + "\n")
        """Convert and audit a gradebook file.
        
        Supports .xls, .xlsx, and .csv formats. Automatically converts Excel to CSV.
        Validates data integrity and reports empty columns/rows.
        
        Args:
            file_path: Path to input file (.xls, .xlsx, or .csv)
            output_csv: Optional output CSV path (auto-generated if not provided)
            
        Returns:
            Tuple of (DataFrame or None, audit_report dict).
            Audit report keys: 'file_name', 'original_format', 'converted',
            'conversion_path', 'row_count', 'column_count', 'status', etc.
        """
        file_path_obj = Path(file_path)
        audit_report = {
            "file_name": file_path_obj.name,
            "original_format": file_path_obj.suffix.lower(),
            "converted": False,
            "conversion_path": "",
            "conversion_errors": [],
            "is_empty": False,
            "is_corrupted": False,
            "corruption_details": [],
            "row_count": 0,
            "column_count": 0,
            "empty_cells_count": 0,
            "null_values_count": 0,
            "warnings": [],
            "status": "success",
        }

        try:
            suffix = file_path_obj.suffix.lower()
            if suffix in [".xls", ".xlsx"]:
                output_path = Path(output_csv) if output_csv else file_path_obj.with_suffix(".csv")
                try:
                    with pd.ExcelFile(file_path_obj) as workbook:
                        df_temp = pd.read_excel(workbook)
                    with open(output_path, "w", encoding="utf-8", newline="") as handle:
                        df_temp.to_csv(handle, index=False)
                    file_path_obj = output_path
                    audit_report["converted"] = True
                    audit_report["conversion_path"] = str(output_path)
                except Exception as exc:
                    audit_report["conversion_errors"].append(str(exc))
                    audit_report["status"] = "conversion_failed"
                    audit_report["is_corrupted"] = True
                    return None, audit_report
            elif suffix == ".csv":
                audit_report["conversion_path"] = str(file_path_obj)
            else:
                audit_report["conversion_errors"].append(f"Unsupported file format: {file_path_obj.suffix}")
                audit_report["status"] = "unsupported_format"
                audit_report["is_corrupted"] = True
                return None, audit_report

            with open(file_path_obj, "r", encoding="utf-8", newline="") as handle:
                df = pd.read_csv(handle)
            audit_report["row_count"] = len(df)
            audit_report["column_count"] = len(df.columns)

            if df.empty:
                audit_report["is_empty"] = True
                audit_report["warnings"].append("DataFrame is empty - contains no data rows")

            null_count = df.isnull().sum().sum()
            audit_report["null_values_count"] = int(null_count)

            empty_cells = (df == "").sum().sum() + df.isna().sum().sum()
            audit_report["empty_cells_count"] = int(empty_cells)

            all_null_cols = df.columns[df.isnull().all()].tolist()
            if all_null_cols:
                audit_report["warnings"].append(f"Columns with all null values: {all_null_cols}")
                df = df.drop(columns=all_null_cols)

            all_null_rows = df[df.isnull().all(axis=1)].index.tolist()
            if all_null_rows:
                audit_report["warnings"].append(f"Found {len(all_null_rows)} rows with all null values")
                df = df.dropna(how="all")

            if audit_report["is_empty"]:
                audit_report["status"] = "empty"

            return df, audit_report

        except Exception as exc:
            audit_report["is_corrupted"] = True
            audit_report["corruption_details"].append(f"Critical error reading file: {exc}")
            audit_report["status"] = "corrupted"
            return None, audit_report
    
    @staticmethod
    def print_audit_report(audit_report: Dict[str, Any]) -> None:
        """Print formatted audit report to console.
        
        Args:
            audit_report: Audit report dict from convert_and_audit()
        """
        print("\n" + "=" * 60)
        print("FILE AUDIT REPORT")
        print("=" * 60)
        print(f"File: {audit_report['file_name']}")
        print(f"Original Format: {audit_report['original_format']}")
        print(f"Status: {audit_report['status'].upper()}")
        if audit_report["converted"]:
            print(f"Converted to .csv: {audit_report.get('conversion_path', 'N/A')}")
        else:
            print(f"File path: {audit_report.get('conversion_path', 'N/A')}")
        if audit_report["conversion_errors"]:
            print(f"Conversion Errors: {audit_report['conversion_errors']}")
        print("=" * 60 + "\n")


# ==================== GRADEBOOK PARSER ====================

class GradebookParser:
    """Parses gradebook CSV files and extracts metadata and student data.
    
    Dynamically detects the number of first-block grades, component exams,
    and grade labels. Handles variable gradebook structures.
    
    Args:
        logger: Optional logging.Logger instance for detailed logging
    
    Attributes:
        raw_df: Raw DataFrame as read from CSV
        file_metadata: Extracted FileMetadata
        students: List of StudentGrades objects
        student_table: Summary DataFrame of all students
        
    Example:
        parser = GradebookParser(logger=logging.getLogger(__name__))
        metadata, students, table = parser.parse('grades.csv')
        print(f"Found {len(students)} students with {metadata.first_twenty_grade_count} first-block grades")
    """
    
    def __init__(self, logger: Optional[logging.Logger] = None):
        """Initialize parser state.
        
        Args:
            logger: Optional logging.Logger instance
        """
        self.logger = logger
        self.raw_df: Optional[pd.DataFrame] = None
        self.file_metadata: Optional[FileMetadata] = None
        self.students: List[StudentGrades] = []
        self.student_table: Optional[pd.DataFrame] = None
    
    def log_metadata(self) -> None:
        """Log extracted metadata (called internally).
        
        Logs institution info, grades structure, and parsing results.
        """
        if not self.logger or not self.file_metadata:
            return
        
        self.logger.info("📋 Metadata Extracted:")
        self.logger.info(f"   University: {self.file_metadata.university_name}")
        self.logger.info(f"   Program: {self.file_metadata.program_name}")
        self.logger.info(f"   Subject: {self.file_metadata.materia_name}")
        self.logger.info(f"   Group: {self.file_metadata.group_name}")
        self.logger.info(f"   Professor: {self.file_metadata.professor_name}")
        self.logger.info(f"   Period: {self.file_metadata.period_name}")
        self.logger.info(f"   First-block grades: {self.file_metadata.first_twenty_grade_count}")
        self.logger.info(f"   Component exams: {len(self.file_metadata.component_grade_numbers)}")
    
    def log_students(self) -> None:
        """Log parsed students info (called internally).
        
        Logs student count and summary statistics.
        """
        if not self.logger or not self.students:
            return
        
        self.logger.info(f"👥 Students Parsed: {len(self.students)}")
        if self.students:
            self.logger.debug(f"   First student: {self.students[0].student_name} ({self.students[0].student_code})")
            self.logger.debug(f"   Last student: {self.students[-1].student_name} ({self.students[-1].student_code})")

    def _extract_grade_labels(self) -> Dict[str, str]:
        """Extract grade labels from footer rows in gradebook.
        
        Looks for grade labels throughout the entire DataFrame.

        The gradebooks are not perfectly consistent in their footer labels:
        examples include "1. LABEL", "5, LABEL", "10.. LABEL", and
        "15 LABEL". All of those forms should map to the numeric grade.
        
        Returns:
            Dict mapping grade numbers to labels
        """
        labels: Dict[str, str] = {}
        pattern = re.compile(r"^(\d+)\s*[\.,]*\s+(.+)$")

        for row_idx in range(len(self.raw_df)):
            for col_idx in range(self.raw_df.shape[1]):
                cell = clean_text(self.raw_df.iat[row_idx, col_idx])
                if not cell:
                    continue

                match = pattern.match(cell)
                if match:
                    grade_num = match.group(1)
                    label = match.group(2).strip()
                    if re.search(r"[A-Za-zÀ-ÿ]", label):
                        labels[grade_num] = label

        return labels

    def parse(self, csv_file: str) -> Tuple[FileMetadata, List[StudentGrades], pd.DataFrame]:
        """Parse gradebook CSV file.
        
        Detects the header row dynamically and supports both the 2019
        gradebook layout and the 2025 layout where course metadata starts
        in the first column.
        
        Args:
            csv_file: Path to gradebook CSV file
            
        Returns:
            Tuple of (FileMetadata, List[StudentGrades], summary DataFrame)
            
        Raises:
            ValueError: If parsing fails
        """
        with open(csv_file, "r", encoding="utf-8", newline="") as handle:
            self.raw_df = pd.read_csv(handle, header=None, dtype=str, keep_default_na=False)

        try:
            header_row_idx = self._find_header_row()
        except ValueError:
            return self._parse_carnet_blocks(csv_file)

        weights_row_idx = max(0, header_row_idx - 1)

        header = [clean_text(v) for v in self.raw_df.iloc[header_row_idx].tolist()]
        weights = [clean_text(v) for v in self.raw_df.iloc[weights_row_idx].tolist()]

        student_no_col = self._find_header_col(header, {"no", "no.", "n°", "nº"}, default=0)
        student_code_col = self._find_header_col(header, {"code"}, default=1)
        student_name_col = self._find_name_col(header, default=2)
        abse_col = self._find_header_prefix_col(header, ("abse",), default=3)
        first_grade_col = abse_col + 1

        # Find the 'moy' column (end of first-block grades)
        try:
            moy_col = next(i for i, token in enumerate(weights) if token.lower() == "moy")
        except StopIteration:
            moy_col = first_grade_col
            while moy_col < len(header) and clean_text(header[moy_col]):
                moy_col += 1

        first_twenty_cols = [
            col for col in range(first_grade_col, moy_col)
            if re.fullmatch(r"\d+(?:\.\d+)?", clean_text(header[col]))
        ]

        # Extract first-block grade numbers
        first_twenty_grade_numbers = []
        for col in first_twenty_cols:
            token = clean_text(header[col])
            if token:
                first_twenty_grade_numbers.append(str(int(float(token))))

        # Extract component exam grade numbers
        component_grade_cols = []
        component_grade_numbers = []
        for i in range(moy_col + 1, len(header)):
            token = clean_text(header[i])
            if re.fullmatch(r"\d+(?:\.\d+)?", token):
                component_grade_cols.append(i)
                component_grade_numbers.append(str(int(float(token))))

        weighted_first_col = moy_col + 1
        first_block_weight_label = format_weight_label(
            weights[weighted_first_col] if weighted_first_col < len(weights) else "",
        )

        component_weight_labels: Dict[str, str] = {}
        for col, grade_num in zip(component_grade_cols, component_grade_numbers):
            weighted_col = col + 1
            component_weight_labels[grade_num] = format_weight_label(
                weights[weighted_col] if weighted_col < len(weights) else "",
                default=first_block_weight_label,
            )

        # Find definitive grade column
        definitive_col = None
        for i, token in enumerate(header):
            if token.lower().startswith("défin") or token.lower().startswith("defin"):
                definitive_col = i
                break

        grade_labels = self._extract_grade_labels()

        # Create FileMetadata
        self.file_metadata = FileMetadata(
            file_name=Path(csv_file).name,
            university_name=self._first_non_empty_in_row(0),
            program_name=self._first_non_empty_in_row(1),
            materia_name=self._first_non_empty_in_row(3),
            group_name=self._extract_group(
                self._first_matching_in_row(4, r"\b(groupe|classe|grupo|class)\b")
            ),
            professor_name=self._extract_professor(
                self._first_matching_in_row(5, r"professeur")
            ),
            period_name=self._extract_period(
                self._first_matching_in_row(6, r"(p[ée]riode|\d{4})")
            ),
            first_twenty_grade_count=len(first_twenty_grade_numbers),
            first_twenty_grade_numbers=first_twenty_grade_numbers,
            component_grade_numbers=component_grade_numbers,
            grade_labels=grade_labels,
            first_block_weight_label=first_block_weight_label,
            component_weight_labels=component_weight_labels,
        )

        # Parse student rows
        student_rows = self.raw_df.iloc[header_row_idx + 1:].copy()
        
        def row_value(row: pd.Series, col: Optional[int]) -> Any:
            if col is None or col not in row.index:
                return ""
            return row[col]

        def parse_student_number(row: pd.Series) -> Optional[int]:
            number = clean_text(row_value(row, student_no_col))
            if re.fullmatch(r"\d+", number):
                return int(number)
            return None

        def is_student_row(row: pd.Series) -> bool:
            number = parse_student_number(row)
            code = clean_text(row_value(row, student_code_col))
            name = clean_text(row_value(row, student_name_col))
            return bool(name) and (number is not None or bool(code))

        student_rows = student_rows[student_rows.apply(is_student_row, axis=1)]

        self.students = []

        enumeration_fallbacks: List[str] = []

        for expected_student_number, (_, row) in enumerate(student_rows.iterrows(), start=1):
            spreadsheet_student_number = parse_student_number(row)
            student_number = expected_student_number
            if spreadsheet_student_number == expected_student_number:
                student_number = spreadsheet_student_number
            else:
                student_name = clean_text(row_value(row, student_name_col))
                observed = (
                    str(spreadsheet_student_number)
                    if spreadsheet_student_number is not None else "blank/non-numeric"
                )
                enumeration_fallbacks.append(
                    f"{student_name or 'unnamed student'}: {observed} -> {expected_student_number}"
                )

            # Parse first-block grades
            first_twenty_grades: Dict[str, Optional[float]] = {}
            for col in first_twenty_cols:
                grade_num = str(int(float(clean_text(header[col]))))
                label = grade_labels.get(grade_num, f"Grade {grade_num}")
                first_twenty_grades[f"{grade_num} - {label}"] = to_float(row[col])

            # Parse component grades
            component_grades: Dict[str, Optional[float]] = {}
            weighted_components: Dict[str, Optional[float]] = {}

            first_block_average = to_float(row[moy_col]) if moy_col < len(row) else None
            first_block_weighted = to_float(row[weighted_first_col]) if weighted_first_col < len(row) else None
            component_grades[f"1-{len(first_twenty_grade_numbers)} average"] = first_block_average
            weighted_components[
                f"1-{len(first_twenty_grade_numbers)} weighted {first_block_weight_label}"
            ] = first_block_weighted

            for col in component_grade_cols:
                grade_num = str(int(float(clean_text(header[col]))))
                label = grade_labels.get(grade_num, f"Grade {grade_num}")
                component_grades[f"{grade_num} - {label}"] = to_float(row[col])

                weighted_col = col + 1
                if weighted_col < len(row):
                    weight_label = component_weight_labels.get(grade_num, first_block_weight_label)
                    weighted_components[f"{grade_num} weighted {weight_label}"] = to_float(row[weighted_col])

            definitive_grade = to_float(row[definitive_col]) if definitive_col is not None else None

            student = StudentGrades(
                student_number=student_number,
                student_code=clean_text(row_value(row, student_code_col)),
                student_name=clean_text(row_value(row, student_name_col)),
                abse=to_float(row_value(row, abse_col)),
                first_twenty_grades=first_twenty_grades,
                component_grades=component_grades,
                weighted_components=weighted_components,
                definitive_grade=definitive_grade,
            )
            self.students.append(student)

        if self.logger and enumeration_fallbacks:
            preview = "; ".join(enumeration_fallbacks[:5])
            extra = len(enumeration_fallbacks) - 5
            if extra > 0:
                preview = f"{preview}; and {extra} more"
            self.logger.warning("Student enumeration corrected from row order: %s", preview)

        # Create summary table
        self.student_table = pd.DataFrame({
            "student_number": [s.student_number for s in self.students],
            "student_code": [s.student_code for s in self.students],
            "student_name": [s.student_name for s in self.students],
            "abse": [s.abse for s in self.students],
            "definitive_grade": [s.definitive_grade for s in self.students],
        })
        
        # Log results
        self.log_metadata()
        self.log_students()

        return self.file_metadata, self.students, self.student_table

    def _find_header_row(self) -> int:
        """Find the row containing the student/grade column headers."""
        for row_idx in range(len(self.raw_df)):
            row = [clean_text(v).lower() for v in self.raw_df.iloc[row_idx].tolist()]
            if (
                self._find_header_col(row, {"no", "no.", "n°", "nº"}, default=None) is not None
                and self._find_header_col(row, {"code"}, default=None) is not None
                and self._find_name_col(row, default=None) is not None
                and self._find_header_prefix_col(row, ("abse",), default=None) is not None
            ):
                return row_idx

        raise ValueError("Could not find the student header row in the gradebook")

    def _parse_carnet_blocks(self, csv_file: str) -> Tuple[FileMetadata, List[StudentGrades], pd.DataFrame]:
        """Parse per-student carnet sheets exported from report-card templates."""
        header_rows = self._find_carnet_header_rows()
        if not header_rows:
            raise ValueError("Could not find a supported gradebook or carnet layout")

        first_header = header_rows[0]
        header = [clean_text(v) for v in self.raw_df.iloc[first_header].tolist()]
        no_col = self._find_header_col(header, {"no", "no.", "n°", "nº"}, default=0)
        item_col = self._find_header_col(header, {"item"}, default=1)
        note_col = self._find_header_col(header, {"note"}, default=2)
        moy_col = self._find_header_col(header, {"moy"}, default=3)
        weighted_col = self._first_numeric_header_col(header, start_after=moy_col) or 4
        first_block_weight_label = format_weight_label(header[weighted_col])

        first_items = self._collect_carnet_items(first_header, no_col, item_col)
        first_block_numbers = [
            grade_num for grade_num, label in first_items
            if not self._is_component_label(label)
        ]
        component_numbers = [
            grade_num for grade_num, label in first_items
            if self._is_component_label(label)
        ]
        grade_labels = {grade_num: label for grade_num, label in first_items}
        component_weight_labels = {
            grade_num: first_block_weight_label for grade_num in component_numbers
        }

        subject = self._first_non_empty_in_row(max(0, first_header - 3))
        period = self._extract_period(Path(csv_file).stem)
        sheet_label = self._extract_sheet_label_from_csv_name(csv_file)

        self.file_metadata = FileMetadata(
            file_name=Path(csv_file).name,
            university_name="",
            program_name="",
            materia_name=subject,
            group_name=sheet_label,
            professor_name="",
            period_name=period,
            first_twenty_grade_count=len(first_block_numbers),
            first_twenty_grade_numbers=first_block_numbers,
            component_grade_numbers=component_numbers,
            grade_labels=grade_labels,
            first_block_weight_label=first_block_weight_label,
            component_weight_labels=component_weight_labels,
        )

        self.students = []
        for student_number, header_row_idx in enumerate(header_rows, start=1):
            student = self._parse_carnet_student(
                student_number,
                header_row_idx,
                no_col,
                item_col,
                note_col,
                moy_col,
                weighted_col,
                first_block_numbers,
                component_numbers,
                grade_labels,
                first_block_weight_label,
                component_weight_labels,
            )
            self.students.append(student)

        self.student_table = pd.DataFrame({
            "student_number": [s.student_number for s in self.students],
            "student_code": [s.student_code for s in self.students],
            "student_name": [s.student_name for s in self.students],
            "abse": [s.abse for s in self.students],
            "definitive_grade": [s.definitive_grade for s in self.students],
        })

        self.log_metadata()
        self.log_students()

        return self.file_metadata, self.students, self.student_table

    def _parse_carnet_student(
        self,
        student_number: int,
        header_row_idx: int,
        no_col: int,
        item_col: int,
        note_col: int,
        moy_col: int,
        weighted_col: int,
        first_block_numbers: List[str],
        component_numbers: List[str],
        grade_labels: Dict[str, str],
        first_block_weight_label: str,
        component_weight_labels: Dict[str, str],
    ) -> StudentGrades:
        student_name = self._first_non_empty_in_row(max(0, header_row_idx - 2))
        abse = self._extract_absences(self._first_non_empty_in_row(max(0, header_row_idx - 1)))

        first_block_set = set(first_block_numbers)
        component_set = set(component_numbers)
        first_twenty_grades: Dict[str, Optional[float]] = {}
        component_grades: Dict[str, Optional[float]] = {}
        weighted_components: Dict[str, Optional[float]] = {}
        first_block_average: Optional[float] = None
        first_block_weighted: Optional[float] = None
        definitive_grade: Optional[float] = None

        for row_idx in range(header_row_idx + 1, len(self.raw_df)):
            row = self.raw_df.iloc[row_idx]
            item_text = clean_text(row[item_col]) if item_col in row.index else ""
            if not item_text:
                if self._row_is_blank(row):
                    break
                continue

            if self._is_final_carnet_row(item_text):
                definitive_grade = to_float(row[weighted_col]) if weighted_col in row.index else None
                break

            grade_num = clean_text(row[no_col]) if no_col in row.index else ""
            if not re.fullmatch(r"\d+", grade_num):
                continue

            grade_num = str(int(grade_num))
            label = grade_labels.get(grade_num, item_text)

            if grade_num in first_block_set:
                first_twenty_grades[f"{grade_num} - {label}"] = (
                    to_float(row[note_col]) if note_col in row.index else None
                )
                if first_block_average is None and moy_col in row.index:
                    first_block_average = to_float(row[moy_col])
                if first_block_weighted is None and weighted_col in row.index:
                    first_block_weighted = to_float(row[weighted_col])
            elif grade_num in component_set:
                component_grades[f"{grade_num} - {label}"] = (
                    to_float(row[moy_col]) if moy_col in row.index else None
                )
                weight_label = component_weight_labels.get(grade_num, first_block_weight_label)
                weighted_components[f"{grade_num} weighted {weight_label}"] = (
                    to_float(row[weighted_col]) if weighted_col in row.index else None
                )

        component_grades[f"1-{len(first_block_numbers)} average"] = first_block_average
        weighted_components[
            f"1-{len(first_block_numbers)} weighted {first_block_weight_label}"
        ] = first_block_weighted

        return StudentGrades(
            student_number=student_number,
            student_code="",
            student_name=student_name,
            abse=abse,
            first_twenty_grades=first_twenty_grades,
            component_grades=component_grades,
            weighted_components=weighted_components,
            definitive_grade=definitive_grade,
        )

    def _find_carnet_header_rows(self) -> List[int]:
        header_rows: List[int] = []
        for row_idx in range(len(self.raw_df)):
            row = [clean_text(v).lower() for v in self.raw_df.iloc[row_idx].tolist()]
            has_number = self._find_header_col(row, {"no", "no.", "n°", "nº"}, default=None) is not None
            has_item = self._find_header_col(row, {"item"}, default=None) is not None
            has_note = self._find_header_col(row, {"note"}, default=None) is not None
            has_moy = self._find_header_col(row, {"moy"}, default=None) is not None
            if has_number and has_item and has_note and has_moy:
                header_rows.append(row_idx)
        return header_rows

    def _collect_carnet_items(
        self,
        header_row_idx: int,
        no_col: int,
        item_col: int,
    ) -> List[Tuple[str, str]]:
        items: List[Tuple[str, str]] = []
        for row_idx in range(header_row_idx + 1, len(self.raw_df)):
            row = self.raw_df.iloc[row_idx]
            item_text = clean_text(row[item_col]) if item_col in row.index else ""
            if self._is_final_carnet_row(item_text):
                break
            grade_num = clean_text(row[no_col]) if no_col in row.index else ""
            if re.fullmatch(r"\d+", grade_num) and item_text:
                items.append((str(int(grade_num)), item_text))
            elif self._row_is_blank(row):
                break
        return items

    @staticmethod
    def _first_numeric_header_col(header: List[str], start_after: Optional[int]) -> Optional[int]:
        start = (start_after if start_after is not None else -1) + 1
        for idx in range(start, len(header)):
            if to_float(header[idx]) is not None:
                return idx
        return None

    @staticmethod
    def _is_component_label(label: str) -> bool:
        return "examen" in clean_text(label).lower()

    @staticmethod
    def _is_final_carnet_row(value: Any) -> bool:
        return clean_text(value).lower().startswith("note du cours")

    @staticmethod
    def _extract_absences(value: Any) -> Optional[float]:
        match = re.search(r"absences?\s*:\s*(\d+(?:[.,]\d+)?)", clean_text(value), flags=re.IGNORECASE)
        if not match:
            return None
        return to_float(match.group(1).replace(",", "."))

    @staticmethod
    def _row_is_blank(row: pd.Series) -> bool:
        return all(not clean_text(value) for value in row.tolist())

    @staticmethod
    def _extract_sheet_label_from_csv_name(csv_file: str) -> str:
        stem = Path(csv_file).stem
        match = re.search(r"_(FRAN.+)$", stem, flags=re.IGNORECASE)
        return match.group(1).replace("_", " ") if match else ""

    @staticmethod
    def _normalize_header_token(value: Any) -> str:
        return clean_text(value).lower().strip()

    @classmethod
    def _find_header_col(cls, header: List[str], names: set[str], default: Optional[int]) -> Optional[int]:
        normalized_names = {cls._normalize_header_token(name) for name in names}
        for idx, token in enumerate(header):
            if cls._normalize_header_token(token) in normalized_names:
                return idx
        return default

    @classmethod
    def _find_header_prefix_col(
        cls,
        header: List[str],
        prefixes: Tuple[str, ...],
        default: Optional[int],
    ) -> Optional[int]:
        for idx, token in enumerate(header):
            normalized = cls._normalize_header_token(token)
            if any(normalized.startswith(prefix) for prefix in prefixes):
                return idx
        return default

    @classmethod
    def _find_name_col(cls, header: List[str], default: Optional[int]) -> Optional[int]:
        for idx, token in enumerate(header):
            normalized = cls._normalize_header_token(token)
            if (
                "nom" in normalized
                or "prénom" in normalized
                or "prenom" in normalized
                or "étudiant" in normalized
                or "etudiant" in normalized
                or "estudiante" in normalized
            ):
                return idx
        return default

    def _first_non_empty_in_row(self, row_idx: int) -> str:
        if row_idx >= len(self.raw_df):
            return ""
        for value in self.raw_df.iloc[row_idx].tolist():
            text = clean_text(value)
            if text:
                return text
        return ""

    def _first_matching_in_row(self, row_idx: int, pattern: str) -> str:
        if row_idx >= len(self.raw_df):
            return ""
        regex = re.compile(pattern, flags=re.IGNORECASE)
        for value in self.raw_df.iloc[row_idx].tolist():
            text = clean_text(value)
            if text and regex.search(text):
                return text
        return self._first_non_empty_in_row(row_idx)

    @staticmethod
    def _extract_group(value: Any) -> str:
        """Extract group number from cell value."""
        text = clean_text(value)
        match = re.search(r"(\d+)", text)
        return match.group(1) if match else ""

    @staticmethod
    def _extract_professor(value: Any) -> str:
        """Extract professor name, removing prefix."""
        text = clean_text(value)
        return re.sub(r"^PROFESSEUR\s*:\s*", "", text, flags=re.IGNORECASE)

    @staticmethod
    def _extract_period(value: Any) -> str:
        """Extract period year from cell value."""
        text = clean_text(value)
        match = re.search(r"(\d{4})", text)
        return match.group(1) if match else text


# ==================== PDF GENERATOR ====================

class PDFGenerator:
    """Generates French-styled PDF grade reports for students.
    
    Creates individual PDFs for each student with institution metadata,
    student information, and a formatted grade table.
    
    Args:
        logger: Optional logging.Logger instance for detailed logging
    
    Attributes:
        file_metadata: FileMetadata instance
        students: List of StudentGrades instances
        output_dir: Output directory for PDF files
        created_files: List of generated PDF file paths
        
    Example:
        generator = PDFGenerator(metadata, students, output_dir='reports', 
                               logger=logging.getLogger(__name__))
        pdf_files = generator.generate()
        print(f"Generated {len(pdf_files)} PDFs")
    """
    
    def __init__(self, file_metadata: FileMetadata, students: List[StudentGrades],
                 output_dir: str = "student_pdfs", logger: Optional[logging.Logger] = None,
                 timestamp_iso: Optional[str] = None):
        """Initialize PDF generator.
        
        Args:
            file_metadata: FileMetadata instance with institution info
            students: List of StudentGrades instances
            output_dir: Directory to save PDFs (created if needed)
            logger: Optional logging.Logger instance
            timestamp_iso: Optional ISO format timestamp to display as date
        """
        self.logger = logger
        self.file_metadata = file_metadata
        self.students = students
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.created_files: List[str] = []
        self.timestamp_iso = timestamp_iso
    
    def log_pdf_generation(self, count: int) -> None:
        """Log PDF generation results (called internally).
        
        Args:
            count: Number of PDFs generated
        """
        if not self.logger:
            return
        
        self.logger.info(f"📄 PDFs Generated: {count}")
        self.logger.info(f"   Output directory: {self.output_dir}")
        if count > 0:
            self.logger.debug(f"   First PDF: {Path(self.created_files[0]).name}")
            if count > 1:
                self.logger.debug(f"   Last PDF: {Path(self.created_files[-1]).name}")

    def generate(self, use_parallel: bool = True) -> List[str]:
        """Generate one PDF per student.
        
        Creates French-styled PDFs with institution metadata, student info,
        and a formatted grade table with color-coded sections.
        
        Args:
            use_parallel: When True, generate PDFs in parallel when possible

        Returns:
            List of generated PDF file paths
        """
        styles = getSampleStyleSheet()
        styles_dict = self._create_styles(styles)
        
        if use_parallel and len(self.students) > 1:
            max_workers = min(4, os.cpu_count() or 1, len(self.students))
            created_paths: list[Optional[Path]] = [None] * len(self.students)
            with ThreadPoolExecutor(max_workers=max_workers) as executor:
                futures = {
                    executor.submit(self._generate_student_pdf, student, styles_dict): idx
                    for idx, student in enumerate(self.students)
                }
                for future in as_completed(futures):
                    idx = futures[future]
                    created_paths[idx] = future.result()
            self.created_files = [str(path) for path in created_paths if path is not None]
        else:
            for student in self.students:
                pdf_path = self._generate_student_pdf(student, styles_dict)
                self.created_files.append(str(pdf_path))
        
        # Log results
        self.log_pdf_generation(len(self.created_files))

        return self.created_files

    def _generate_student_pdf(self, student: StudentGrades, 
                             styles_dict: Dict[str, ParagraphStyle]) -> Path:
        """Generate PDF for a single student.
        
        Args:
            student: StudentGrades instance
            styles_dict: Dictionary of ParagraphStyle objects
            
        Returns:
            Path to generated PDF file
        """
        student_num = student.student_number if student.student_number is not None else 0
        student_name_safe = safe_filename(student.student_name)
        pdf_name = f"{student_num:02d}_{student_name_safe}.pdf"
        pdf_path = self.output_dir / pdf_name

        doc = SimpleDocTemplate(
            str(pdf_path),
            pagesize=A4,
            leftMargin=12 * mm,
            rightMargin=12 * mm,
            topMargin=10 * mm,
            bottomMargin=10 * mm,
            title="RÉSULTAT FINAL",
            author=self.file_metadata.professor_name,
        )

        story = self._build_document_story(student, styles_dict)
        doc.build(story)

        return pdf_path

    def _build_document_story(self, student: StudentGrades, 
                             styles_dict: Dict[str, ParagraphStyle]) -> List:
        """Build document content story.
        
        Args:
            student: StudentGrades instance
            styles_dict: Dictionary of ParagraphStyle objects
            
        Returns:
            List of Platypus elements for document
        """
        story = []

        # Date section (if provided)
        if self.timestamp_iso:
            info_parts = []
            # Convert ISO format to readable date
            try:
                from datetime import datetime
                date_obj = datetime.fromisoformat(self.timestamp_iso.replace('Z', '+00:00'))
                date_str = date_obj.strftime("%d/%m/%Y")
                info_parts.append(f"Date: {date_str}")
            except:
                info_parts.append(f"Date: {self.timestamp_iso}")
            if info_parts:
                story.append(Paragraph(" | ".join(info_parts), styles_dict["header"]))
                story.append(Spacer(1, 2 * mm))

        # Header section
        story.append(Paragraph("RÉSULTAT FINAL", styles_dict["title"]))
        story.append(Paragraph(self.file_metadata.university_name, styles_dict["header"]))
        story.append(Paragraph(self.file_metadata.program_name, styles_dict["header"]))
        story.append(Paragraph(self.file_metadata.materia_name, styles_dict["header"]))
        story.append(Paragraph(
            f"GROUPE {self.file_metadata.group_name} - {self.file_metadata.period_name}",
            styles_dict["header"]
        ))
        story.append(Paragraph(
            f"PROFESSEUR : {self.file_metadata.professor_name}",
            styles_dict["header"]
        ))
        story.append(Spacer(1, 2 * mm))

        # Student info
        story.append(Paragraph(student.student_name.upper(), styles_dict["name"]))
        story.append(Paragraph(f"Code étudiant : {student.student_code}", styles_dict["header"]))
        story.append(Paragraph(f"Absences : {format_grade(student.abse)}", styles_dict["header"]))
        story.append(Spacer(1, 2 * mm))

        # Grade table
        table = self._build_grade_table(student, styles_dict)
        story.append(table)
        story.append(Spacer(1, 2 * mm))

        # Footer
        story.append(Paragraph(
            "Document généré automatiquement.",
            ParagraphStyle("Foot", parent=styles_dict["header"], fontSize=8)
        ))

        return story

    def _build_grade_table(self, student: StudentGrades, 
                          styles_dict: Dict[str, ParagraphStyle]) -> Table:
        """Build grade table for student.
        
        Args:
            student: StudentGrades instance
            styles_dict: Dictionary of ParagraphStyle objects
            
        Returns:
            Formatted Table element
        """
        raw_rows, row_meta = self._build_student_rows(student)
        
        # Create table data with header
        table_data = [["CONCEPT", "NOTE", "%", "MOY"]]
        
        for row_idx, (concept, note, pct, moy) in enumerate(raw_rows):
            cell_style = self._style_for_grade_row(row_idx, row_meta, styles_dict)
            table_data.append([
                Paragraph(concept, cell_style),
                Paragraph(note, cell_style),
                Paragraph(pct, cell_style),
                Paragraph(moy, cell_style),
            ])

        # Create table with styles
        table = Table(
            table_data,
            colWidths=[112 * mm, 20 * mm, 18 * mm, 20 * mm],
            repeatRows=1,
        )

        # Apply styling
        table_styles = self._create_table_styles(row_meta)
        table.setStyle(TableStyle(table_styles))

        return table

    @staticmethod
    def _style_for_grade_row(
        row_idx: int,
        row_meta: Dict[str, int],
        styles_dict: Dict[str, ParagraphStyle],
    ) -> ParagraphStyle:
        if row_idx == row_meta["final_row"]:
            return styles_dict["cell_final"]
        if row_meta["first_block_start"] <= row_idx <= row_meta["first_block_end"]:
            return styles_dict["cell_first_block"]
        if row_meta["component_start"] <= row_idx <= row_meta["component_end"]:
            return styles_dict["cell_component"]
        return styles_dict["cell"]

    def _build_student_rows(self, student: StudentGrades) -> Tuple[List[List[str]], Dict[str, int]]:
        """Build row data for grade table.
        
        Args:
            student: StudentGrades instance
            
        Returns:
            Tuple of (row_data list, row_indices dict)
        """
        rows: List[List[str]] = []

        # First-block grades
        ordered_first = sorted(
            student.first_twenty_grades.items(),
            key=lambda item: int(extract_grade_num_from_key(item[0]) or 0),
        )

        first_block_start = 0
        first_block_weight_label = self.file_metadata.first_block_weight_label
        for key, grade in ordered_first:
            concept_label = extract_label_from_key(key)
            rows.append([concept_label, format_grade(grade), first_block_weight_label, ""])

        # Apply MOY to first row only
        weighted_avg_key = (
            f"1-{self.file_metadata.first_twenty_grade_count} weighted {first_block_weight_label}"
        )
        weighted_avg_value = student.weighted_components.get(weighted_avg_key)

        first_block_end = len(rows) - 1
        if rows:
            rows[first_block_start][3] = format_grade(weighted_avg_value)

        # Component exams
        component_start = len(rows)
        for grade_num in self.file_metadata.component_grade_numbers:
            concept = format_concept_label(
                self.file_metadata.grade_labels.get(grade_num, f"NOTE {grade_num}")
            )

            component_value = None
            for key, val in student.component_grades.items():
                if key.startswith(f"{grade_num} -"):
                    component_value = val
                    break

            weight_label = self.file_metadata.component_weight_labels.get(
                grade_num,
                first_block_weight_label,
            )
            weighted_key = f"{grade_num} weighted {weight_label}"
            weighted_value = student.weighted_components.get(weighted_key)

            rows.append([
                concept,
                format_grade(component_value),
                weight_label,
                format_grade(weighted_value),
            ])

        # Final grade
        final_row_index = len(rows)
        rows.append(["NOTE DÉFINITIVE", "", "", format_grade(student.definitive_grade)])

        meta = {
            "first_block_start": first_block_start,
            "first_block_end": first_block_end,
            "component_start": component_start,
            "component_end": final_row_index - 1,
            "final_row": final_row_index,
        }

        return rows, meta

    def _create_styles(self, sample_styles) -> Dict[str, ParagraphStyle]:
        """Create paragraph styles for document.
        
        Args:
            sample_styles: getSampleStyleSheet()
            
        Returns:
            Dictionary of ParagraphStyle objects
        """
        return {
            "title": ParagraphStyle(
                "TitleFR",
                parent=sample_styles["Title"],
                fontName="Helvetica-Bold",
                fontSize=14,
                alignment=1,
                spaceAfter=6,
            ),
            "header": ParagraphStyle(
                "HeaderFR",
                parent=sample_styles["Normal"],
                fontName="Helvetica",
                fontSize=9,
                leading=11,
            ),
            "name": ParagraphStyle(
                "NameFR",
                parent=sample_styles["Normal"],
                fontName="Helvetica-Bold",
                fontSize=10,
                spaceAfter=2,
            ),
            "cell": ParagraphStyle(
                "CellFR",
                parent=sample_styles["Normal"],
                fontName="Helvetica",
                fontSize=8,
                leading=10,
            ),
            "cell_first_block": ParagraphStyle(
                "CellFirstBlockFR",
                parent=sample_styles["Normal"],
                fontName="Helvetica",
                fontSize=8,
                leading=10,
                textColor=colors.HexColor("#7A2CB8"),
            ),
            "cell_component": ParagraphStyle(
                "CellComponentFR",
                parent=sample_styles["Normal"],
                fontName="Helvetica",
                fontSize=8,
                leading=10,
                textColor=colors.HexColor("#3F5F23"),
            ),
            "cell_final": ParagraphStyle(
                "CellFinalFR",
                parent=sample_styles["Normal"],
                fontName="Helvetica-Bold",
                fontSize=8,
                leading=10,
                textColor=colors.red,
            ),
        }

    def _create_table_styles(self, row_meta: Dict[str, int]) -> List[Tuple]:
        """Create table styling directives.
        
        Args:
            row_meta: Row index metadata
            
        Returns:
            List of TableStyle directives
        """
        first_start = 1 + row_meta["first_block_start"]
        first_end = 1 + row_meta["first_block_end"]
        comp_start = 1 + row_meta["component_start"]
        comp_end = 1 + row_meta["component_end"]
        final_row = 1 + row_meta["final_row"]

        return [
            ("BACKGROUND", (0, 0), (-1, 0), colors.lightgrey),
            ("TEXTCOLOR", (0, 0), (-1, 0), colors.black),
            ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
            ("FONTSIZE", (0, 0), (-1, 0), 8),
            ("ALIGN", (1, 0), (-1, -1), "CENTER"),
            ("ALIGN", (0, 0), (0, -1), "LEFT"),
            ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
            ("GRID", (0, 0), (-1, -1), 0.5, colors.black),
            ("LEFTPADDING", (0, 0), (-1, -1), 3),
            ("RIGHTPADDING", (0, 0), (-1, -1), 3),
            ("TOPPADDING", (0, 0), (-1, -1), 2),
            ("BOTTOMPADDING", (0, 0), (-1, -1), 2),
            ("TEXTCOLOR", (0, first_start), (-1, first_end), colors.HexColor("#7A2CB8")),
            ("TEXTCOLOR", (0, comp_start), (-1, comp_end), colors.HexColor("#3F5F23")),
            ("LINEABOVE", (0, comp_start), (-1, comp_start), 1.2, colors.black),
            ("SPAN", (2, first_start), (2, first_end)),
            ("SPAN", (3, first_start), (3, first_end)),
            ("ALIGN", (2, first_start), (3, first_end), "CENTER"),
            ("SPAN", (0, final_row), (2, final_row)),
            ("ALIGN", (0, final_row), (2, final_row), "RIGHT"),
            ("TEXTCOLOR", (0, final_row), (3, final_row), colors.red),
            ("FONTNAME", (0, final_row), (3, final_row), "Helvetica-Bold"),
            ("FONTSIZE", (0, final_row), (3, final_row), 10),
            ("LINEABOVE", (0, final_row), (-1, final_row), 1.0, colors.black),
        ]


# ==================== MAIN PROCESSOR ====================

class GradebookProcessor:
    """Main orchestrator for complete gradebook processing pipeline.
    
    Coordinates file conversion, parsing, and PDF generation in a single
    fluent interface. Maintains state at each step for recovery and debugging.
    
    Args:
        logger: Optional logging.Logger instance for detailed logging
    
    Attributes:
        source_file: Path to source file
        audit_report: File conversion audit report
        file_metadata: Extracted FileMetadata
        students: List of parsed StudentGrades
        student_table: Summary DataFrame
        pdf_files: List of generated PDF file paths
        
    Example:
        logger = logging.getLogger(__name__)
        processor = GradebookProcessor('notas.xlsx', logger=logger)
        processor.convert()
        processor.parse()
        processor.generate_pdfs()
        print(f"Success: {len(processor.pdf_files)} PDFs created")
    """
    
    def __init__(self, source_file: str, sheet_name: Optional[str] = None, 
                 logger: Optional[logging.Logger] = None):
        """Initialize processor.
        
        Args:
            source_file: Path to gradebook file (.xls, .xlsx, or .csv)
            sheet_name: Optional sheet name (for Excel files with multiple sheets)
            logger: Optional logging.Logger instance
        """
        self.logger = logger
        self.source_file = source_file
        self.sheet_name = sheet_name
        self.audit_report: Optional[Dict[str, Any]] = None
        self.csv_file: Optional[str] = None
        self.file_metadata: Optional[FileMetadata] = None
        self.students: List[StudentGrades] = []
        self.student_table: Optional[pd.DataFrame] = None
        self.pdf_files: List[str] = []

    @staticmethod
    def get_sheet_names(source_file: str, include_empty: bool = False) -> List[str]:
        """Get list of sheet names from Excel file.
        
        Args:
            source_file: Path to Excel file (.xls or .xlsx)
            include_empty: Return every workbook sheet when True. By default,
                only gradebook-like sheets are returned.
            
        Returns:
            List of sheet names, or empty list if file is CSV or cannot be read.
        """
        if not source_file.lower().endswith(('.xls', '.xlsx')):
            return []
        
        try:
            with pd.ExcelFile(source_file) as xls:
                if include_empty:
                    return xls.sheet_names
                return [
                    sheet_name
                    for sheet_name in xls.sheet_names
                    if GradebookProcessor._sheet_has_gradebook_header(xls, sheet_name)
                ]
        except Exception:
            return []

    @staticmethod
    def _sheet_has_gradebook_header(xls: pd.ExcelFile, sheet_name: str) -> bool:
        try:
            df = pd.read_excel(
                xls,
                sheet_name=sheet_name,
                header=None,
                dtype=str,
                keep_default_na=False,
                nrows=15,
            )
        except Exception:
            return False

        if df.empty:
            return False

        for _, row in df.iterrows():
            tokens = [clean_text(value).lower() for value in row.tolist()]
            has_number = any(token in {"no", "no.", "n°", "nº"} for token in tokens)
            has_code = "code" in tokens
            has_student_name = any(
                "nom" in token
                or "prénom" in token
                or "prenom" in token
                or "étudiant" in token
                or "etudiant" in token
                or "estudiante" in token
                for token in tokens
            )
            has_absences = any(token.startswith("abse") for token in tokens)
            if has_number and has_code and has_student_name and has_absences:
                return True

            has_item = "item" in tokens
            has_note = "note" in tokens
            has_moy = "moy" in tokens
            if has_number and has_item and has_note and has_moy:
                return True

        return False

    def convert(self) -> bool:
        """Convert source file to CSV and validate.
        
        Returns:
            True if successful, False otherwise
        """
        converter = FileConverter(logger=self.logger)
        
        # If sheet_name is specified, read only that sheet for Excel files
        if self.sheet_name and self.source_file.lower().endswith(('.xls', '.xlsx')):
            try:
                with pd.ExcelFile(self.source_file) as workbook:
                    raw_df = pd.read_excel(workbook, sheet_name=self.sheet_name)
                # Create audit report for sheet-based processing
                self.audit_report = {
                    "file_name": Path(self.source_file).name,
                    "sheet_name": self.sheet_name,
                    "original_format": Path(self.source_file).suffix.lower(),
                    "converted": False,
                    "conversion_path": None,
                    "conversion_errors": [],
                    "is_empty": False,
                    "is_corrupted": False,
                    "corruption_details": [],
                    "row_count": len(raw_df),
                    "column_count": len(raw_df.columns),
                    "empty_cells_count": 0,
                    "null_values_count": 0,
                    "warnings": [],
                    "status": "success",
                }
                
                # Save as CSV for consistent processing
                csv_output = Path(self.source_file).stem + f"_{self.sheet_name}.csv"
                with open(csv_output, "w", encoding="utf-8", newline="") as handle:
                    raw_df.to_csv(handle, index=False)
                self.audit_report["conversion_path"] = csv_output
                self.csv_file = csv_output
                
                converter.log_conversion(self.audit_report)
                FileConverter.print_audit_report(self.audit_report)
                return True
                
            except Exception as e:
                self.audit_report = {
                    "file_name": Path(self.source_file).name,
                    "sheet_name": self.sheet_name,
                    "status": "failed",
                    "conversion_errors": [str(e)],
                }
                print(f"Error reading sheet '{self.sheet_name}': {e}")
                return False
        else:
            # Use standard conversion for CSV or no sheet specified
            raw_df, self.audit_report = converter.convert_and_audit(self.source_file)
            
            if self.audit_report['status'] != 'success':
                FileConverter.print_audit_report(self.audit_report)
                return False
            
            converter.log_conversion(self.audit_report)
            FileConverter.print_audit_report(self.audit_report)
            self.csv_file = self.audit_report['conversion_path']
            return True

    def parse(self) -> bool:
        """Parse CSV file to extract metadata and student data.
        
        Returns:
            True if successful, False otherwise
        """
        if self.csv_file is None:
            print("Error: Must call convert() first")
            return False

        try:
            parser = GradebookParser(logger=self.logger)
            self.file_metadata, self.students, self.student_table = parser.parse(self.csv_file)
            return True
        except Exception as e:
            print(f"Parse error: {e}")
            return False

    def generate_pdfs(self, output_dir: str = "student_pdfs",
                      timestamp_iso: Optional[str] = None,
                      use_parallel: bool = True) -> bool:
        """Generate PDF reports for all students.
        
        Args:
            output_dir: Directory to save PDFs
            timestamp_iso: Optional ISO format timestamp to display as date
            use_parallel: When True, generate PDFs in parallel if supported
            use_parallel: When True, generate PDFs in parallel if supported
            
        Returns:
            True if successful, False otherwise
        """
        if self.file_metadata is None or not self.students:
            print("Error: Must call parse() first")
            return False

        try:
            generator = PDFGenerator(
                self.file_metadata,
                self.students,
                output_dir,
                logger=self.logger,
                timestamp_iso=timestamp_iso,
            )
            self.pdf_files = generator.generate(use_parallel=use_parallel)
            return True
        except Exception as e:
            print(f"PDF generation error: {e}")
            return False

    def run_pipeline(self, output_dir: str = "student_pdfs",
                     timestamp_iso: Optional[str] = None,
                     use_parallel: bool = True) -> bool:
        """Execute complete processing pipeline: convert → parse → generate PDFs.
        
        Args:
            output_dir: Directory to save PDFs
            timestamp_iso: Optional ISO format timestamp to display as date
            use_parallel: When True, generate PDFs in parallel if supported
            
        Returns:
            True if all steps successful, False otherwise
        """
        return (
            self.convert() and
            self.parse() and
            self.generate_pdfs(output_dir, timestamp_iso=timestamp_iso, use_parallel=use_parallel)
        )

    def summary(self) -> None:
        """Print processing summary to console."""
        print("\n" + "=" * 70)
        print("PROCESSING SUMMARY")
        print("=" * 70)
        if self.file_metadata:
            print(f"\nInstitution: {self.file_metadata.university_name}")
            print(f"Subject: {self.file_metadata.materia_name}")
            print(f"Group: {self.file_metadata.group_name}")
            print(f"Period: {self.file_metadata.period_name}")
            print(f"Professor: {self.file_metadata.professor_name}")
            print(f"First-block grades: {self.file_metadata.first_twenty_grade_count}")
            print(f"Component exams: {len(self.file_metadata.component_grade_numbers)}")
        
        print(f"\nStudents processed: {len(self.students)}")
        print(f"PDFs generated: {len(self.pdf_files)}")
        
        if self.pdf_files:
            print(f"\nFirst 3 PDFs:")
            for pdf in self.pdf_files[:3]:
                print(f"  - {pdf}")
        
        print("=" * 70)
