#!/usr/bin/env python3
"""Independent check of the stereo guard (plan-21), in the layout of plan-03 (tools/fidelity/run.py, tests/data/fidelity/).

For each stereo structure of tests/data/fidelity/set.smi (the lines with `@`, `/` or `\\`: centres and double bonds), two geometry edits are replayed natively (the `replay`
example): a plain flip of the whole drawing, and a drag of one neighbour of every wedged centre (moved to the point mirrored through the
centre) and of one neighbour of every double bond (moved to the mirror point across its line). For each, our signature (the SMILES of the drawing, `{"stereo":"kept"}` of `draw-result/1`) is compared with the stereo layer
of the InChI RDKit writes from the molfile of the drawing before and after: the layer must change exactly when our signature changes.
Data (the reference JSON) goes to stdout, diagnostics to stderr.

Usage:
  cargo build -p mzdraw --example replay
  python3 tools/fidelity/stereo.py --replay target/debug/examples/replay > tests/data/fidelity/stereo.json

Exit 0 when every case agrees, 1 when one does not, 2 on a tool error, 3 when RDKit is not installed ("not measured"). RDKit is fetched
at run time (`pip install rdkit`, a virtual environment is fine) and never committed; what is committed is the reference output for our
synthetic inputs: the program name and version, the date, and per case whether our signature and the InChI stereo layer changed.
"""
import argparse, datetime, json, os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
SET = os.path.join(ROOT, "tests", "data", "fidelity", "set.smi")


def run(replay, request):
    r = subprocess.run([replay], input=json.dumps(request), capture_output=True, text=True)
    if r.returncode not in (0, 1):
        raise RuntimeError(r.stderr.strip())
    return r.stdout


def stereo_layer(inchi):
    """The layers of an InChI that carry the configuration: /b (double bonds), /t, /m and /s (centres)."""
    return [p for p in inchi.split("/")[1:] if p[0] in "btms"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--replay", required=True)
    a = ap.parse_args()
    try:
        from rdkit import Chem, rdBase
        from rdkit.Chem import inchi
    except ImportError:
        print("not measured: RDKit is not installed (pip install rdkit)", file=sys.stderr)
        return 3

    def layer(molfile):
        m = Chem.MolFromMolBlock(molfile)
        return stereo_layer(inchi.MolToInchi(m)) if m is not None else None

    cases = []
    for line in open(SET):
        smi, name = line.split()
        if not any(c in smi for c in "@/\\"):
            continue
        start = json.loads(run(a.replay, {"import": smi, "out": "mzdraw"}))
        atoms = {x["id"]: x for x in start["atoms"]}
        ids = sorted(atoms)
        edits = [("flip", [{"op": "flip", "atoms": ids, "axis": "h", "at": 0.0}])]
        centres = sorted({b["a"] for b in start["bonds"] if b.get("s")})
        drag = []
        for c in centres:
            n = next((b["b"] if b["a"] == c else b["a"] for b in start["bonds"] if c in (b["a"], b["b"]) and not b.get("s")), None)
            if n is not None:
                p, q = atoms[c], atoms[n]
                drag.append({"op": "move", "atoms": [n], "dx": 2 * (p["x"] - q["x"]), "dy": 2 * (p["y"] - q["y"])})
        # a double bond: one neighbour of its first atom goes to the mirror point across the line of the bond
        for b in start["bonds"]:
            if b.get("o") != 2:
                continue
            p, q = atoms[b["a"]], atoms[b["b"]]
            n = next((x["b"] if x["a"] == b["a"] else x["a"] for x in start["bonds"] if b["a"] in (x["a"], x["b"]) and x["id"] != b["id"]), None)
            if n is None or not any(b["b"] in (x["a"], x["b"]) and x["id"] != b["id"] for x in start["bonds"]):
                continue
            r = atoms[n]
            ux, uy = q["x"] - p["x"], q["y"] - p["y"]
            t = ((r["x"] - p["x"]) * ux + (r["y"] - p["y"]) * uy) / (ux * ux + uy * uy)
            fx, fy = p["x"] + t * ux, p["y"] + t * uy
            drag.append({"op": "move", "atoms": [n], "dx": 2 * (fx - r["x"]), "dy": 2 * (fy - r["y"])})
        edits.append(("drag", drag))
        base = run(a.replay, {"import": smi, "out": "molfile"})
        for edit, log in edits:
            res = json.loads(run(a.replay, {"import": smi, "log": log, "expect": [{"stereo": "kept"}]}))
            ours = not res["ok"]
            after = layer(run(a.replay, {"import": smi, "log": log, "out": "molfile"}))
            before = layer(base)
            theirs = before != after
            cases.append({"name": name, "edit": edit, "ours_changed": ours, "inchi_changed": theirs, "agree": ours == theirs})
    bad = [c for c in cases if not c["agree"]]
    json.dump({
        "format": "stereo-reference/1",
        "date": datetime.date.today().isoformat(),
        "programs": {"rdkit": rdBase.rdkitVersion},
        "cases": len(cases),
        "agree": len(cases) - len(bad),
        "flips_changed": sum(c["ours_changed"] for c in cases if c["edit"] == "flip"),
        "drags_changed": sum(c["ours_changed"] for c in cases if c["edit"] == "drag"),
        "results": cases,
    }, sys.stdout, indent=1)
    print(file=sys.stdout)
    for c in bad:
        print("disagree:", c, file=sys.stderr)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
