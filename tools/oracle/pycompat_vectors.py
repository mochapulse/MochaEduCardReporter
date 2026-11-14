"""Generate reference vectors for the helper functions of the legacy engine.

Output: fixtures/expected/pycompat.json, shape {"<name>": [{"in": str, "out": str|null}, ...]}.
Inputs: every distinct non-empty cell string from the grid.json files under the
expected folder, plus hand-picked edge cases. Run dump.py first.

Usage: legacy/.venv/bin/python tools/oracle/pycompat_vectors.py [--expected fixtures/expected]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "legacy" / "src"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import libs.excel_analyzer as ea  # noqa: E402
from libs.excel_analyzer import GradebookParser as GP  # noqa: E402
from dump import write_json  # noqa: E402

WS = ["\xa0", "\t", "\n", "\r\n", "  ", "\u2003", "\u3000", "\x1c", "\x1d", "\x1e", "\x1f",
      "\x85", "\u2028", "\u2029", "\u200b", "\ufeff", "\u1680", "\u202f", "\v", "\f", "\x0b\x0c"]

NUMBERS = ["3", "3.0", "3.25", "3.35", "2.5", "0.05", "4.45", "1e3", "1E-2", "inf", "-inf", "Infinity",
           "-Infinity", "+inf", "nan", "NaN", "-nan", "1_0", "_1", "1_", "1__0", "1_000.5", " 4 ", "4,5",
           "+3", "-0", "-0.0", "-0.04", ".5", "5.", "-.5", "+.5e1", "0x10", "٣", "１２", "１２.５", "",
           " ", "0", "00", "007", "10", "100", "20", "1.5", "0.1", "0.2", "0.3", "0.15", "0.333", "0.25",
           "0.35", "0.45", "4.95", "9.95", "9.99", "19.95", "1e16", "1e-5", "1e22", "1e400", "123456789.0",
           "0.30000000000000004", "33.300000000000004", "15.000000000000002", "20.0", "100.0", "0.5",
           "5e-324", "1.7976931348623157e308", "12abc", "abc", "1 2", "1,000", "1.2.3", "--1", "+-1",
           "e5", "1e", "1e+", ".", "+", "-", "٣.٥", "1\u00a0", "\u00a01", "3\n", "\t3.5\t", "4.0 ", "NAN",
           "iNf", "infinity", "1e5", "1E5", "9" * 30, "0." + "0" * 20 + "1", "12345678901234567890"]

PERCENTS = ["20%", "0.2", "0,2", "1", "1.5", "20", "0.15", "0.333", "100", "abc", "20 %", "", "%", "0%",
            "0", "1%", "15%", "12,5%", "12.5%", "33.3%", "0.5", "0,5", "50", "50%", " 30 % ", "2", "1.0",
            "0.10", "0.25", "0.35", "0.45", "100%", "150%", "1e1", "inf", "nan", "-5", "-0.2", "10 %",
            "20%%", "%20", "a%", "0,333", "0.3333333333333333", "1,5", "1.000", "99.99", "1e-3",
            "\xa020\xa0%"]

LABELS = [
    "Examen. Le 3 mars", "Quiz 1. Vocabulaire. Le 3 mars", "Quiz: Vocabulaire: Le 3 mars",
    "Quiz. Vocab: Le 3 mars", "Quiz: Vocab. Le 3 mars", "No separator here", "", ".", ". ", ": ", ":", ".:",
    "A. ", "A: ", "A. B", "A: B", "A.B", "A.Éducation", "A.É", "a.bC", "A.Àb. Cd", "A.Öx", "A.×x",
    "A.Ø", "A.Ùx", "A.ÿ", "Le 3 mars", "Le lundi", "Cat. Le lundi", "Cat. Le 3", "Cat. Desc Le 3 mars",
    "Cat. Desc Le 3", "Cat. Le 3 mars Le 4 avril", "Cat. Le", "Cat. Le 5", "Cat: Le 5 mai.",
    "Cat. desc.", "Cat. desc:", "Cat. : x", "Cat.: x", "Cat : x", "Cat .x", "Cat. .", "  Cat  .  desc  ",
    "Cat.  desc", "Cat.\tdesc", "Cat.\u00a0desc", "x: y. z", "x. y: z", "x.y: z", "Examen final.Écrit",
    "EXAMEN 1. Compréhension écrite. Le 12 mai", "PROJET: Présentation orale: Le 2 juin", "Oral. Le 3",
    "Oral. xLe 3", "Oral. Le  3", "Oral. Le\n3", "Oral. Le ٣", "Oral. Le １", "Trailing. ", "Trailing: ",
    "Trailing.  ", ". Leading", ": Leading", "Quiz 1: Le 3", "Quiz 1: desc Le 3", "Quiz.\u00c0 x",
    "Quiz.\u00dex", "Quiz.\u00d7x", "A.B.C. D", "A. B. C", "Séance 1. Le 5 mars", "Dossier. Le 5, Le 6 mars",
    "NOTE 5", "Note: ", "Note. ", "Note.Z", "z.Z.Z"]

KEYS = ["12 - Label", "12-Label", "x - y", "1-5 average", "1 - ", "1 -", "1- x", " 1 - x", "1  - x",
        "1\t- x", "12 - Quiz. Vocab. Le 3 mars", "12 - A - B", "12 - ", "123 - Examen: Final", "٣ - x",
        "１ - x", "0 - x", "007 - x", "1-5 weighted 20%", "1 weighted 20%", "12 -- x", "12 – x", "ab - cd",
        "- x", "-", "", "12", "12 -x - y", "5 - Quiz.Écrit", "5 - Quiz. Écrit. Le 3 mars", "5 -  x ",
        "5 - x\u00a0", "99999999999999999999 - x", "1.5 - x", "1 -\n-x", "3 - Le 3 mars"]

FILENAMES = ["Jean-Pierre Dupont", "José Ñandú", "MARTÍNEZ, Ana", "O'Brien", "Anne-Marie  Lefèvre",
             "  padded  ", "a\tb", "a\nb", "a\u00a0b", "!!!", "@#$%^&*()", "---", "___", "-_-", "", " ",
             "...", "a/b\\c", "a:b*c?d\"e<f>g|h", "ñandú", "Müller", "Çelik", "名前", "Привет", "αβγ",
             "٣٤", "１２３", "a_b", "a-b", "a - b", "a.b", "e\u0301", "x\u200by", "x\ufeffy", "x\u3000y",
             "(Nom) [Prénom]", "A.B. C", "Dupont, Jean", "L'élève", "\u2014dash\u2013", "tab\tonly\t",
             "😀", "a😀b", "1234567890", "Z" * 100, "ÀÉÎÕÜ", "àéîõü", "ß", "ǅ", "ª", "²", "½", "①",
             "a\x1cb", "a\x85b", "_x_", "-x-"]

ABSENCES = ["Absences : 3", "ABSENCE: 2,5", "absences 4", "Absence: 3", "absences:3", "Absences :3.5",
            "Absences : 3.", "Absences : .5", "Absences : 3,", "Absences :  3", "Absences\t: 3",
            "Absences\n: 3", "absences : 12abc", "Les absences: 2 et absences: 3", "xAbsences: 7",
            "Absencess: 3", "Absen: 3", "Absences : -3", "Absences : ٣", "Absences : １２", "ABSENCES : 10",
            "Absences : 1.25", "Absences : 1,25", "Absences : 0", "Absences : 00", "Absences : 007",
            "Absences : 3 (justifiées)", "Total absences: 4", "", "absences", "absences:", "absences: x",
            "Absences\u00a0: 3", "Absences : 3\u00a0", "Absences : 1e3", "Absences : 99999999999999999999",
            "Absences : 2,5,6", "Absences : 2.5.6", "AbSeNcEs:\t8"]

GROUPS = ["Groupe 1", "GROUPE : 02", "Grupo 12 ", "1", "", "abc", "Groupe A", "G12-34", "Groupe ٣",
          "Groupe １２", "  7  ", "Groupe 007", "2019-1", "Groupe\u00a05", "x9y8"]

PROFS = ["PROFESSEUR : Dupont", "professeur: Jean", "PROFESSEUR:Dupont", "PROFESSEUR  :   Dupont  ",
         "PROFESSEUR Dupont", "Prof. Dupont", "", "Dupont PROFESSEUR : x", "PROFESSEUR :", "PROFESSEUR : ",
         "PROFESSEUR\t:\tX", "PROFESSEUR\u00a0:\u00a0X", "Professeur : José Ñ", "PROFESSEUR : A : B",
         "ProfesseurX : Y", "PROFESSEUR ::X"]

PERIODS = ["2019-1", "Période 2025-2", "2025", "abc", "", "  12 ", "123", "1234", "12345", "Semestre 1",
           "2019 - 2020", "٢٠١٩", "２０１９", "année 2024/2025", "x2025y2026", "20 25", "2025\u00a0", "I-2019"]

CSVNAMES = ["NOTAS FRANCÉS 2019-1_FRANCAIS I-01.csv", "NOTAS FRANCÉS 2025-2_FRANCAIS IV LE MATIN.csv",
            "NOTAS FRANCÉS 2019-1_Hoja1.csv", "a_francais b.csv", "a_FRANCAIS_I_01.csv", "a_FRANC.csv",
            "a_FRAN.csv", "a_FRANx.csv", "FRANCAIS.csv", "_FRANCAIS I.csv", "x_FRANCAIS I_y_FRANCAIS II.csv",
            "dir/sub/NOTAS_FRANCAIS IV.csv", "/abs/path/x_FRANÇAIS IV APRÈS-MIDI.csv", "noext_FRANCAIS I",
            "x_FRANCAIS I.tar.gz", "x_FRANCAIS.I.csv", "x_francais i.csv", "x_Français IV.csv",
            "x_FRAN\u00c7AIS.csv", "x__FRANCAIS.csv", "x_FRANCAIS\ni.csv", ".csv", "", "x_FRANCAIS I.CSV",
            "C:\\dir\\x_FRANCAIS I.csv", "x_FRANCAIS I .csv", "x_FRANCAIS__I.csv", "a.b_FRANCAIS I.csv",
            ".hidden_FRANCAIS", "x_FRANCAIS I/", "NOTAS 2019_EXPRESSION ORALE 03.csv"]

COMPONENT = ["Examen", "EXAMEN 1", "examen final", "Éxamen", "Exámen", "Exam", "xexamenx", "", "Quiz",
             " ExAmEn ", "EXAMEN\u00a0FINAL", "Examens"]
FINALROW = ["Note du cours", "NOTE DU COURS : 4.5", "note du cours", "  Note du cours", "Note  du cours",
            "Note du\u00a0cours", "Note du cours.", "xNote du cours", "Note", "Note du", "", "NOTE DÉFINITIVE",
            "Notes du cours", "note\tdu cours", "NOTE DU COURS"]

FMT_FLOATS = ["0.05", "0.15", "0.25", "0.35", "0.45", "0.55", "0.65", "0.75", "0.85", "0.95", "1.05", "1.15",
              "1.25", "1.35", "1.45", "2.5", "3.25", "3.35", "4.45", "4.95", "0.0", "-0.0", "-0.04", "-0.05",
              "-0.06", "-2.5", "-3.25", "1e16", "1e-5", "1e-4", "1e-3", "1e5", "123456789.0", "1e15", "1e17",
              "1e21", "1e22", "1e100", "0.30000000000000004", "20.0", "33.300000000000004", "100.0", "0.5",
              "1.5", "15.000000000000002", "0.1", "0.2", "0.3", "1.0", "2.0", "10.0", "12.5", "7.5", "9.95",
              "9.05", "19.95", "99.95", "999999.5", "1234567.0", "1234567.5", "0.000123456", "123456.7",
              "1234567.891", "0.1234567", "inf", "-inf", "nan", "5e-324", "1.7976931348623157e308",
              "3.14159265358979", "2.675", "1.005", "0.045", "0.055", "8.5", "9.5", "10.5", "4.0", "4",
              "20", "3.999", "3.95", "3.949999", "0.049999999", "0.95", "0.96", "99.99", "999999",
              "1e6", "1e-7", "0.0001", "0.00001", "123456", "1234567", "12345678", "0.000001234"]
FMT_FLOATS += [repr(0.1 + 0.2), repr(0.1 * 3), repr(1.1 * 1.1), repr(4.35 * 100), repr(1 / 3), repr(2 / 3)]
FMT_FLOATS += [repr(k / 100) for k in range(0, 1001, 5)]
FMT_FLOATS += [repr(k / 10) for k in range(0, 201)]


def ident(fn: Callable[[str], Any]) -> Callable[[str], Optional[str]]:
    def run(s: str) -> Optional[str]:
        r = fn(s)
        return None if r is None else str(r)
    return run


def py_float(s: str) -> Optional[str]:
    try:
        return repr(float(s))
    except ValueError:
        return None


def to_float(s: str) -> Optional[str]:
    r = ea.to_float(s)
    return None if r is None else repr(r)


def boolstr(fn: Callable[[str], bool]) -> Callable[[str], str]:
    return lambda s: "true" if fn(s) else "false"


def dedupe(items: List[str]) -> List[str]:
    return list(dict.fromkeys(items))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--expected", default="fixtures/expected")
    args = ap.parse_args()
    expected = Path(args.expected)

    cells: List[str] = []
    for grid in sorted(expected.glob("*/grid.json")):
        for row in json.loads(grid.read_text(encoding="utf-8")):
            cells.extend(c for c in row if c != "")
    cells = dedupe(cells)

    ws_cases = []
    for w in WS:
        ws_cases += [w, f"a{w}b", f"{w}a{w}", f"a{w}{w}b", f"{w}", f"4{w}"]
    ws_cases += ["a  b", "  a  ", "a\t\tb", "a\n\nb", " \t\n ", "x\u00a0\u00a0y", "\u00a0", "\xa0Note\xa0du\xa0cours"]
    generic = dedupe(NUMBERS + ws_cases + LABELS + KEYS + FILENAMES + ABSENCES + GROUPS + PROFS + PERIODS
                     + COMPONENT + FINALROW + PERCENTS)

    def inputs(extra: List[str]) -> List[str]:
        return dedupe(extra + generic + cells)

    float_like = dedupe(FMT_FLOATS + [c for c in dedupe(NUMBERS + cells) if py_float(c) is not None])
    float_like = [c for c in float_like if py_float(c) is not None]

    spec: Dict[str, Any] = {
        "clean_text": (ea.clean_text, inputs([])),
        "to_grade_str": (ea.to_grade_str, inputs(NUMBERS)),
        "extract_grade_num_from_key": (ea.extract_grade_num_from_key, inputs(KEYS)),
        "format_concept_label": (ea.format_concept_label, inputs(LABELS)),
        "extract_label_from_key": (ea.extract_label_from_key, inputs(KEYS)),
        "safe_filename": (ea.safe_filename, inputs(FILENAMES)),
        "format_weight_label": (lambda s: ea.format_weight_label(s, "20%"), inputs(PERCENTS)),
        "to_float": (to_float, inputs(NUMBERS)),
        "extract_group": (GP._extract_group, inputs(GROUPS)),
        "extract_professor": (GP._extract_professor, inputs(PROFS)),
        "extract_period": (GP._extract_period, inputs(PERIODS)),
        "extract_absences": (GP._extract_absences, inputs(ABSENCES)),
        "extract_sheet_label_from_csv_name": (GP._extract_sheet_label_from_csv_name, inputs(CSVNAMES)),
        "is_component_label": (boolstr(GP._is_component_label), inputs(COMPONENT)),
        "is_final_carnet_row": (boolstr(GP._is_final_carnet_row), inputs(FINALROW)),
        "py_float": (py_float, inputs(NUMBERS)),
        "fmt_1f": (lambda s: f"{float(s):.1f}", float_like),
        "fmt_g": (lambda s: f"{float(s):g}", float_like),
    }
    out: Dict[str, List[Dict[str, Optional[str]]]] = {}
    for name, (fn, ins) in spec.items():
        vec = []
        for s in ins:
            r = fn(s)
            vec.append({"in": s, "out": None if r is None else str(r)})
        out[name] = vec
    write_json(expected / "pycompat.json", out)
    print({k: len(v) for k, v in out.items()}, file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
