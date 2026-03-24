"""
Sheet 1 Example: FRANCAIS I

Process the FRANÇAIS I course sheet independently.
This sheet contains 23 students with 8 first-block grades and 1 component exam.
"""

import logging
from datetime import datetime
from production import GradebookProcessor


def setup_logger() -> logging.Logger:
    logger = logging.getLogger("FRANCAIS_I")
    logger.setLevel(logging.INFO)
    logger.handlers.clear()
    
    handler = logging.StreamHandler()
    handler.setLevel(logging.INFO)
    formatter = logging.Formatter(
        '%(asctime)s | %(levelname)-8s | %(message)s',
        datefmt='%H:%M:%S'
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)
    return logger


# ==================== FRANCAIS I PROCESSING ====================

print("\n" + "=" * 80)
print("FRANÇAIS I - COURSE GRADEBOOK PROCESSING")
print("=" * 80 + "\n")

logger = setup_logger()
source_file = 'NOTAS FRANCÉS 2019-1.xls'
sheet_name = 'FRANCAIS I-01'

# Create processor for FRANÇAIS I
processor = GradebookProcessor(source_file, sheet_name=sheet_name, logger=logger)

# Run pipeline with timestamp and professor email
timestamp = datetime.now().isoformat()
email = "j.ramirez@universidaddecaldas.edu.co"
if processor.run_pipeline(output_dir='pdfs_francais_i', 
                          timestamp_iso=timestamp,
                          email_professor=email):
    processor.summary()
    
    # Display detailed information
    print("\n" + "=" * 80)
    print("FRANÇAIS I - DETAILED ANALYSIS")
    print("=" * 80)
    
    metadata = processor.file_metadata
    
    print(f"\n👤 Course Information:")
    print(f"   Subject: {metadata.materia_name}")
    print(f"   Group: {metadata.group_name}")
    print(f"   Professor: {metadata.professor_name}")
    print(f"   Period: {metadata.period_name}")
    
    print(f"\n📊 Grade Structure:")
    print(f"   First-block grades: {metadata.first_twenty_grade_count} ({', '.join(metadata.first_twenty_grade_numbers)})")
    print(f"   Component exams: {', '.join(metadata.component_grade_numbers)}")
    
    print(f"\n📝 Grade Details:")
    for num, label in sorted(metadata.grade_labels.items(), key=lambda x: int(x[0])):
        print(f"   {num}: {label}")
    
    print(f"\n👥 Students ({len(processor.students)} total):")
    for i, student in enumerate(processor.students[:5], 1):
        print(f"   {i}. {student.student_name} ({student.student_code})")
        print(f"      Absences: {student.abse}")
        print(f"      Final Grade: {student.definitive_grade}")
    
    if len(processor.students) > 5:
        print(f"   ... and {len(processor.students) - 5} more students")
    
    print(f"\n✓ Generated {len(processor.pdf_files)} PDFs in pdfs_francais_i/")
    
else:
    print("✗ Processing failed")

print("\n" + "=" * 80 + "\n")
