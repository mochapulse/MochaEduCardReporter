"""
Multi-Sheet Gradebook Processing Example

Demonstrates how to process multiple sheets from a single Excel file.
The NOTAS FRANCÉS 2019-1.xls file contains 3 different course sheets:
  1. FRANCAIS I-01
  2. FRANCAIS II-01
  3. EXPRESSION ORALE 03

Each sheet is processed independently to generate separate PDF reports.
"""

import logging
from datetime import datetime
from pathlib import Path
from production import GradebookProcessor


# ==================== SETUP LOGGING ====================

def setup_logger(name: str, level: int = logging.INFO) -> logging.Logger:
    """Create a configured logger instance."""
    logger = logging.getLogger(name)
    logger.setLevel(level)
    logger.handlers.clear()
    
    handler = logging.StreamHandler()
    handler.setLevel(level)
    formatter = logging.Formatter(
        '%(asctime)s | %(levelname)-8s | %(message)s',
        datefmt='%H:%M:%S'
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)
    return logger


# ==================== CONFIGURATION ====================

logger = setup_logger("GradebookProcessor", level=logging.INFO)

source_file = 'NOTAS FRANCÉS 2019-1.xls'

# Get available sheets dynamically from the Excel file
sheets = GradebookProcessor.get_sheet_names(source_file)
# Filter out empty sheets
sheets = [s for s in sheets if s.strip()]


# ==================== PROCESS ALL SHEETS ====================

print("\n" + "=" * 80)
print("MULTI-SHEET GRADEBOOK PROCESSING")
print("=" * 80)
print(f"\nSource file: {source_file}")
print(f"Sheets to process: {len(sheets)}\n")

all_results = {}

timestamp = datetime.now().isoformat()
email = "j.ramirez@universidaddecaldas.edu.co"

for i, sheet_name in enumerate(sheets, 1):
    print(f"\n{'-' * 80}")
    print(f"SHEET {i}/{len(sheets)}: {sheet_name}")
    print(f"{'-' * 80}")
    
    # Create output directory for this sheet
    output_dir = f"pdfs_{sheet_name.replace(' ', '_').replace('/', '-')}"
    
    # Create processor with sheet_name parameter
    processor = GradebookProcessor(source_file, sheet_name=sheet_name, logger=logger)
    
    # Run pipeline with timestamp and professor email
    success = processor.run_pipeline(output_dir=output_dir,
                                     timestamp_iso=timestamp,
                                     email_professor=email)
    
    if success:
        # Store results
        all_results[sheet_name] = {
            'metadata': processor.file_metadata,
            'student_count': len(processor.students),
            'pdf_count': len(processor.pdf_files),
            'output_dir': output_dir,
        }
        
        processor.summary()
        print(f"\n✓ Sheet '{sheet_name}' processed successfully!")
    else:
        print(f"\n✗ Sheet '{sheet_name}' processing failed!")


# ==================== SUMMARY OF ALL SHEETS ====================

print("\n\n" + "=" * 80)
print("COMPLETE SUMMARY")
print("=" * 80)

total_students = sum(r['student_count'] for r in all_results.values())
total_pdfs = sum(r['pdf_count'] for r in all_results.values())

print(f"\n📊 Overall Statistics:")
print(f"   Total sheets processed: {len(all_results)}")
print(f"   Total students: {total_students}")
print(f"   Total PDFs generated: {total_pdfs}")

print(f"\n📚 Details by Sheet:")
for sheet_name, result in all_results.items():
    print(f"\n   {sheet_name}:")
    print(f"     Institution: {result['metadata'].university_name}")
    print(f"     Subject: {result['metadata'].materia_name}")
    print(f"     Group: {result['metadata'].group_name}")
    print(f"     Professor: {result['metadata'].professor_name}")
    print(f"     Students: {result['student_count']}")
    print(f"     PDFs: {result['pdf_count']}")
    print(f"     Output: {result['output_dir']}/")

print("\n" + "=" * 80)
print("✓ All sheets processed successfully!")
print("=" * 80 + "\n")


# ==================== INDIVIDUAL SHEET ACCESS ====================

print("\nAccessing data from first sheet:\n")

first_sheet = sheets[0]
if first_sheet in all_results:
    result = all_results[first_sheet]
    metadata = result['metadata']
    
    print(f"Sheet: {first_sheet}")
    print(f"  University: {metadata.university_name}")
    print(f"  Program: {metadata.program_name}")
    print(f"  Subject: {metadata.materia_name}")
    print(f"  First-block grades: {metadata.first_twenty_grade_count}")
    print(f"  Component exams: {len(metadata.component_grade_numbers)}")
    print(f"  Grade labels: {len(metadata.grade_labels)}")
