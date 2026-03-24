"""
Gradebook Processing Example

Demonstrates how to use GradebookProcessor to convert, parse, and generate
PDF reports from a gradebook file with optional logging.

Two examples:
1. Basic usage without logging (clean and simple)
2. Advanced usage with logging for detailed execution tracking
"""

import logging
from production import GradebookProcessor, FileMetadata, StudentGrades


# ==================== SETUP LOGGING ====================

def setup_logger(name: str, level: int = logging.INFO) -> logging.Logger:
    """Create a configured logger instance.
    
    Args:
        name: Logger name
        level: Logging level (logging.DEBUG, logging.INFO, etc.)
        
    Returns:
        Configured logging.Logger instance
    """
    logger = logging.getLogger(name)
    logger.setLevel(level)
    
    # Console handler with formatted output
    handler = logging.StreamHandler()
    handler.setLevel(level)
    
    formatter = logging.Formatter(
        '%(asctime)s | %(name)s | %(levelname)-8s | %(message)s',
        datefmt='%H:%M:%S'
    )
    handler.setFormatter(formatter)
    
    # Remove existing handlers to avoid duplicates
    logger.handlers.clear()
    logger.addHandler(handler)
    
    return logger


# ==================== EXAMPLE 1: BASIC USAGE (NO LOGGING) ====================

print("\n" + "=" * 70)
print("EXAMPLE 1: BASIC USAGE (No Logging)")
print("=" * 70)

source_file = 'NOTAS FRANCÉS 2019-1.xls'
processor = GradebookProcessor(source_file)

# Run complete pipeline
success = processor.run_pipeline(output_dir='student_pdfs')

if success:
    processor.summary()
    print("\n✓ Processing completed successfully!")
else:
    print("\n✗ Processing failed. Check errors above.")


# ==================== EXAMPLE 2: ADVANCED USAGE WITH LOGGING ====================

print("\n\n" + "=" * 70)
print("EXAMPLE 2: ADVANCED USAGE WITH LOGGING")
print("=" * 70 + "\n")

# Setup logger
logger = setup_logger("GradebookProcessor", level=logging.INFO)

# Create processor with logger
processor_with_logging = GradebookProcessor(source_file, logger=logger)

# Run complete pipeline (with detailed logging)
success = processor_with_logging.run_pipeline(output_dir='student_pdfs')

if success:
    processor_with_logging.summary()
    print("\n✓ Processing completed successfully!")
else:
    print("\n✗ Processing failed. Check errors above.")


# ==================== EXAMPLE 3: STEP-BY-STEP CONTROL WITH LOGGING ====================

print("\n\n" + "=" * 70)
print("EXAMPLE 3: STEP-BY-STEP CONTROL WITH LOGGING")
print("=" * 70 + "\n")

# Create processor with logging
processor_steps = GradebookProcessor(source_file, logger=logger)

# Step 1: Convert file
print("Step 1: Converting file...")
if processor_steps.convert():
    print(f"✓ Converted to: {processor_steps.csv_file}\n")
    
    # Step 2: Parse gradebook
    print("Step 2: Parsing gradebook...")
    if processor_steps.parse():
        print(f"✓ Parsed {len(processor_steps.students)} students\n")
        
        # Step 3: Generate PDFs
        print("Step 3: Generating PDFs...")
        if processor_steps.generate_pdfs(output_dir='student_pdfs'):
            print(f"✓ Generated {len(processor_steps.pdf_files)} PDFs\n")
        else:
            print("✗ PDF generation failed\n")
    else:
        print("✗ Parsing failed\n")
else:
    print("✗ Conversion failed\n")


# ==================== EXAMPLE 4: ACCESSING INTERMEDIATE DATA ====================

print("\n" + "=" * 70)
print("EXAMPLE 4: ACCESSING INTERMEDIATE DATA")
print("=" * 70)

# Use the processor from Example 2 which has all data populated
if success:
    print("\nFileMetadata:")
    metadata = processor_with_logging.file_metadata
    print(f"  University: {metadata.university_name}")
    print(f"  Program: {metadata.program_name}")
    print(f"  Subject: {metadata.materia_name}")
    print(f"  Group: {metadata.group_name}")
    print(f"  Professor: {metadata.professor_name}")
    print(f"  Period: {metadata.period_name}")
    print(f"  First-block grades: {metadata.first_twenty_grade_count}")
    print(f"  Component exams: {len(metadata.component_grade_numbers)}")
    
    print("\nGrade Labels:")
    for num, label in sorted(metadata.grade_labels.items(), key=lambda x: int(x[0])):
        print(f"  {num}: {label}")
    
    print(f"\nStudents ({len(processor_with_logging.students)} total):")
    for i, student in enumerate(processor_with_logging.students[:3]):
        print(f"\n  Student {i+1}: {student.student_name}")
        print(f"    Code: {student.student_code}")
        print(f"    Absences: {student.abse}")
        print(f"    Final grade: {student.definitive_grade}")
    
    print("\nStudent Summary Table:")
    print(processor_with_logging.student_table.head())
    
    print(f"\nGenerated PDFs ({len(processor_with_logging.pdf_files)} total):")
    for pdf in processor_with_logging.pdf_files[:5]:
        from pathlib import Path
        print(f"  {Path(pdf).name}")


# ==================== END ====================

print("\n" + "=" * 70)
print("All examples completed!")
print("=" * 70 + "\n")
