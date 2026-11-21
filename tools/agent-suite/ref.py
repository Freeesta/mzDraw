#!/usr/bin/env python3
"""Independent check of the targets of the agent task suite with RDKit (fetched at run time, never committed):

    pip install rdkit            # in a virtual environment
    python3 -I tools/agent-suite/ref.py [--write tests/data/agent-suite/inchikeys.txt]

For each task it takes the SMILES that the reference solution gives (the example `replay`, `out: smiles`) and the target pieces of
tasks.json, reads both with RDKit and compares the InChIKeys of the pieces as multisets. Prints one line per task. With --write the
keys of the targets are written with the RDKit version and the date (the keys are committed; the program is not). Exit 0 when every
task agrees, 1 when one does not (or RDKit does not read a SMILES), 2 when RDKit is not installed."""
import datetime, json, pathlib, subprocess, sys

try:
    from rdkit import Chem, __version__
    from rdkit.Chem import inchi
except ImportError:
    print("ref.py: RDKit is not installed (pip install rdkit)", file=sys.stderr)
    sys.exit(2)

root = pathlib.Path(__file__).resolve().parents[2]
suite = root / "tests/data/agent-suite"
replay = root / "target/debug/examples/replay"


def keys(smiles):
    out = []
    for piece in smiles.split("."):
        m = Chem.MolFromSmiles(piece)
        if m is None:
            return None
        out.append(inchi.MolToInchiKey(m))
    return sorted(out)


def ours(request):
    request = dict(json.loads(request), out="smiles")
    for k in ("report", "dry_run", "expect"):
        request.pop(k, None)
    p = subprocess.run([str(replay)], input=json.dumps(request), capture_output=True, text=True)
    if p.returncode != 0:
        print(f"ref.py: replay failed: {p.stderr}", file=sys.stderr)
        sys.exit(1)
    return p.stdout


tasks = json.loads((suite / "tasks.json").read_text())["tasks"]
rows, bad = [], 0
for t in tasks:
    # SMILES of the targets: bracket atoms of the task are read as written, so the pieces are joined with a dot
    want = keys(".".join(t["check"]["pieces"]))
    got = keys(ours((suite / "ref" / f"{t['id']}.json").read_text()))
    ok = want is not None and want == got
    bad += not ok
    rows.append((t["id"], want))
    print(f"{t['id']}\t{'agree' if ok else 'DIFFER'}\t{','.join(want or [])}")
print(f"rdkit {__version__}", file=sys.stderr)
if "--write" in sys.argv:
    path = pathlib.Path(sys.argv[sys.argv.index("--write") + 1])
    lines = [f"# InChIKeys of the targets of tests/data/agent-suite/tasks.json, one line per task: the keys of its pieces, sorted.",
             f"# RDKit {__version__}, {datetime.date.today().isoformat()}; written by tools/agent-suite/ref.py (RDKit is fetched at run time, not committed)."]
    lines += [f"{i}\t{','.join(k or [])}" for i, k in rows]
    path.write_text("\n".join(lines) + "\n")
sys.exit(1 if bad else 0)
