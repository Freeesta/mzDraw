#!/usr/bin/env python3
"""Independent check of `save("trasf")` (plan-12), in the layout of tools/fidelity/maps.py.

For each mapped reaction SMILES of tests/data/fidelity/trasf-set.txt (synthetic: hydroxylation, N-demethylation, ester hydrolysis,
substitution, one more of each kind), RDKit writes the reaction as an RXN block (the input of mzDraw), mzDraw (the native `replay` example)
reads it and writes `trasf/1` with its two records. This script then rebuilds the two molecules and the map from OUR record (not from the
input) as a mapped reaction (`ReactionFromSmarts` on the SMILES of the two molecules with the map numbers of the record's pairs), runs that
reaction on the parent with `RunReactants`, and compares the InChIKey of what RDKit makes with the InChIKey of the product record. The
formula difference RDKit computes from the two records must also equal the record's `delta`. The hashes and the record rules are checked by
mzLab's own reader in crates/mzdraw/tests/trasf.rs (the second check). Data goes to stdout, diagnostics to stderr.

Usage:
  cargo build -p mzdraw --example replay
  python3 tools/fidelity/trasf.py --replay target/debug/examples/replay > tests/data/fidelity/trasf.json

Exit 0 when every case agrees, 1 when one does not, 2 on a tool error, 3 when RDKit is not installed ("not measured"). RDKit is fetched at
run time (`pip install rdkit`) and never committed; what is committed is the reference output: the program name and version, the date and
per case the InChIKey RDKit made, the delta and an FNV-1a hash of our file.
"""
import argparse, datetime, json, os, re, subprocess, sys

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
        pairs = [(a, b) for a, b in rec["map"] if b is not None]
        pm = {a: k + 1 for k, (a, b) in enumerate(pairs)}
        qm = {b: k + 1 for k, (a, b) in enumerate(pairs)}
        # the reaction from OUR record: atoms by element and bonds only, the map numbers of the record's pairs; the hydrogens are RDKit's own
        def skeleton(rec, maps):
            m = Chem.RWMol()
            for i, a in enumerate(rec["atoms"]):
                at = Chem.Atom(a["el"])
                at.SetAtomMapNum(maps.get(i, 0))
                m.AddAtom(at)
            for b in rec["bonds"]:
                m.AddBond(b["a"], b["b"], {1: Chem.BondType.SINGLE, 2: Chem.BondType.DOUBLE, 3: Chem.BondType.TRIPLE}[b["o"]])
            return Chem.MolToSmarts(m.GetMol())
        template = AllChem.ReactionFromSmarts(skeleton(parent, pm) + ">>" + skeleton(product, qm))
        p_plain = mol_of(parent, {})
        p_free = Chem.MolFromSmiles(Chem.MolToSmiles(p_plain))        # hydrogens recomputed from the valence, not copied
        made = set()
        for x in template.RunReactants((p_free,)):
            try:
                Chem.SanitizeMol(x[0])
                made.add(MolToInchiKey(x[0]))
            except Exception:
                pass
        want = MolToInchiKey(Chem.MolFromSmiles(Chem.MolToSmiles(mol_of(product, {}))))
        # the delta: formula of the product minus the formula of the parent, element by element
        def counts(m):
            f = rdMolDescriptors.CalcMolFormula(Chem.AddHs(m))
            return {e: int(n or 1) for e, n in re.findall(r"([A-Z][a-z]?)(\d*)", f)}
        cp, cq = counts(p_plain), counts(mol_of(product, {}))
        diff = {e: cq.get(e, 0) - cp.get(e, 0) for e in set(cp) | set(cq)}
        written = {}
        for sign, body in re.findall(r"([+-])([A-Za-z0-9]+)", rec["delta"]):
            for e, n in re.findall(r"([A-Z][a-z]?)(\d*)", body):
                written[e] = written.get(e, 0) + (1 if sign == "+" else -1) * int(n or 1)
        ok = want in made and {k: v for k, v in diff.items() if v} == written
        bad += not ok
        rows.append({"reaction": line, "delta": rec["delta"], "inchikey": want, "rdkit_reproduces": want in made, "ok": ok, "file_hash": fnv1a(r.stdout)})
        print(("ok   " if ok else "FAIL ") + line + "  " + rec["delta"], file=sys.stderr)
    out = {"program": "RDKit", "version": rdBase.rdkitVersion, "date": datetime.date.today().isoformat(), "cases": rows}
    print(json.dumps(out, indent=2))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
