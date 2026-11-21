#!/usr/bin/env python3
"""Independent check of the diff lens `lens/1` (tp-04). For each synthetic parent -> product pair below, mzDraw derives the product and
(1) writes the drawing as an RXN V2000 with atom maps, (2) gives its own `lens/1`. RDKit reads the RXN and, from ITS OWN mapped atoms and
bonds (element, charge, isotope, radical electrons and total hydrogen count of the atoms; bond order of the bonds), lists the added,
changed and cleaved sets and the lost maps; the element-count change comes from RDKit's formulas with explicit hydrogens. Our `lens/1`
is translated to map numbers with our own document and must equal RDKit's sets.
What it does NOT show: that the maps are the ones a chemist would assign (they are our numbering, read back), that the tint is drawn
where the sets say (that is the display list, tested in Rust and in Chromium), or anything about pairs without maps (no lens there).
Usage:  cargo build -p mzdraw --example replay
        python3 tools/fidelity/lens.py --replay target/debug/examples/replay --out tests/data/fidelity/lens.json
Exit 0 all agree, 1 a disagreement, 2 tool error, 3 RDKit missing. RDKit is installed at run time (pip install rdkit), never committed;
the committed reference has the program name and version, the date and the success rate."""
import argparse, collections, datetime, json, math, re, subprocess, sys


def lactam():
    atoms = [{"id": i + 1, "el": e, "x": round(math.cos(math.radians(60 * i)), 4), "y": round(math.sin(math.radians(60 * i)), 4)} for i, e in enumerate("NCCCCC")]
    atoms += [{"id": 7, "el": "C", "x": 2.0, "y": 0.0}, {"id": 8, "el": "O", "x": 0.5, "y": 1.7321}]
    bonds = [{"id": 9 + i, "a": i + 1, "b": (i + 1) % 6 + 1, "o": 1} for i in range(6)] + [{"id": 15, "a": 1, "b": 7, "o": 1}, {"id": 16, "a": 2, "b": 8, "o": 2}]
    return json.dumps({"format": "mzdraw/1", "next": 17, "atoms": atoms, "bonds": bonds})


ADD_O = [{"op": "atom.add", "el": "O", "x": -1.0, "y": -1.7321}, {"op": "bond.add", "a": 5, "b": -1, "o": 1}]
CASES = [
    ("identical copy", []),
    ("hydroxylation (atom and bond added, carbon changed)", ADD_O),
    ("demethylation (atom lost, nitrogen changed)", [{"op": "remove", "atoms": [7], "bonds": [15]}]),
    ("ring opening by hydrolysis (bond cleaved, oxygen added)", [{"op": "remove", "atoms": [], "bonds": [9]}, {"op": "atom.add", "el": "O", "x": 1.366, "y": 1.366}, {"op": "bond.add", "a": 2, "b": -1, "o": 1}]),
    ("C=O reduced to C-O (bond order changed)", [{"op": "bond.set", "id": 16, "o": 1}]),
    ("ring closure across the ring (bond formed between existing atoms)", [{"op": "bond.add", "a": 3, "b": 6, "o": 1}]),
    ("carbon replaced by nitrogen (element changed)", [{"op": "atom.set", "id": 5, "el": "N"}]),
    ("isotope label (isotope changed)", [{"op": "atom.set", "id": 7, "iso": 13}]),
    ("N-oxide (charge changed, atom added)", [{"op": "atom.add", "el": "O", "x": 1.5, "y": -1.2}, {"op": "bond.add", "a": 1, "b": -1, "o": 1}, {"op": "atom.set", "id": 1, "chg": 1}, {"op": "atom.set", "id": -1, "chg": -1}]),
    ("chain of three carbons grown on a ring carbon", [{"op": "atom.add", "el": "C", "x": -1.0, "y": -1.0}, {"op": "bond.add", "a": 4, "b": -1, "o": 1}, {"op": "atom.add", "el": "C", "x": -1.0, "y": -2.0}, {"op": "bond.add", "a": -1, "b": -2, "o": 1}, {"op": "atom.add", "el": "C", "x": -1.0, "y": -3.0}, {"op": "bond.add", "a": -2, "b": -3, "o": 1}]),
]


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--replay", required=True); ap.add_argument("--out"); args = ap.parse_args()
    try:
        from rdkit import Chem, rdBase
        from rdkit.Chem import AllChem
    except ImportError:
        print("RDKit is not installed: not measured (pip install rdkit)", file=sys.stderr); return 3

    def replay(req):
        r = subprocess.run([args.replay], input=json.dumps(req), capture_output=True, text=True)
        if r.returncode != 0: raise SystemExit("replay failed: " + r.stdout + r.stderr)
        return r.stdout

    def atoms_of(m):
        return {a.GetAtomMapNum(): (a.GetSymbol(), a.GetFormalCharge(), a.GetIsotope(), a.GetNumRadicalElectrons(), a.GetTotalNumHs()) for a in m.GetAtoms()}

    def bonds_of(m):
        return {tuple(sorted((b.GetBeginAtom().GetAtomMapNum(), b.GetEndAtom().GetAtomMapNum()))): int(b.GetBondTypeAsDouble()) for b in m.GetBonds()}

    def formula(m):
        c = collections.Counter()
        for a in Chem.AddHs(m).GetAtoms(): c[a.GetSymbol()] += 1
        return c

    def theirs(r, p):
        ra, pa, rb, pb = atoms_of(r), atoms_of(p), bonds_of(r), bonds_of(p)
        f = formula(p); f.subtract(formula(r))
        return {
            "added": sorted(set(pa) - set(ra)),
            "changed": sorted(m for m in set(pa) & set(ra) if pa[m] != ra[m]),
            "added_bonds": sorted(k for k in pb if k not in rb),
            "changed_bonds": sorted(k for k in pb if k in rb and pb[k] != rb[k]),
            "cleaved": sorted(k for k in rb if k not in pb and k[0] in pa and k[1] in pa),
            "lost": sorted(set(ra) - set(pa)),
            "delta": {k: v for k, v in f.items() if v},
        }

    def ours(doc, lens):
        mp = {a["id"]: a.get("map", 0) for a in doc["atoms"]}
        bd = {b["id"]: tuple(sorted((mp[b["a"]], mp[b["b"]]))) for b in doc["bonds"]}
        (l,) = lens["arrows"]
        # the delta is written gains first then losses: a sign applies to every element up to the next sign
        d, sign = {}, 1
        for tok in re.findall(r"[+-]|[A-Z][a-z]?\d*", l["delta"]):
            if tok in "+-": sign = 1 if tok == "+" else -1
            else:
                el, n = re.match(r"([A-Z][a-z]?)(\d*)", tok).groups(); d[el] = d.get(el, 0) + sign * int(n or 1)
        return {
            "added": sorted(mp[i] for i in l["added"]),
            "changed": sorted(mp[i] for i in l["changed"]),
            "added_bonds": sorted(bd[i] for i in l["added_bonds"]),
            "changed_bonds": sorted(bd[i] for i in l["changed_bonds"]),
            "cleaved": sorted(tuple(sorted((mp[a], mp[b]))) for a, b in l["cleaved"]),
            "lost": sorted(l["lost"]),
            "delta": d,
        }

    results, bad = [], 0
    for name, patch in CASES:
        req = {"import": lactam(), "commands": [{"command": "tp:derive", "args": {"of": 1, "patch": patch}}]}
        rxn = AllChem.ReactionFromRxnBlock(replay({**req, "out": "rxn"}))
        (r,), (p,) = list(rxn.GetReactants()), list(rxn.GetProducts())
        for m in (r, p): Chem.SanitizeMol(m)
        doc = json.loads(replay({**req, "out": "mzdraw"})); lens = json.loads(replay({**req, "out": "lens"}))
        t, o = theirs(r, p), ours(doc, lens)
        ok = t == o
        bad += not ok
        print(("ok   " if ok else "FAIL ") + name, file=sys.stderr)
        if not ok: print("  rdkit:", t, "\n  ours: ", o, file=sys.stderr)
        results.append({"case": name, "rdkit": {k: (v if k == "delta" else [list(x) if isinstance(x, tuple) else x for x in v]) for k, v in t.items()}, "ok": ok})
    ref = {"format": "fidelity-lens/1", "date": datetime.date.today().isoformat(), "programs": {"rdkit": rdBase.rdkitVersion}, "success": f"{len(results) - bad}/{len(results)}", "results": results}
    out = open(args.out, "w") if args.out else sys.stdout
    json.dump(ref, out, indent=1); out.write("\n")
    return 1 if bad else 0

if __name__ == "__main__": sys.exit(main())
