"""
Interactive Sheet Selector

Allows selecting which sheet from the Excel file to process.
Displays available sheets and processes the selected one.
"""

import logging
import sys
from datetime import datetime
from production import GradebookProcessor


def setup_logger() -> logging.Logger:
    logger = logging.getLogger("GradebookProcessor")
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


def display_menu(sheets: list) -> int:
    """Display menu and get user selection."""
    print("\n" + "=" * 80)
    print("SELECT A SHEET TO PROCESS")
    print("=" * 80 + "\n")
    
    for i, sheet in enumerate(sheets, 1):
        print(f"  {i}. {sheet}")
    
    print(f"\n  0. Process all sheets")
    print(f"  Q. Quit\n")
    
    while True:
        choice = input("Enter your choice (1-3, 0 for all, or Q): ").strip().upper()
        
        if choice == 'Q':
            return -1
        
        if choice == '0':
            return 0
        
        try:
            choice_num = int(choice)
            if 1 <= choice_num <= len(sheets):
                return choice_num
        except ValueError:
            pass
        
        print("Invalid choice. Please try again.")


def process_sheet(source_file: str, sheet_name: str, output_dir: str, 
                  logger: logging.Logger, timestamp_iso: str, email_professor: str) -> bool:
    """Process a single sheet and return success status."""
    processor = GradebookProcessor(source_file, sheet_name=sheet_name, logger=logger)
    
    if processor.run_pipeline(output_dir=output_dir,
                             timestamp_iso=timestamp_iso,
                             email_professor=email_professor):
        processor.summary()
        return True
    return False


# ==================== MAIN ====================

logger = setup_logger()
source_file = 'NOTAS FRANCÉS 2019-1.xls'

# Get available sheets dynamically
sheets = GradebookProcessor.get_sheet_names(source_file)
sheets = [s for s in sheets if s.strip()]  # Filter empty sheets

output_dirs = {
    'FRANCAIS I-01': 'pdfs_francais_i',
    'FRANCAIS II-01': 'pdfs_francais_ii',
    'EXPRESSION ORALE 03': 'pdfs_expression_orale',
}

timestamp = datetime.now().isoformat()
email = "j.ramirez@universidaddecaldas.edu.co"

print("\n" + "=" * 80)
print("GRADEBOOK SHEET SELECTOR")
print("=" * 80)
print(f"\nSource file: {source_file}")
print(f"Available sheets: {len(sheets)}")

choice = display_menu(sheets)

if choice == -1:
    print("\nExiting...")
    sys.exit(0)

sheets_to_process = sheets if choice == 0 else [sheets[choice - 1]]

print("\n" + "=" * 80)
print(f"PROCESSING {len(sheets_to_process)} SHEET(S)")
print("=" * 80)

results = {}
for i, sheet in enumerate(sheets_to_process, 1):
    print(f"\n[{i}/{len(sheets_to_process)}] Processing: {sheet}")
    print("-" * 80 + "\n")
    
    output_dir = output_dirs.get(sheet, f"pdfs_{sheet.replace(' ', '_').replace('/', '-')}")
    success = process_sheet(source_file, sheet, output_dir, logger, timestamp, email)
    
    results[sheet] = success
    
    if success:
        print(f"\n✓ '{sheet}' processed successfully!")
    else:
        print(f"\n✗ '{sheet}' processing failed!")


# ==================== FINAL SUMMARY ====================

print("\n\n" + "=" * 80)
print("PROCESSING SUMMARY")
print("=" * 80)

successful = sum(1 for success in results.values() if success)
failed = len(results) - successful

print(f"\n  Total sheets: {len(results)}")
print(f"  Successful: {successful}")
print(f"  Failed: {failed}")

for sheet, success in results.items():
    status = "✓" if success else "✗"
    print(f"  {status} {sheet}")

print("\n" + "=" * 80 + "\n")

if failed > 0:
    sys.exit(1)
