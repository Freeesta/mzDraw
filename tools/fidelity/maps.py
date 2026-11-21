#!/usr/bin/env python3
"""Independent check of atom-to-atom maps (plan-11), in the layout of plan-03 (tools/fidelity/run.py, tests/data/fidelity/).

For each mapped reaction SMILES of tests/data/fidelity/maps-set.txt (20 synthetic reactions, written here), every molecule goes through
mzDraw (the native `replay` example: import, molfile out), the reaction file is read by mzDraw and written again as an RXN V2000, and
RDKit (`AllChem.ReactionFromRxnBlock`, `GetAtomMapNum`) reads that RXN and the SMILES of the whole drawing. The maps RDKit reads must be
the maps RDKit reads from the input SMILES. Data (the reference JSON) goes to stdout, diagnostics to stderr.

Usage:
  cargo build -p mzdraw --example replay
  python3 tools/fidelity/maps.py --replay target/debug/examples/replay > tests/data/fidelity/maps.json

Exit 0 when all reactions agree, 1 when one does not, 2 on a tool error, 3 when RDKit is not installed ("not measured"). RDKit is
fetched at run time (`pip install rdkit`, a virtual environment is fine) and never committed; what is committed is the reference output
for our synthetic inputs: the program name and version, the date, the maps RDKit read, and an FNV-1a hash of our RXN (so that
crates/mzdraw/tests/maps.rs, which needs no Python, notices when our writer changes and the tool must be rerun).
"""
import argparse, datetime, json, os, subprocess, sys


def fnv1a(text):
    """The same hash as `fnv1a` of run.py (not imported: run.py needs Open Babel at import time, this check does not)."""
    h = 0xCBF29CE484222325
    for b in text.encode("utf-8"):
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
SET = os.path.join(ROOT, "tests", "data", "fidelity", "maps-set.txt")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--replay", required=True, help="the native replay example (cargo build -p mzdraw --example replay)")
    args = ap.parse_args()
    try:
        from rdkit import Chem, rdBase
        from rdkit.Chem import AllChem
    except ImportError:
        print("RDKit is not installed: not measured (pip install rdkit)", file=sys.stderr)
        return 3
    if not os.path.exists(args.replay):
        print("build the replay example first: cargo build -p mzdraw --example replay", file=sys.stderr)
        return 2

    def replay(request):
        r = subprocess.run([args.replay], input=json.dumps(request), capture_output=True, text=True)
        if r.returncode != 0:
            raise SystemExit("replay failed: " + r.stdout + r.stderr)
        return r.stdout

    results, bad = [], 0
    for reaction in [l.strip() for l in open(SET) if l.strip()]:
        left, right = reaction.split(">>")
        sides = [left.split("."), right.split(".")]
        expected = [[sorted(a.GetAtomMapNum() for a in Chem.MolFromSmiles(s).GetAtoms() if a.GetAtomMapNum()) for s in side] for side in sides]
        mols = [[replay({"import": s, "out": "molfile"}) for s in side] for side in sides]
        block = "$RXN\n\n  synthetic\n\n%3d%3d\n" % (len(mols[0]), len(mols[1])) + "".join("$MOL\n" + m for side in mols for m in side)
        ours = replay({"import": block, "out": "rxn"})
        rxn = AllChem.ReactionFromRxnBlock(ours)
        got = [[sorted(a.GetAtomMapNum() for a in m.GetAtoms() if a.GetAtomMapNum()) for m in side] for side in (rxn.GetReactants(), rxn.GetProducts())]
        whole = replay({"import": ours, "out": "smiles"})
        from_smiles = sorted(a.GetAtomMapNum() for a in Chem.MolFromSmiles(whole).GetAtoms() if a.GetAtomMapNum())
        want = sorted(n for side in expected for mol in side for n in mol)
        ok = got == expected and from_smiles == want
        bad += not ok
        print(("ok   " if ok else "FAIL ") + reaction, file=sys.stderr)
        results.append({"reaction": reaction, "hash": fnv1a(ours), "rdkit_rxn": got, "rdkit_smiles": from_smiles, "same_as_input": ok})
    json.dump({
        "format": "fidelity-maps/1",
        "date": datetime.date.today().isoformat(),
        "programs": {"rdkit": rdBase.rdkitVersion},
        "hash": "fnv1a-64 of the RXN text our writer gives",
        "success": f"{len(results) - bad}/{len(results)}",
        "results": results,
    }, sys.stdout, indent=1)
    print()
    return 1 if bad else 0


sys.exit(main())
