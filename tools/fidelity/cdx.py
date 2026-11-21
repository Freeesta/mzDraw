"""Fidelity check of the CDX reader against Open Babel (E17 of the backlog).

For each synthetic CDX file of the golden cases in tests/data/log/cdx.json, Open Babel reads the CDX and mzDraw's recorded
SMILES (the golden output of the same file) is read too; the InChIKeys of both are compared and one plain line is printed per
file. Needs `pip install openbabel-wheel`. Usage: python3 tools/fidelity/cdx.py
"""
import base64
import json
import os
import sys
import tempfile

from openbabel import openbabel, pybel

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
cases = json.load(open(os.path.join(root, "tests", "data", "log", "cdx.json")))["cases"]
print("Open Babel", openbabel.OBReleaseVersion())
bad = 0
for case in cases:
    # only the files that are read as they are (no operation after the import)
    if case["request"]["log"]:
        continue
    name = case["name"][:50]
    with tempfile.NamedTemporaryFile(suffix=".cdx", delete=False) as f:
        f.write(base64.b64decode(case["request"]["import_base64"]))
    try:
        theirs = {m.write("inchikey").strip() for m in pybel.readfile("cdx", f.name)}
        ours = pybel.readstring("smi", case["out"]["smiles"]).write("inchikey").strip()
    except Exception as e:  # a file one side cannot read is said, not hidden
        print("NOT COMPARED:", name, "-", e)
        bad += 1
        continue
    finally:
        os.unlink(f.name)
    if ours in theirs:
        print("SAME:", name)
    else:
        print("DIFFERENT:", name, "| mzDraw", ours, "| Open Babel", ", ".join(sorted(theirs)) or "nothing read")
        bad += 1
print("all files agree" if not bad else f"{bad} file(s) to look at")
sys.exit(1 if bad else 0)
