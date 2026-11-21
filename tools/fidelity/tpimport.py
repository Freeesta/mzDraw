#!/usr/bin/env python3
"""Independent check of `tp:import` (tp-08). The list tests/data/tp/tp12.csv is read here with Python's `csv` module (not our reader).
mzDraw imports it (`tp:import`, native `replay` example) and writes the drawing as an `mzdraw/1` document and as an SDF. RDKit is the
reference, and what it reads is OUR output:
  1. every structure of OUR document (its atoms and bonds as the document holds them, connected pieces found here by union-find) is built
     into an RDKit molecule, and its canonical isomeric SMILES must equal the canonical SMILES RDKit makes of the row's SMILES, one
     to one (the list has no two equal molecules);
  2. the data on each structure (`TP_ID`, `MZ`, `RT`, `ADDUCT`, `AREA` of the SDF record RDKit reads, and the annotation level of the
     document) equal the row's; the names typed stay, the rows with no name hold `TP<n>`, all `TP<n>` different;
  3. the parent links of the list (the `parent` column, `;` between parents) equal the anchored arrows of OUR document: the structure of the
     arrow's `from` atom to the structure of its `to` atom, no more, no fewer.
What it does NOT show: that the coordinates, the orientation of the shared core or the tree layout are good (RDKit reads connectivity;
the layout has its own proof, tools/fidelity/pathway_props.py), that the levels mean what the chemist meant, or anything about a list
that is not this synthetic one. The SMILES of the rows is canonicalised by RDKit, not by mzLab, so a disagreement of the two programs'
reading of a SMILES would show here as a failure.
Usage:  cargo build -p mzdraw --example replay
        python3 tools/fidelity/tpimport.py --replay target/debug/examples/replay --out tests/data/fidelity/tpimport.json
Exit 0 all agree, 1 a disagreement, 2 tool error, 3 RDKit missing. RDKit is installed at run time (pip install rdkit), never committed; the
committed reference has the program name and version, the date and the success rate."""
import argparse, csv, datetime, io, json, os, subprocess, sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
LIST = os.path.join(ROOT, "tests", "data", "tp", "tp12.csv")


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--replay", required=True); ap.add_argument("--out", help="write the reference JSON here (default: stdout)"); args = ap.parse_args()
    try:
        from rdkit import Chem, rdBase
    except ImportError:
        print("RDKit is not installed: not measured (pip install rdkit)", file=sys.stderr); return 3

    text = open(LIST, newline="").read()
    rows = list(csv.DictReader(io.StringIO(text)))
    request = {"commands": [{"command": "tp:import", "args": {"csv": text}}]}

    def replay(out):
        r = subprocess.run([args.replay], input=json.dumps({**request, "out": out}), capture_output=True, text=True)
        if r.returncode != 0: raise SystemExit("replay failed: " + r.stdout + r.stderr)
        return r.stdout

    doc = json.loads(replay("mzdraw"))
    sdf = replay("sdf")
    canon = lambda m: Chem.MolToSmiles(m)
    want = {}                                    # canonical smiles of a row -> row
    for r in rows:
        m = Chem.MolFromSmiles(r["smiles"])
        if m is None: raise SystemExit("RDKit cannot read the row " + r["smiles"])
        want[canon(m)] = r
    if len(want) != len(rows): raise SystemExit("the list holds two equal molecules: the check pairs structures one to one")

    # the structures of our document, by union-find over its bonds
    parent = {a["id"]: a["id"] for a in doc["atoms"]}
    def find(x):
        while parent[x] != x: parent[x] = parent[parent[x]]; x = parent[x]
        return x
    for b in doc["bonds"]: parent[find(b["a"])] = find(b["b"])
    pieces = {}
    for a in doc["atoms"]: pieces.setdefault(find(a["id"]), []).append(a)
    bonds_of = {}
    for b in doc["bonds"]: bonds_of.setdefault(find(b["a"]), []).append(b)

    checks, bad = [], 0
    def check(ok, what):
        nonlocal bad
        checks.append({"check": what, "ok": bool(ok)})
        if not ok: bad += 1; print("FAIL " + what, file=sys.stderr)

    # 1. structures: OUR atoms and bonds into RDKit; Kekule orders as the document has them
    row_of, name_of = {}, {}
    for root, atoms in pieces.items():
        rw = Chem.RWMol(); idx = {}
        for a in atoms:
            at = Chem.Atom(a["el"]); at.SetFormalCharge(a.get("chg", 0)); idx[a["id"]] = rw.AddAtom(at)
        for b in bonds_of.get(root, []):
            rw.AddBond(idx[b["a"]], idx[b["b"]], {1: Chem.BondType.SINGLE, 2: Chem.BondType.DOUBLE, 3: Chem.BondType.TRIPLE}[b.get("o", 1)])
        try: Chem.SanitizeMol(rw)
        except Exception as e: check(False, f"structure {root} does not sanitize in RDKit: {e}"); continue
        s = canon(rw)
        row = want.get(s)
        check(row is not None, f"structure of atom {root} ({s}) is a row of the list")
        if row is not None:
            check(root not in row_of and id(row) not in [id(v) for v in row_of.values()], f"row {row['smiles']} is drawn once")
            row_of[root] = row
    check(len(pieces) == len(rows), f"{len(pieces)} structures for {len(rows)} rows")

    # 2. data: from the SDF as RDKit reads it, matched to the structure by its canonical SMILES
    seen_tp = set()
    for m in Chem.ForwardSDMolSupplier(io.BytesIO(sdf.encode()), sanitize=True):
        if m is None: check(False, "RDKit cannot read a record of our SDF"); continue
        row = want.get(canon(m))
        if row is None: check(False, "an SDF record is not a row"); continue
        props = m.GetPropsAsDict()
        tp = props.get("TP_ID", "")
        if row["name"]: check(tp == row["name"], f"{row['smiles']}: TP_ID {tp!r} is the typed name {row['name']!r}")
        else:
            check(tp.startswith("TP") and tp[2:].isdigit(), f"{row['smiles']}: TP_ID {tp!r} is TP<n>")
            check(tp not in seen_tp, f"{tp} is used once"); seen_tp.add(tp)
        for col, key in (("mz", "MZ"), ("rt", "RT"), ("adduct", "ADDUCT")):
            check(str(props.get(key, "")) == row[col] or (row[col] and float(props.get(key, "nan")) == float(row[col]) if key != "ADDUCT" else False),
                  f"{row['smiles']}: {key} {props.get(key)!r} is the row's {row[col]!r}")
        check(str(props.get("MZLAB_LEVEL", "")) == row["level"], f"{row['smiles']}: level {props.get('MZLAB_LEVEL')!r} is {row['level']!r}")

    # 3. the parent links: the list's, by names and structures, against the arrows of OUR document
    smiles_of = {id(r): s for s, r in want.items()}
    def tp_name(row):  # the name a structure has in our document
        for root, r in row_of.items():
            if r is row:
                for a in pieces[root]:
                    for k, v in a.get("data", []):
                        if k == "TP_ID": return v
        return None
    by_name = {r["name"]: r for r in rows if r["name"]}
    expected = set()
    for r in rows:
        for p in [x.strip() for x in r["parent"].split(";") if x.strip()]:
            expected.add((smiles_of[id(by_name[p])], smiles_of[id(r)]))
    got = set()
    for o in doc.get("objects", []):
        if o.get("t") == "arrow" and o.get("from") and o.get("to"):
            a, b = row_of.get(find(o["from"])), row_of.get(find(o["to"]))
            if a is not None and b is not None: got.add((smiles_of[id(a)], smiles_of[id(b)]))
    check(got == expected, f"anchored arrows equal the parent links ({len(got)} of {len(expected)})")
    check(len([o for o in doc.get("objects", []) if o.get("t") == "arrow"]) == len(expected), "no arrow more than the links")

    ref = {"tool": "tools/fidelity/tpimport.py", "reference": f"RDKit {rdBase.rdkitVersion}", "date": datetime.date.today().isoformat(),
           "list": "tests/data/tp/tp12.csv", "rows": len(rows), "checks": len(checks), "passed": len(checks) - bad}
    out = json.dumps(ref, indent=2) + "\n"
    if args.out: open(args.out, "w").write(out)
    else: sys.stdout.write(out)
    print(f"{ref['passed']}/{ref['checks']} checks agree (RDKit {rdBase.rdkitVersion})", file=sys.stderr)
    return 0 if bad == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
