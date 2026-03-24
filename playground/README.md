# Gradebook Processor

A comprehensive solution for converting, parsing, and generating PDF reports from educational gradebooks. Supports both single sheets and multiple sheets from Excel files.

## Files

- **production.py**: Main module with refactored production-ready code (984 lines)
  - `FileConverter`: Handles file conversion (.xls, .xlsx → .csv)
  - `GradebookParser`: Parses CSV and extracts metadata + student data
  - `PDFGenerator`: Generates French-styled PDF reports
  - `GradebookProcessor`: Main orchestrator for complete pipeline
  - `FileMetadata`, `StudentGrades`: Data models

- **test.py**: Basic example script with 4 usage scenarios
- **examples_multi_sheet.py**: Process all sheets from Excel file (52 students, 3 courses)
- **example_francais_i.py**: Process FRANÇAIS I sheet (23 students)
- **example_francais_ii.py**: Process FRANÇAIS II sheet (19 students)
- **example_expression_orale.py**: Process EXPRESSION ORALE sheet (10 students)
- **example_interactive.py**: Interactive menu to select sheets
- **test.ipynb**: Jupyter notebook example

## Quick Start

### Option 1: Process All Sheets (Recommended)

```bash
python examples_multi_sheet.py
```

Automatically processes all 3 sheets and generates PDFs by course:
- FRANÇAIS I: 23 students → `pdfs_FRANCAIS_I-01/`
- FRANÇAIS II: 19 students → `pdfs_FRANCAIS_II-01/`
- EXPRESSION ORALE: 10 students → `pdfs_EXPRESSION_ORALE_03/`

### Option 2: Process Individual Sheets

```bash
# FRANÇAIS I
python example_francais_i.py

# FRANÇAIS II
python example_francais_ii.py

# EXPRESSION ORALE
python example_expression_orale.py
```

### Option 3: Interactive Selection

```bash
python example_interactive.py
```

Menu-driven interface to select which sheet(s) to process.

### Option 4: Using in Your Code

```python
import logging
from production import GradebookProcessor

logger = logging.getLogger(__name__)
source_file = 'NOTAS FRANCÉS 2019-1.xls'

# Process specific sheet
processor = GradebookProcessor(source_file, sheet_name='FRANCAIS I-01', logger=logger)
processor.run_pipeline(output_dir='reports')

# Or process all sheets dynamically
import pandas as pd
xls = pd.ExcelFile(source_file)
for sheet in xls.sheet_names:
    processor = GradebookProcessor(source_file, sheet_name=sheet, logger=logger)
    processor.run_pipeline(output_dir=f'reports_{sheet}')
```

## Multi-Sheet Processing

The Excel file `NOTAS FRANCÉS 2019-1.xls` contains 3 sheets:

| Sheet | Subject | Group | Students | Grades | Exams |
|-------|---------|-------|----------|--------|-------|
| FRANCAIS I-01 | FRANÇAIS I | 01 | 23 | 8 | 1 |
| FRANCAIS II-01 | FRANÇAIS II | 01 | 19 | 12 | 0 |
| EXPRESSION ORALE 03 | EXPRESSION ORALE | 03 | 10 | 10 | 0 |

Each sheet has independent grade structures and student rosters.

## Features

### Dynamic Sheet Selection
```python
processor = GradebookProcessor('file.xls', sheet_name='FRANCAIS I-01')
processor.run_pipeline()
```

### Separate Output Directories
Each sheet generates PDFs in its own directory:
- `pdfs_FRANCAIS_I-01/` → 23 PDFs
- `pdfs_FRANCAIS_II-01/` → 19 PDFs
- `pdfs_EXPRESSION_ORALE_03/` → 10 PDFs

### Logging Support
```python
import logging
logger = logging.getLogger(__name__)
processor = GradebookProcessor('file.xls', sheet_name='FRANCAIS I-01', logger=logger)
processor.run_pipeline()
```

## Logging Methods

All main classes support optional logging:

```python
# FileConverter
converter = FileConverter(logger=logger)
converter.log_conversion(audit_report)

# GradebookParser
parser = GradebookParser(logger=logger)
parser.log_metadata()
parser.log_students()

# PDFGenerator
generator = PDFGenerator(metadata, students, logger=logger)
generator.log_pdf_generation(count)

# GradebookProcessor
processor = GradebookProcessor(source_file, sheet_name=sheet, logger=logger)
```

## Data Access at Any Step

```python
processor = GradebookProcessor('file.xls', sheet_name='FRANCAIS I-01')

# Step 1: Convert file
processor.convert()
# Access: processor.audit_report, processor.csv_file

# Step 2: Parse gradebook
processor.parse()
# Access: processor.file_metadata, processor.students, processor.student_table

# Step 3: Generate PDFs
processor.generate_pdfs()
# Access: processor.pdf_files
```

## Comparison of Example Files

| Example | Use Case | Processing | Output | Students |
|---------|----------|-----------|--------|----------|
| `examples_multi_sheet.py` | Batch all courses | All 3 sheets at once | 3 directories | 52 total |
| `example_francais_i.py` | Single course FRANÇAIS I | FRANCAIS I-01 sheet | `pdfs_francais_i/` | 23 |
| `example_francais_ii.py` | Single course FRANÇAIS II | FRANCAIS II-01 sheet | `pdfs_francais_ii/` | 19 |
| `example_expression_orale.py` | Single course EXPRESSION ORALE | EXPRESSION ORALE 03 sheet | `pdfs_expression_orale/` | 10 |
| `example_interactive.py` | User-driven selection | Menu-based choice (1-3 or 0) | Dynamic | Variable |

## Architecture

**Pipeline Flow**:

```
Excel File (xls/xlsx)
    ↓
FileConverter.convert()
    ├─ Reads file [or specific sheet]
    └─ Outputs CSV
    ↓
GradebookParser.parse()
    ├─ Reads CSV
    ├─ Extracts FileMetadata
    └─ Extracts List[StudentGrades]
    ↓
PDFGenerator.generate_pdfs()
    ├─ Processes each StudentGrades
    ├─ Creates styled PDF with tables
    └─ Outputs individual PDF files
```

**Class Responsibilities**:

- **FileMetadata**: Data container for institution/course/semester info (11 fields, dynamic for grades)
- **StudentGrades**: Data container for individual student scores
- **FileConverter**: File I/O and format conversion (xls/xlsx → csv)
- **GradebookParser**: CSV parsing and data extraction
- **PDFGenerator**: PDF creation with ReportLab, French formatting, color coding
- **GradebookProcessor**: Main orchestrator, manages pipeline flow, sheet selection

**Key Enhancement - Multi-Sheet Support**:

```python
# Original: Single file processing
processor = GradebookProcessor(source_file)

# Enhanced: Sheet-level processing
processor = GradebookProcessor(source_file, sheet_name='FRANCAIS I-01')

### Modularity
- Each class has a single responsibility
- Independent and composable
- Easy to test and extend

### Open Design
- Access data at any step
- Combine classes as needed
- Optional logging doesn't interfere with main flow

### Production Ready
- Comprehensive error handling
- Detailed audit reports
- Extensible logging system
- Complete documentation

## Class Hierarchy

```
FileConverter (static methods)
├── convert_and_audit()
└── log_conversion()

GradebookParser
├── parse()
├── log_metadata()
└── log_students()

PDFGenerator
├── generate()
└── log_pdf_generation()

GradebookProcessor
├── convert()
├── parse()
├── generate_pdfs()
├── run_pipeline()
└── summary()
```

## Data Models

### FileMetadata
Institutional and subject information extracted from gradebook headers:
- University name, program, subject
- Group, professor, period
- First-block grade count and numbers
- Component exam numbers
- Grade labels dictionary

### StudentGrades
Individual student grade information:
- Student number, code, name
- Absences count
- First-block grades (dict)
- Component grades (dict)
- Weighted components (dict)
- Definitive grade (final)

## Example Output

```
======================================================================
PROCESSING SUMMARY
======================================================================

Institution: UNIVERSITÉ DE CALDAS
Subject: FRANÇAIS I
Group: 01
Period: 2019
Professor: JOSÉ FERNANDO RAMÍREZ OSORIO
First-block grades: 8
Component exams: 4

Students processed: 23
PDFs generated: 23

First 3 PDFs:
  - student_pdfs/01_Student_Name.pdf
  - student_pdfs/02_Student_Name.pdf
  - student_pdfs/03_Student_Name.pdf
======================================================================
```

## Requirements

- Python 3.7+
- pandas
- reportlab
- openpyxl (for Excel support)

## License

Educational Mailer System
