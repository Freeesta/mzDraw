#!/usr/bin/env python3
"""Second, independent check of the 12 targets of tests/data/velocity/tasks.json with RDKit `Chem.CanonSmiles`.
Run (needs `pip install rdkit`, fetched at run time, never committed):   python3 -I tools/velocity/check_targets.py
Writes one line per task: id, RDKit canonical SMILES, and for the stereo task the CIP labels. Exit 1 when a target is not read."""
import json, pathlib, sys
from rdkit import Chem, __version__
root = pathlib.Path(__file__).resolve().parents[2]
tasks = json.loads((root / "tests/data/velocity/tasks.json").read_text())["tasks"]
bad = 0
print(f"rdkit {__version__}")
for t in tasks:
    m = Chem.MolFromSmiles(t["target_smiles"])
    if m is None:
        print(f"{t['id']}\tNOT READ"); bad += 1; continue
    extra = ""
    if "@" in t["target_smiles"]:
        Chem.AssignStereochemistry(m, cleanIt=True, force=True)
        extra = "\tCIP " + ",".join(f"{i}{c}" for i, c in Chem.FindMolChiralCenters(m, useLegacyImplementation=False))
    print(f"{t['id']}\t{Chem.CanonSmiles(t['target_smiles'])}{extra}")
sys.exit(1 if bad else 0)
