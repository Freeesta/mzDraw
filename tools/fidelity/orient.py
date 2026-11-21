#!/usr/bin/env python3
"""Independent check of `scheme:orient` (plan-14): RDKit `rdDepictor.GenerateDepictionMatching2DStructure` depicts the product with
its core on the reactant's; the angle of the core in that depiction must equal the angle of the core in the output of the command, within 2 degrees.

    python3 tools/fidelity/orient.py [--write]   # needs `pip install rdkit`; exit 3 when RDKit is not installed;
                                                 # --write saves tools/fidelity/orient.reference.json

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


def structures(doc):
    """The connected structures of a drawing as lists of atom ids, ordered by their smallest id (a union of the bonds)."""
    parent = {a["id"]: a["id"] for a in doc["atoms"]}

    def root(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for b in doc["bonds"]:
        parent[root(b["a"])] = root(b["b"])
    groups = {}
    for a in doc["atoms"]:
        groups.setdefault(root(a["id"]), []).append(a["id"])
    return sorted((sorted(g) for g in groups.values()), key=lambda g: g[0])


def mol_of(doc, ids):
    """An RDKit mol with the drawn coordinates for the atoms `ids` (in that order) and the bonds between them."""
    keep = set(ids)
    m = Chem.RWMol()
    idx = {}
    by_id = {a["id"]: a for a in doc["atoms"]}
    for i in ids:
        idx[i] = m.AddAtom(Chem.Atom(by_id[i]["el"]))
    for b in doc["bonds"]:
        if b["a"] in keep and b["b"] in keep:
            m.AddBond(idx[b["a"]], idx[b["b"]], Chem.BondType.SINGLE)
    conf = Chem.Conformer(len(ids))
    for i in ids:
        conf.SetAtomPosition(idx[i], Point3D(by_id[i]["x"], by_id[i]["y"], 0.0))
    m.AddConformer(conf)
    return m.GetMol()


def angle(conf, i, j):
    p, q = conf.GetAtomPosition(i), conf.GetAtomPosition(j)
    return math.degrees(math.atan2(q.y - p.y, q.x - p.x))


def diff(a, b):
    return abs((a - b + 540) % 360 - 180)


def main():
    cases = json.load(open(CASES))["cases"]
    worst, n, skipped = 0.0, 0, 0
    for case in cases:
        before = json.loads(case["request"]["import"])
        after = json.loads(case["out"]["mzdraw"])
        # each structure is compared with the one before it, in the order of their smallest atom id (the order of the scheme)
        groups = structures(before)
        for k in range(1, len(groups)):
            name = f"{case['name'][:48]} [{k}/{len(groups) - 1}]"
            src, tgt_in, ours = mol_of(before, groups[k - 1]), mol_of(before, groups[k]), mol_of(after, groups[k])
            # RDKit has a reference where the source is an exact substructure of the product
            match = tgt_in.GetSubstructMatch(src)
            if not match or len(match) < 3 or src.GetNumAtoms() != len(match):
                print("skip (no exact substructure match, RDKit has no reference):", name)
                skipped += 1
                continue
            # the reference: RDKit depicts the product with its core on the source as it stands AFTER the command (for the second
            # step of a chain, the source is the structure the first step turned)
            src_out = mol_of(after, groups[k - 1])
            ref = Chem.Mol(tgt_in)
            rdDepictor.GenerateDepictionMatching2DStructure(ref, src_out)
            # the angle of the core between its first and its last atom: the source, RDKit's product and ours
            a0, a1 = 0, len(match) - 1
            want = angle(src_out.GetConformer(), a0, a1)
            got_rd = angle(ref.GetConformer(), match[a0], match[a1])
            got_us = angle(ours.GetConformer(), match[a0], match[a1])
            d = diff(got_rd, got_us)
            worst, n = max(worst, d), n + 1
            print(f"{name:60} source {want:8.2f}  rdkit {got_rd:8.2f}  ours {got_us:8.2f}  |rdkit-ours| {d:.3f}")
    print(f"RDKit {rdBase.rdkitVersion}: {n} pairs of structures compared ({len(cases)} cases), {skipped} skipped (no exact substructure match), worst difference {worst:.3f} degrees (limit 2)")
    if "--write" in sys.argv:
        # the reference output: counts, version and date, no third-party data
        import datetime
        ref = {"program": "RDKit", "version": rdBase.rdkitVersion, "date": datetime.date.today().isoformat(), "script": "tools/fidelity/orient.py",
               "cases": len(cases), "pairs_compared": n, "pairs_skipped": skipped, "worst_degrees": round(worst, 3), "limit_degrees": 2.0}
        with open(os.path.join(HERE, "orient.reference.json"), "w") as f:
            json.dump(ref, f, indent=1)
            f.write("\n")
    sys.exit(0 if n and worst <= 2.0 else 1)


if __name__ == "__main__":
    main()
