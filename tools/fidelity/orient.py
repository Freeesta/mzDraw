#!/usr/bin/env python3
"""Independent check of `scheme:orient` (plan-14): RDKit `rdDepictor.GenerateDepictionMatching2DStructure` depicts the product with
its core on the reactant's; the angle of the core in that depiction must equal the angle of the core in mzDraw's output, within 2 degrees.

    python3 tools/fidelity/orient.py          # needs `pip install rdkit`; exit 3 when RDKit is not installed

Inputs are the synthetic cases of tests/data/log/scheme-orient.json (the drawing before the command, `request.import`, and after it,
`out.mzdraw`). Nothing is fetched or committed from outside: RDKit is installed by the person who runs this.
"""
import json, math, os, sys

try:
    from rdkit import Chem, rdBase
    from rdkit.Chem import rdDepictor
    from rdkit.Geometry import Point3D
except ImportError:
    print("RDKit is not installed (pip install rdkit)", file=sys.stderr)
    sys.exit(3)

HERE = os.path.dirname(os.path.abspath(__file__))
CASES = os.path.join(HERE, "..", "..", "tests", "data", "log", "scheme-orient.json")


def pieces(doc, lo, hi):
    """(RDKit mol with the drawn coordinates, ids of its atoms in order) for the atoms with lo <= id < hi."""
    atoms = [a for a in doc["atoms"] if lo <= a["id"] < hi]
    m = Chem.RWMol()
    idx = {}
    for a in atoms:
        idx[a["id"]] = m.AddAtom(Chem.Atom(a["el"]))
    for b in doc["bonds"]:
        if b["a"] in idx and b["b"] in idx:
            m.AddBond(idx[b["a"]], idx[b["b"]], Chem.BondType.SINGLE)
    conf = Chem.Conformer(len(atoms))
    for a in atoms:
        conf.SetAtomPosition(idx[a["id"]], Point3D(a["x"], a["y"], 0.0))
    m.AddConformer(conf)
    return m.GetMol(), [a["id"] for a in atoms]


def angle(conf, i, j):
    p, q = conf.GetAtomPosition(i), conf.GetAtomPosition(j)
    return math.degrees(math.atan2(q.y - p.y, q.x - p.x))


def diff(a, b):
    return abs((a - b + 540) % 360 - 180)


def main():
    cases = json.load(open(CASES))["cases"]
    worst, n = 0.0, 0
    for case in cases:
        before = json.loads(case["request"]["import"])
        after = json.loads(case["out"]["mzdraw"])
        src, src_ids = pieces(before, 0, 100)
        tgt_in, tgt_ids = pieces(before, 100, 1000)
        ours, _ = pieces(after, 100, 1000)
        # the core: the source is a substructure of the product in the cases that have an RDKit reference
        match = tgt_in.GetSubstructMatch(src)
        if not match or len(match) < 3 or len(src_ids) != len(match):
            print("skip (no substructure match):", case["name"])
            continue
        ref = Chem.Mol(tgt_in)
        rdDepictor.GenerateDepictionMatching2DStructure(ref, src)
        # the angle of the core between its first and its last atom, from the source (truth of the reference), RDKit and ours
        a0, a1 = 0, len(match) - 1
        want = angle(src.GetConformer(), a0, a1)
        got_rd = angle(ref.GetConformer(), match[a0], match[a1])
        got_us = angle(ours.GetConformer(), match[a0], match[a1])
        d = diff(got_rd, got_us)
        worst, n = max(worst, d), n + 1
        print(f"{case['name'][:60]:60} source {want:8.2f}  rdkit {got_rd:8.2f}  mzdraw {got_us:8.2f}  |rdkit-mzdraw| {d:.3f}")
    print(f"RDKit {rdBase.rdkitVersion}: {n} cases compared, worst difference {worst:.3f} degrees (limit 2)")
    sys.exit(0 if n and worst <= 2.0 else 1)


if __name__ == "__main__":
    main()
