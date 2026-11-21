#!/usr/bin/env python3
"""Independent check of the formulas of `digest/1` with RDKit (fetched at run time, never committed):

    pip install rdkit            # in a virtual environment, outside the repository
    cargo build --locked -p mzdraw --example replay
    python3 -I tools/digest/ref.py [--write tests/data/digest/rdkit-formulas.txt]

For each structure of the fidelity set (tests/data/fidelity/set.smi, synthetic) the native replay writes `{"import": SMILES, "out": "digest"}`;
RDKit reads the same SMILES and gives `CalcMolFormula` of each connected fragment. The multisets of the formulas are compared (a charge
is written the way the digest does: the count before the sign, `Fe2+`; RDKit writes `Fe+2`; an R group, which RDKit reads as the dummy atom `*`, is compared by its element counts). This reads OUR output (the bytes of the
digest) and RDKit's own parse of the SMILES; it recomputes nothing of ours. It shows that the formulas agree on this set; it does not
show the boxes, the ids or the labels (the box has its own test against the lines of the SVG). With --write the RDKit formulas of the set
are written with the RDKit version and the date (the reference output is committed; the program is not). Exit 0 when every structure
agrees, 1 when one does not, 2 when RDKit is not installed."""
import datetime, json, pathlib, re, subprocess, sys

try:
    from rdkit import Chem, __version__
    from rdkit.Chem.rdMolDescriptors import CalcMolFormula
except ImportError:
    print("ref.py: RDKit is not installed (pip install rdkit)", file=sys.stderr)
    sys.exit(2)

root = pathlib.Path(__file__).resolve().parents[2]
replay = root / "target/debug/examples/replay"
lines = [l.split() for l in (root / "tests/data/fidelity/set.smi").read_text().splitlines() if l.strip() and not l.startswith("#")]


def theirs(smiles):
    m = Chem.MolFromSmiles(smiles)
    if m is None:
        return None
    out = []
    for frag in Chem.GetMolFrags(m, asMols=True):
        f = CalcMolFormula(frag)
        # RDKit writes the charge as `+2` / `-`; the digest writes `2+` / `-`
        c = re.search(r"([+-])(\d*)$", f)
        if c:
            f = f[: c.start()] + c.group(2) + c.group(1)
        out.append(f)
    return sorted(out)


def counts(formula):
    """Element counts of a formula as a sorted list; `*` (RDKit's dummy atom) and `R` (the digest's R group) are one element."""
    body = re.sub(r"\d*[+-]$", "", formula)
    out = {}
    for el, n in re.findall(r"([A-Z][a-z]?|\*)(\d*)", body):
        el = "R" if el == "*" else el
        out[el] = out.get(el, 0) + int(n or 1)
    return sorted(out.items()), re.search(r"\d*[+-]$", formula)


def same(want, got):
    """Equal as written; a formula with a dummy atom is compared by element counts (RDKit sorts `*` first, the digest writes `R` as an element)."""
    if want is None or len(want) != len(got):
        return False
    # an unordered pairing: each side sorted, formulas with a dummy atom matched by their counts
    rest = list(got)
    for w in want:
        hit = next((g for g in rest if g == w or ("*" in w and counts(w)[0] == counts(g)[0] and bool(counts(w)[1]) == bool(counts(g)[1]))), None)
        if hit is None:
            return False
        rest.remove(hit)
    return True


def ours(smiles):
    p = subprocess.run([str(replay)], input=json.dumps({"import": smiles, "out": "digest"}), capture_output=True, text=True)
    if p.returncode != 0:
        print(f"ref.py: replay failed on {smiles}: {p.stderr}", file=sys.stderr)
        sys.exit(1)
    return sorted(piece["formula"] for piece in json.loads(p.stdout)["pieces"])


bad, rows = 0, []
for smiles, name in lines:
    want, got = theirs(smiles), ours(smiles)
    ok = same(want, got)
    bad += not ok
    rows.append((name, want))
    print(f"{name}\t{'agree' if ok else 'DIFFER'}\t{'.'.join(want or [])}" + ("" if ok else f"\tdigest: {'.'.join(got)}"))
print(f"{len(rows) - bad} of {len(rows)} agree (rdkit {__version__})", file=sys.stderr)
if "--write" in sys.argv:
    path = pathlib.Path(sys.argv[sys.argv.index("--write") + 1])
    head = [
        "# Hill formulas of the pieces of each structure of tests/data/fidelity/set.smi, from RDKit CalcMolFormula, sorted; charges written as the digest writes them.",
        f"# RDKit {__version__}, {datetime.date.today().isoformat()}; {len(rows) - bad} of {len(rows)} agree with digest/1; written by tools/digest/ref.py (RDKit is fetched at run time, not committed).",
    ]
    path.write_text("\n".join(head + [f"{n}\t{'.'.join(w or [])}" for n, w in rows]) + "\n")
sys.exit(1 if bad else 0)
