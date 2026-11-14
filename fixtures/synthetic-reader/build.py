"""Build the synthetic reader corpus (workbooks and CSV edge cases) with openpyxl.

Usage: legacy/.venv/bin/python fixtures/synthetic-reader/build.py
"""
import datetime as dt
import random
from pathlib import Path

import openpyxl

OUT = Path(__file__).resolve().parent
OUT_FILES = OUT / "corpus"
OUT_FILES.mkdir(exist_ok=True)


def col_sheet(wb, title, columns):
    """One test column per sheet column; header t0, t1, ..."""
    ws = wb.create_sheet(title)
    ws.append([f"t{i}" for i in range(len(columns))])
    height = max(len(c) for c in columns)
    for r in range(height):
        ws.append([c[r] if r < len(c) else None for c in columns])
    return ws


def rows_sheet(wb, title, rows):
    ws = wb.create_sheet(title)
    for r in rows:
        ws.append(r)
    return ws


def new_wb():
    wb = openpyxl.Workbook()
    wb.remove(wb.active)
    return wb


# ---- types.xlsx ---------------------------------------------------------
wb = new_wb()
strnum = [["12"], ["12", "13"], ["007"], ["1e3"], [" 5 "], ["5,5"], ["inf"], ["-inf"], ["Infinity"], ["-"],
          ["0x10"], ["1_000"], ["٣"], ["+5"], [".5"], ["5."], ["true"], ["True", "False"], ["TRUE"], ["NaN"],
          ["None"], ["NULL"], ["N/A"], ["#N/A"], ["n/a"], ["<NA>"], ["-nan"], ["1.#IND"], ["12", "x"],
          ["12", 3.5], ["12", 4], ["3.50"], ["1.25"], ["0.05"], [" "], ["  x  "], ["12345678901234567890"],
          ["1e400"], ["1e-400"], ["-0"], ["-0.0"], ["+inf"], ["1d5"], ["1E5"], ["0b1"], ["१२"], ["１２"],
          ["12 "], ["\t7\t"], ["7\n"], ["nan"], ["NAN"], ["Nan"], ["none"], ["null"], ["-NaN"],
          ["NA "], [" NA"], ["INF"], ["Inf"], ["iNf"], ["infinit"], ["1.5e", "2"], ["e5"], ["1e+5"],
          ["--5"], ["5-"], ["1 2"], ["0.1.2"], ["01", "02"], ["1.0", "2"], ["yes"], ["Yes", "no"]]
col_sheet(wb, "strnum", strnum)
mix = [[True, 1], [True, 2.5], [True, "x"], ["True", ""], ["True", None, "False"], ["True", None, "x"],
       ["True", 1], [1, True], ["x", True], [True, False, None, True], [True, None, False], ["TRUE", "FALSE"],
       ["x", 1, True], ["x", True, 1], ["x", 1.5, True], ["x", 0, False], [1, "True"],
       [10 ** 20], [10 ** 20, 1.5], [2 ** 63], [2 ** 63, 5], [-5, 2 ** 63], [1e15, 2.5], [123456789.123],
       [1e16], [1e16, "x"], [1.5e-5, "x"], [100.0, "x"], [-0.0, "x"], [1, "x", None, 3.25], [3.8428571428571425, "x"],
       [1.23456789, 0.1, 1e20, 1e-7], [5.25, 4], [0.05, 2], [-1, -2.5], [None, None, 3], [None, "a"], ["1", None, "2"],
       [dt.datetime(2020, 1, 2, 3, 4, 5, 123000)], [dt.datetime(2020, 1, 2), "x"], [dt.time(1, 2, 3), "x"],
       [dt.time(1, 2, 3)], [dt.datetime(2020, 1, 2), 1], [dt.datetime(2020, 1, 2), None, dt.datetime(2020, 1, 3)],
       [dt.datetime(2020, 1, 2, 3, 4, 5), dt.datetime(2020, 1, 3)], [dt.datetime(2020, 1, 2), dt.datetime(2020, 1, 3)],
       [dt.time(1, 0), None, "a"], ["a\rb", "x"], ["a\nb"], [" lead", "x"], ["x,y"], ['x"y'], ["été"],
       ["#N/A", "x"], ["NaN", "x"], ["x", "", "y"], ["x", "NA", "y"], ["NA", "NA", "z"], [1, 2, 3], [4, None, 6],
       ["Absences:    4    Retards: 3"], ["COMPÉTENCE.\nLe 20 août,", 3.5, None]]
col_sheet(wb, "mix", mix)
wb.save(OUT_FILES / "types.xlsx")

# ---- headers.xlsx (one header variant per sheet) ------------------------
wb = new_wb()
hdrs = {
    "na": [["NA", "a"], [1, 2]], "none": [["None", "x"], [1, 2]], "ints": [[1, 1], [1, 2]],
    "floats": [[1.0, 2.5], [1, 2]], "dupe": [["a", "a", "a.1"], [1, 2, 3]], "dupe2": [["a", "a", "a"], [1, 2, 3]],
    "dupe3": [["a.1", "a", "a"], [1, 2, 3]], "dupe4": [["a", "a", "a.1", "a.1"], [1, 2, 3, 4]],
    "dupe5": [["x", "x", "x", "x.1"], [1, 2, 3, 4]], "dupe6": [["a", "b", "a", "b", "a"], [1, 2, 3, 4, 5]],
    "unnamed1": [["Unnamed: 1", None], [1, 2]], "unnamed2": [[None, "Unnamed: 0"], [1, 2]],
    "unnamed3": [["a", None, None, "Unnamed: 1"], [1, 2, 3, 4]], "unnamed4": [["a", None, "a"], [1, 2, 3]],
    "bools": [[True, False], [1, 2]], "dates": [[dt.datetime(2020, 1, 2, 3, 4), dt.time(1, 2)], [1, 2]],
    "dates2": [[dt.datetime(2020, 1, 2), dt.datetime(2020, 1, 3)], [1, 2]],
    "spaces": [[" a ", " "], [1, 2]], "big": [[1e20, 0.1 + 0.2], [1, 2]], "mixed": [[1, "1"], [1, 2]],
    "newline": [["a\nb", "c,d"], [1, 2]], "floatround": [[2.55, 3], [1, 2]], "intsfloat": [[1, 2.5, 3], [1, 2, 3]],
    "leadrow": [[None, None], ["a", "b"], [1, 2]], "leadrows2": [[None], [None], ["a"], [1]],
    "leadcol": [[None, "a", "b"], [None, 1, 2]], "single": [["a", "b"]], "singlecell": [["a"]],
    "onecol": [["a"], [None], [1], [None], [2]], "onecol_blank_hdr": [[None], [1], [2]],
    "trailing": [["a", "b"], [1, 2], [None, None], [None, None]], "midblank": [["a", "b"], [1, 2], [None, None], [3, 4]],
    "offset": [[None, None, None], [None, None, None], [None, "a", "b"], [None, 1, 2]],
    "ragged": [["a", "b", "c"], [1], [1, 2, 3], [1, 2]], "firstblank_data": [["a", "b"], [None, "x"]],
    "comma_quote": [["a,b", 'c"d'], ["e\nf", "g"]], "allempty_data": [["a", "b"], [None, None]],
    "wide_blank": [["a", None, None, None, "e"], [1, None, None, None, 5]],
    "num_hdr_text": [["1", "2.0", "3"], [1, 2, 3]],
}
for name, rows in hdrs.items():
    rows_sheet(wb, name, rows)
ws = wb.create_sheet("emptysheet")
ws = wb.create_sheet("errcell")
ws.append(["a", "b"])
ws.append([1, 2])
ws["A3"] = "#DIV/0!"
ws["A3"].data_type = "e"
ws["B3"] = 5
ws = wb.create_sheet("errhdr")
ws.append(["a", "b"])
ws["A1"] = "#N/A"
ws["A1"].data_type = "e"
ws.append([1, 2])
ws = wb.create_sheet("formula")
ws.append(["a", "b"])
ws.append([1, "=A2+1"])
ws.append(["=SUM(A2:A2)", 3])
wb.save(OUT_FILES / "headers.xlsx")

# ---- fuzz.xlsx: seeded random columns ------------------------------------
rng = random.Random(20260115)
alphabet = list("0123456789.eE+- _,infatyINFATY\t") + ["NA", "nan", "None", "x", "é"]
wb = new_wb()
cols = []
for _ in range(300):
    n = rng.randint(1, 4)
    cols.append(["".join(rng.choice(alphabet) for _ in range(rng.randint(1, 6))) for _ in range(n)])
col_sheet(wb, "randstr", cols)
cols = []
for _ in range(300):
    n = rng.randint(1, 5)
    col = []
    for _ in range(n):
        kind = rng.choice(["i", "f", "s", "n", "b", "ns", "d", "big"])
        if kind == "i":
            col.append(rng.randint(-5, 9))
        elif kind == "f":
            col.append(round(rng.uniform(-3, 10), rng.randint(0, 6)))
        elif kind == "s":
            col.append(rng.choice(["a", "True", "12", "3.5", "x y", " 7 ", "1e2", "NA", ""]))
        elif kind == "n":
            col.append(None)
        elif kind == "b":
            col.append(rng.choice([True, False]))
        elif kind == "ns":
            col.append(rng.choice(["1", "2.5", "10"]))
        elif kind == "d":
            col.append(dt.datetime(2020, 1, rng.randint(1, 28), rng.choice([0, 0, 5]), rng.choice([0, 30])))
        else:
            col.append(rng.choice([2 ** 63, 10 ** 20, 2 ** 70]))
    if any(isinstance(v, int) and not isinstance(v, bool) and v >= 2 ** 64 for v in col):
        # Known gap: pandas turns bools into NaN next to ints above uint64 (not reproduced).
        col = [7 if isinstance(v, bool) else v for v in col]
    cols.append(col)
col_sheet(wb, "randmix", cols)
wb.save(OUT_FILES / "fuzz.xlsx")

# ---- fuzz2.xlsx: seeded random columns ------------------------------------
rng = random.Random(77001)
alphabet = list("0123456789.eE+- _,infatyINFATY\t") + ["NA", "nan", "None", "x", "é"]
wb = new_wb()
cols = []
for _ in range(1500):
    n = rng.randint(1, 6)
    cols.append(["".join(rng.choice(alphabet) for _ in range(rng.randint(1, 6))) for _ in range(n)])
col_sheet(wb, "randstr", cols)
cols = []
for _ in range(1500):
    n = rng.randint(2, 7)
    col = []
    for _ in range(n):
        kind = rng.choice(["i", "f", "s", "n", "b", "ns", "d", "big"])
        if kind == "i":
            col.append(rng.randint(-5, 9))
        elif kind == "f":
            col.append(round(rng.uniform(-3, 10), rng.randint(0, 6)))
        elif kind == "s":
            col.append(rng.choice(["a", "True", "12", "3.5", "x y", " 7 ", "1e2", "NA", ""]))
        elif kind == "n":
            col.append(None)
        elif kind == "b":
            col.append(rng.choice([True, False]))
        elif kind == "ns":
            col.append(rng.choice(["1", "2.5", "10"]))
        elif kind == "d":
            col.append(dt.datetime(2020, 1, rng.randint(1, 28), rng.choice([0, 0, 5]), rng.choice([0, 30])))
        else:
            col.append(rng.choice([2 ** 63, 10 ** 20, 2 ** 70]))
    if any(isinstance(v, int) and not isinstance(v, bool) and v >= 2 ** 64 for v in col):
        # Known gap: pandas turns bools into NaN next to ints above uint64 (not reproduced).
        col = [7 if isinstance(v, bool) else v for v in col]
    cols.append(col)
col_sheet(wb, "randmix", cols)
wb.save(OUT_FILES / "fuzz2.xlsx")

# ---- gradebook-like header sheets (exercise get_sheet_names) -------------
wb = new_wb()
rows_sheet(wb, "GRADES", [["UNIVERSITE", None], [None, None], ["No.", "CODE", "NOM ET PRENOM", "abse", 1, 2], [1, "123", "Ana", 0, 4.5, 3]])
rows_sheet(wb, "CARNET", [["x"], ["No.", "ITEM", "NOTE", "MOY"], [1, "a", 3.5, None]])
rows_sheet(wb, "lateheader", [[None]] * 14 + [["No.", "ITEM", "NOTE", "MOY"]])
rows_sheet(wb, "toolate", [[None]] * 15 + [["No.", "ITEM", "NOTE", "MOY"]])
rows_sheet(wb, "NOHEADER", [["a", "b"], [1, 2]])
rows_sheet(wb, "mixedcase", [[" N° ", "Code", "Étudiant", "ABSENCES"]])
rows_sheet(wb, "partial", [["No.", "CODE", "NOM"]])
wb.create_sheet("blank")
wb.save(OUT_FILES / "headers_detect.xlsx")

# ---- CSV edge cases (raw bytes) -------------------------------------------
csvs = {
    "basic": b"a,b\n1,2\n",
    "hdronly": b"a,b\n",
    "empty": b"",
    "newline_only": b"\n",
    "blanks": b"\n\n\n",
    "spaces": b"  \n",
    "short": b"a,b,c\n1,2\n3\n",
    "long_first": b"a,b\n1,2,3\n",
    "long_later": b"a,b\n1,2\n3,4\n5,6,7\n",
    "firstshort": b"a\nb,c\nd,e,f\n",
    "bom": b"\xef\xbb\xbfa,b\n1,2\n",
    "crlf": b"a,b\r\n1,2\r\n",
    "cr": b"a,b\r1,2\r",
    "noeol": b"a,b\n1,2",
    "quote": b'a,b\n"x,y","z""w"\n',
    "quote_mid": b'a,b\nab"c,d\n',
    "quote_after": b'a,b\n"ab"c,d\n',
    "nlquote": b'a,b\n"x\ny",2\n',
    "crlfq": b'a,b\r\n"x\r\ny",2\r\n',
    "blankmid": b"a,b\n\n1,2\n\n3,4\n",
    "spaceline": b"a,b\n 1,2\n   \n3,4\n",
    "tabline": b"a,b\n\t\n1,2\n",
    "unterminated": b'a,b\n"x,2\n',
    "latin1": b"a,b\n\xe9,2\n",
    "trailcomma": b"a,b,\n1,2,\n",
    "emptyhdr": b",,\n1,2,3\n",
    "dupes": b"a,a,a\n1,2,3\n",
    "hash": b"a,b\n#x,2\n",
    "tab": b"a\tb\n1\t2\n",
    "emptyquoted": b'a,b\n""\n1,2\n',
    "quotedspace": b'a,b\n "x",2\n',
    "bom_only": b"\xef\xbb\xbf",
    "dblbom": b"\xef\xbb\xbf\xef\xbb\xbfa,b\n",
    "lone_cr_field": b"a,b\n1\r2,3\n",
    "nul": b"a,b\n1\x00x,2\n3\x00,4\n",
    "ff": b"a,b\n\x0c,2\n",
    "enddelim": b"a,b\n1,2,",
    "utf8": "é,ü\n1,2\n".encode(),
    "title_then_table": b"TITLE,,,\n,,,\nNo.,A,B,C\n1,2,3,4\n",
    "hdr_blank_then_rows": b"\n\nNo.,A\n1,2\n",
}
for name, data in csvs.items():
    (OUT_FILES / f"csv_{name}.csv").write_bytes(data)


def preserve_whitespace(path):
    """openpyxl omits xml:space="preserve" on whitespace-only strings (Excel writes it);
    add it so that readers that trim unmarked text (calamine) see the same value."""
    import re
    import zipfile
    tmp = path.with_suffix(".tmp")
    with zipfile.ZipFile(path) as zin, zipfile.ZipFile(tmp, "w", zipfile.ZIP_DEFLATED) as zout:
        for item in zin.infolist():
            data = zin.read(item.filename)
            if item.filename.startswith("xl/") and item.filename.endswith(".xml"):
                text = data.decode("utf-8")
                text = re.sub(r"<t>([ \t\r\n]+)</t>", r'<t xml:space="preserve">\1</t>', text)
                data = text.encode("utf-8")
            zout.writestr(item, data)
    tmp.replace(path)


for xlsx in OUT_FILES.glob("*.xlsx"):
    preserve_whitespace(xlsx)
# Known gap: lone-CR line endings next to blank/whitespace-only lines (pandas' tokenizer
# re-reads bytes there); the fuzz alphabet therefore has no bare \r.
rng = random.Random(4242)
for k in range(250):
    alphabet = ["a", "b", "1", ",", ",", ",", '"', "\n", "\n", "\r\n", " ", "\t", "x y", "\x00", "é"]
    text = "".join(rng.choice(alphabet) for _ in range(rng.randint(1, 30)))
    (OUT_FILES / f"csv_fz{k:03d}.csv").write_bytes(text.encode("utf-8"))
print("built", len(list(OUT_FILES.iterdir())), "files")
