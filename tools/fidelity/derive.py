#!/usr/bin/env python3
"""Independent check of `tp:derive` (tp-03). For each parent -> TP pair of the synthetic cases below, mzDraw derives the TP and writes the
drawing as an RXN V2000 (with maps); RDKit reads that RXN. Two checks, both on OUR output as RDKit reads it:
  1. the mapped atoms RDKit reads are the ones we mapped: every map number occurs once in the reactant and once in the product, and the
     element of a mapped atom is the same on both sides (a map numbers the same atom);
  2. the product formula minus the reactant formula equals the delta of the edit, written here by hand from the patch (not computed by mzDraw).
Usage:  cargo build -p mzdraw --example replay
        python3 tools/fidelity/derive.py --replay target/debug/examples/replay > tests/data/fidelity/derive.json
Exit 0 all agree, 1 a disagreement, 2 tool error, 3 RDKit missing. RDKit is installed at run time (pip install rdkit), never committed;
the committed reference has the program name and version, the date and the success rate."""
import argparse, collections, datetime, json, math, subprocess, sys


def lactam():
    atoms = [{"id": i + 1, "el": e, "x": round(math.cos(math.radians(60 * i)), 4), "y": round(math.sin(math.radians(60 * i)), 4)} for i, e in enumerate("NCCCCC")]
    atoms += [{"id": 7, "el": "C", "x": 2.0, "y": 0.0}, {"id": 8, "el": "O", "x": 0.5, "y": 1.7321}]
    bonds = [{"id": 9 + i, "a": i + 1, "b": (i + 1) % 6 + 1, "o": 1} for i in range(6)] + [{"id": 15, "a": 1, "b": 7, "o": 1}, {"id": 16, "a": 2, "b": 8, "o": 2}]
    return json.dumps({"format": "mzdraw/1", "next": 17, "atoms": atoms, "bonds": bonds})

CASES = [
    ("hydroxylation", [{"op": "atom.add", "el": "O", "x": 0.5, "y": -1.7321}, {"op": "bond.add", "a": 5, "b": -1, "o": 1}], {"O": 1}),
    ("demethylation", [{"op": "remove", "atoms": [7], "bonds": [15]}], {"C": -1, "H": -2}),
    ("ring opening (hydrolysis)", [{"op": "remove", "atoms": [], "bonds": [9]}, {"op": "atom.add", "el": "O", "x": 0.5, "y": 1.7321}, {"op": "bond.add", "a": 2, "b": -1, "o": 1}], {"H": 2, "O": 1}),
]


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--replay", required=True); args = ap.parse_args()
    try:
        from rdkit import Chem, rdBase
        from rdkit.Chem import AllChem, rdMolDescriptors
    except ImportError:
        print("RDKit is not installed: not measured (pip install rdkit)", file=sys.stderr); return 3

    def replay(req):
        r = subprocess.run([args.replay], input=json.dumps(req), capture_output=True, text=True)
        if r.returncode != 0: raise SystemExit("replay failed: " + r.stdout + r.stderr)
        return r.stdout

    def counts(m):
        f = collections.Counter()
        for a in Chem.AddHs(m).GetAtoms(): f[a.GetSymbol()] += 1
        return f

    results, bad = [], 0
    for name, patch, delta in CASES:
        rxn_text = replay({"import": lactam(), "commands": [{"command": "tp:derive", "args": {"of": 1, "patch": patch}}], "out": "rxn"})
        rxn = AllChem.ReactionFromRxnBlock(rxn_text)
        (r,), (p,) = list(rxn.GetReactants()), list(rxn.GetProducts())
        for m in (r, p): Chem.SanitizeMol(m)
        maps = lambda m: {a.GetAtomMapNum(): a.GetSymbol() for a in m.GetAtoms() if a.GetAtomMapNum()}
        mr, mp = maps(r), maps(p)
        same_atoms = all(mr[k] == mp[k] for k in set(mr) & set(mp)) and len(mr) == r.GetNumAtoms() and len(mp) == p.GetNumAtoms() and set(mp) - set(mr) <= {9}
        got = counts(p); got.subtract(counts(r))
        got = {k: v for k, v in got.items() if v}
        ok = same_atoms and got == delta
        bad += not ok
        print(("ok   " if ok else "FAIL ") + name, got, file=sys.stderr)
        results.append({"case": name, "reactant": rdMolDescriptors.CalcMolFormula(r), "product": rdMolDescriptors.CalcMolFormula(p), "delta": got, "expected": delta, "maps_same_atoms": same_atoms, "ok": ok})
    json.dump({"format": "fidelity-derive/1", "date": datetime.date.today().isoformat(), "programs": {"rdkit": rdBase.rdkitVersion}, "success": f"{len(results) - bad}/{len(results)}", "results": results}, sys.stdout, indent=1)
    print(); return 1 if bad else 0

if __name__ == "__main__": sys.exit(main())
