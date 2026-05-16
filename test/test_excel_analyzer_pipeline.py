from pathlib import Path
import sys

import pytest


ROOT_DIR = Path(__file__).resolve().parents[1]
SRC_DIR = ROOT_DIR / "src"
TEMPLATE_DIR = ROOT_DIR / "templates"

sys.path.insert(0, str(SRC_DIR))

from libs.excel_analyzer import GradebookProcessor  # noqa: E402


EXPECTED_EXCEL_SHEETS = {
    "CARNET DE NOTES FRANÇAIS IV 2025.xlsx": {
        "FRANCAIS IV MATIN": 23,
        "FRANÇAIS IV APRÈS-MIDI": 11,
    },
    "NOTAS FRANCÉS 2019-1.xls": {
        "FRANCAIS I-01": 23,
        "FRANCAIS II-01": 19,
        "EXPRESSION ORALE 03": 10,
    },
    "NOTAS FRANCÉS 2025-2.xls": {
        "FRANCAIS IV LE MATIN": 27,
        "FRANCAIS IV L'APRÈS-MIDI": 20,
        "LENGUA FRANCESA (2)": 20,
    },
}

EXPECTED_CSV_STUDENTS = {
    "NOTAS FRANCÉS 2019-1_EXPRESSION ORALE 03.csv": 10,
    "NOTAS FRANCÉS 2019-1_FRANCAIS I-01.csv": 23,
    "NOTAS FRANCÉS 2019-1_FRANCAIS II-01.csv": 19,
    "NOTAS FRANCÉS 2025-2_FRANCAIS IV LE MATIN.csv": 27,
}

EMPTY_CSV_FILES = {
    "NOTAS FRANCÉS 2019-1_Hoja1.csv",
}


def excel_template_files() -> list[Path]:
    return sorted([*TEMPLATE_DIR.glob("*.xls"), *TEMPLATE_DIR.glob("*.xlsx")])


def csv_template_files() -> list[Path]:
    return sorted(TEMPLATE_DIR.glob("*.csv"))


def assert_pdf_outputs(output_dir: Path, expected_count: int) -> None:
    pdfs = sorted(output_dir.glob("*.pdf"))
    assert len(pdfs) == expected_count
    for pdf in pdfs:
        content = pdf.read_bytes()
        assert content.startswith(b"%PDF")
        assert len(content) > 1_000


def assert_student_numbers_are_sequential(processor: GradebookProcessor) -> None:
    numbers = [student.student_number for student in processor.students]
    assert numbers == list(range(1, len(numbers) + 1))


def run_pipeline(
    source_file: Path,
    output_dir: Path,
    monkeypatch: pytest.MonkeyPatch,
    sheet_name: str | None = None,
) -> GradebookProcessor:
    output_dir.mkdir(parents=True, exist_ok=True)
    monkeypatch.chdir(output_dir.parent)

    processor = GradebookProcessor(str(source_file), sheet_name=sheet_name)
    assert processor.run_pipeline(output_dir=str(output_dir))
    assert processor.file_metadata is not None
    assert processor.students
    assert len(processor.pdf_files) == len(processor.students)
    assert_student_numbers_are_sequential(processor)
    assert_pdf_outputs(output_dir, len(processor.students))
    return processor


def test_all_expected_templates_are_present() -> None:
    expected_files = set(EXPECTED_EXCEL_SHEETS) | set(EXPECTED_CSV_STUDENTS) | EMPTY_CSV_FILES
    actual_files = {path.name for path in [*excel_template_files(), *csv_template_files()]}
    assert expected_files <= actual_files


@pytest.mark.parametrize("source_file", excel_template_files(), ids=lambda path: path.name)
def test_excel_templates_generate_pdfs_for_every_supported_sheet(
    source_file: Path,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    expected_sheets = EXPECTED_EXCEL_SHEETS[source_file.name]
    sheet_names = GradebookProcessor.get_sheet_names(str(source_file))

    assert sheet_names == list(expected_sheets)
    assert set(sheet_names) <= set(GradebookProcessor.get_sheet_names(str(source_file), include_empty=True))

    for sheet_name in sheet_names:
        output_dir = tmp_path / source_file.stem / sheet_name.replace("/", "-")
        processor = run_pipeline(source_file, output_dir, monkeypatch, sheet_name=sheet_name)
        assert len(processor.students) == expected_sheets[sheet_name]
        assert processor.file_metadata.materia_name
        assert processor.file_metadata.first_twenty_grade_count > 0


@pytest.mark.parametrize("source_file", csv_template_files(), ids=lambda path: path.name)
def test_csv_templates_generate_pdfs_or_report_empty_files(
    source_file: Path,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    if source_file.name in EMPTY_CSV_FILES:
        processor = GradebookProcessor(str(source_file))
        assert not processor.convert()
        assert processor.audit_report is not None
        assert processor.audit_report["status"] in {"corrupted", "empty"}
        return

    expected_students = EXPECTED_CSV_STUDENTS[source_file.name]
    output_dir = tmp_path / source_file.stem
    processor = run_pipeline(source_file, output_dir, monkeypatch)

    assert len(processor.students) == expected_students
    assert processor.file_metadata.materia_name


def test_bad_professor_enumeration_falls_back_to_row_order(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    source_file = TEMPLATE_DIR / "NOTAS FRANCÉS 2025-2_FRANCAIS IV LE MATIN.csv"
    lines = source_file.read_text(encoding="utf-8").splitlines()
    header_idx = next(i for i, line in enumerate(lines) if line.startswith("No.,CODE,"))

    first_student_idx = header_idx + 1
    lines[first_student_idx] = lines[first_student_idx].replace("4,49416,", "4,49416,", 1)
    lines[first_student_idx + 1] = lines[first_student_idx + 1].replace("2,45862,", ",45862,", 1)
    lines[first_student_idx + 2] = lines[first_student_idx + 2].replace("3,41106,", "oops,41106,", 1)

    broken_csv = tmp_path / "broken_enumeration.csv"
    broken_csv.write_text("\n".join(lines) + "\n", encoding="utf-8")

    output_dir = tmp_path / "broken_enumeration_pdfs"
    processor = run_pipeline(broken_csv, output_dir, monkeypatch)

    assert len(processor.students) == EXPECTED_CSV_STUDENTS[source_file.name]
    assert [student.student_number for student in processor.students[:3]] == [1, 2, 3]
