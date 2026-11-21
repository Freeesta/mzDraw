#!/usr/bin/env python3
"""Independent check of `save("trasf")` (plan-12), in the layout of tools/fidelity/maps.py.

For each mapped reaction SMILES of tests/data/fidelity/trasf-set.txt (synthetic: hydroxylation, N-demethylation, ester hydrolysis,
substitution), RDKit writes the reaction as an RXN block (the input of mzDraw) and mzDraw (the native `replay` example) reads it and writes
`trasf/1` with its two records. RDKit then judges the INPUT reaction, not our records: `RunReactants` of the input reaction on the input
reactant must make the InChIKey of our product record; the elements of the mapped pairs, of the lost and of the added atoms, the reaction
centre (by element, not by atom index: ours are canonical indices) and the formula change are computed from the input and must equal what
we wrote (`rec["map"]`, `added`, `centre`, `delta`). The hashes and the record rules are checked by mzLab's own reader in
crates/mzdraw/tests/trasf.rs (the second check). Data goes to stdout, diagnostics to stderr.

Usage:
  cargo build -p mzdraw --example replay
  python3 tools/fidelity/trasf.py --replay target/debug/examples/replay > tests/data/fidelity/trasf.json

Exit 0 when every case agrees, 1 when one does not, 2 on a tool error, 3 when RDKit is not installed ("not measured"). RDKit is fetched at
run time (`pip install rdkit`) and never committed; what is committed is the reference output: the program name and version and
per case the checks, the InChIKey, the delta and an FNV-1a hash of our file.
"""
import argparse, json, os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
SET = os.path.join(ROOT, "tests", "data", "fidelity", "trasf-set.txt")


def fnv1a(text):
    h = 0xCBF29CE484222325
    for b in text.encode("utf-8"):
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--replay", required=True)
    args = ap.parse_args()
    try:
        from rdkit import Chem, rdBase
        from rdkit.Chem import AllChem, rdMolDescriptors
        from rdkit.Chem.inchi import MolToInchiKey
    except ImportError:
        print("RDKit is not installed: not measured (pip install rdkit)", file=sys.stderr)
        return 3
    if not os.path.exists(args.replay):
        print("build the replay example first: cargo build -p mzdraw --example replay", file=sys.stderr)
        return 2

    def mol_of(rec, maps):
        """The RDKit molecule of a mol/1 record; `maps` gives the map number of an atom index."""
        m = Chem.RWMol()
        for i, a in enumerate(rec["atoms"]):
            at = Chem.Atom(a["el"])
            at.SetNoImplicit(True)
            at.SetNumExplicitHs(a["h"])
            at.SetFormalCharge(a["chg"])
            at.SetAtomMapNum(maps.get(i, 0))
            m.AddAtom(at)
        order = {1: Chem.BondType.SINGLE, 2: Chem.BondType.DOUBLE, 3: Chem.BondType.TRIPLE}
        for b in rec["bonds"]:
            m.AddBond(b["a"], b["b"], order[b["o"]])
        out = m.GetMol()
        Chem.SanitizeMol(out)
        return out

    rows, bad = [], 0
    for line in open(SET):
        line = line.split("#")[0].strip()
        if not line:
            continue
        rxn = AllChem.ReactionFromSmarts(line, useSmiles=True)
        block = AllChem.ReactionToRxnBlock(rxn).rstrip("\n").replace("$MOL\n\n", "$MOL\nmol\n") + "\n"
        # RDKit writes an empty title line; mzDraw reads such a molfile one line short (found by this script, reported in the PR, not touched here)
        r = subprocess.run([args.replay], input=json.dumps({"import": block, "out": "trasf"}), capture_output=True, text=True)
        if r.returncode != 0:
            print("replay failed:", line, r.stdout, r.stderr, file=sys.stderr)
            return 2
        rec, parent, product = [json.loads(l) for l in r.stdout.strip().split("\n")]
        # what RDKit says about the INPUT reaction (not about our records): the products of RunReactants on the input reactant, the
        # elements of the mapped pairs, of the lost and of the added atoms, the reaction centre, the formula change
        react, prod = line.split(">>")
        rr, pp = Chem.MolFromSmiles(react), Chem.MolFromSmiles(prod)
        by_map = lambda m: {a.GetAtomMapNum(): a for a in m.GetAtoms() if a.GetAtomMapNum()}
        rm, pm_ = by_map(rr), by_map(pp)
        exp_pairs = sorted((rm[k].GetSymbol(), pm_[k].GetSymbol()) for k in rm if k in pm_)
        exp_lost = sorted(a.GetSymbol() for a in rr.GetAtoms() if not a.GetAtomMapNum())
        exp_added = sorted(a.GetSymbol() for a in pp.GetAtoms() if not a.GetAtomMapNum())

        def around(a):
            """what an atom stands in: element, hydrogens, charge, and its bonds by the map of the other end (0: not mapped)"""
            m = a.GetOwningMol()
            return (a.GetSymbol(), a.GetTotalNumHs(), a.GetFormalCharge(),
                    sorted((n.GetAtomMapNum(), str(m.GetBondBetweenAtoms(a.GetIdx(), n.GetIdx()).GetBondType())) for n in a.GetNeighbors()))
        exp_centre = sorted(rm[k].GetSymbol() for k in rm if k in pm_ and around(rm[k]) != around(pm_[k]))
        for a_ in rr.GetAtoms():
            a_.SetAtomMapNum(0)
        r_free = Chem.MolFromSmiles(Chem.MolToSmiles(rr))
        made = set()
        for x in AllChem.ReactionFromSmarts(line).RunReactants((r_free,)):
            try:
                Chem.SanitizeMol(x[0])
                made.add(MolToInchiKey(x[0]))
            except Exception:
                pass
        # what OUR record says
        els = lambda recd: [a["el"] for a in recd["atoms"]]
        pe, qe = els(parent), els(product)
        got_pairs = sorted((pe[a], qe[b]) for a, b in rec["map"] if b is not None)
        got_lost = sorted(pe[a] for a, b in rec["map"] if b is None)
        got_added = sorted(qe[b] for b in rec["added"])
        got_centre = sorted(pe[a] for a in rec["centre"])
        got_key = MolToInchiKey(Chem.MolFromSmiles(Chem.MolToSmiles(mol_of(product, {}))))
        # the delta: formula change of the input reaction, element by element
        def counts(m):
            f = rdMolDescriptors.CalcMolFormula(Chem.AddHs(m))
            return {e: int(n or 1) for e, n in re.findall(r"([A-Z][a-z]?)(\d*)", f)}
        cp, cq = counts(rr), counts(pp)
        diff = {e: cq.get(e, 0) - cp.get(e, 0) for e in set(cp) | set(cq)}
        written = {}
        for sign, body in re.findall(r"([+-])([A-Za-z0-9]+)", rec["delta"]):
            for e, n in re.findall(r"([A-Z][a-z]?)(\d*)", body):
                written[e] = written.get(e, 0) + (1 if sign == "+" else -1) * int(n or 1)
        checks = {"product": got_key in made, "pairs": got_pairs == exp_pairs, "lost": got_lost == exp_lost, "added": got_added == exp_added,
                  "centre": got_centre == exp_centre, "delta": {k: v for k, v in diff.items() if v} == written}
        ok = all(checks.values())
        bad += not ok
        want = got_key
        if not ok:
            print("   ", {k: v for k, v in checks.items() if not v}, file=sys.stderr)
        rows.append({"reaction": line, "delta": rec["delta"], "inchikey": want, "checks": checks, "ok": ok, "file_hash": fnv1a(r.stdout)})
        print(("ok   " if ok else "FAIL ") + line + "  " + rec["delta"], file=sys.stderr)
    out = {"program": "RDKit", "version": rdBase.rdkitVersion, "cases": rows}
    print(json.dumps(out, indent=2))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
