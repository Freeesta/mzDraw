#!/usr/bin/env python3
"""Independent check of the arrow labels of `pathway:label` (tp-07). It reads OUR output and recomputes nothing of ours: for each synthetic
scheme of tests/data/log/pathway-label.json it asks the native replay for the labelled document (`mzdraw`) and for its molfile, hands each
structure to RDKit (rdMolDescriptors.CalcMolFormula, Descriptors.ExactMolWt) and compares, arrow by arrow, the formula difference of the two
structures with the label the replay wrote. The mass difference is RDKit's: it is written to the reference file, since the mass labels are
not built yet (they need arch-web-17), so nothing of the masses is compared with our output.
Usage: python3 tools/fidelity/delta.py [--write] [path/to/replay]   (build the replay with `cargo build -p mzdraw --example replay`)
Without RDKit or the replay it prints `skipped` on stderr and exits 0. Reference output: tools/fidelity/delta.reference.json."""
import json, os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
args = [a for a in sys.argv[1:] if a != "--write"]
REPLAY = args[0] if args else os.path.join(ROOT, "target", "debug", "examples", "replay")
REF = os.path.join(ROOT, "tools", "fidelity", "delta.reference.json")
try:
    from rdkit import Chem, __version__ as RDV
    from rdkit.Chem import Descriptors, rdMolDescriptors
except ImportError:
    print("skipped: rdkit not installed (pip install rdkit)", file=sys.stderr); sys.exit(0)
if not os.path.exists(REPLAY):
    print("skipped: no replay example at " + REPLAY, file=sys.stderr); sys.exit(0)


def ask(req):
    r = subprocess.run([REPLAY], input=json.dumps(req), capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit("replay failed: " + r.stderr)
    return r.stdout


def counts(formula):
    return {el: int(n or 1) for el, n in re.findall(r"([A-Z][a-z]?)(\d*)", formula)}


def delta_of(a, b):
    d = {el: b.get(el, 0) - a.get(el, 0) for el in set(a) | set(b)}
    return {k: v for k, v in d.items() if v}


def tokens(text):
    """The signed deltas of a label as element counts (ASCII or U+2212 signs); None when the text holds anything else."""
    out = {}
    for t in text.replace("−", "-").split():
        if t == "isomer":
            continue
        if not re.fullmatch(r"([+-][A-Z][a-z]?\d*(?:[A-Z][a-z]?\d*)*)+", t):
            return None
        sign = 1
        for s, el, n in re.findall(r"([+-])|([A-Z][a-z]?)(\d*)", t):
            if s:
                sign = 1 if s == "+" else -1
            else:
                out[el] = out.get(el, 0) + sign * int(n or 1)
    return {k: v for k, v in out.items() if v}


cases = json.load(open(os.path.join(ROOT, "tests", "data", "log", "pathway-label.json")))["cases"]
rows, bad = [], 0
for case in cases:
    req = dict(case["request"])
    doc = json.loads(ask(dict(req, out="mzdraw")))
    whole = Chem.MolFromMolBlock(ask(dict(req, out="molfile")), sanitize=True)
    frags = Chem.GetMolFrags(whole, asMols=True, sanitizeFrags=True)
    index = {a["id"]: i for i, a in enumerate(doc["atoms"])}      # molfile order is the order of the atoms of the document
    frag_of = {}
    for f, mol in enumerate(Chem.GetMolFrags(whole)):
        for i in mol:
            frag_of[i] = f
    for o in doc["objects"]:
        if o.get("t") != "arrow" or not o.get("from") or not o.get("to"):
            continue
        a, b = frags[frag_of[index[o["from"]]]], frags[frag_of[index[o["to"]]]]
        want = delta_of(counts(rdMolDescriptors.CalcMolFormula(a)), counts(rdMolDescriptors.CalcMolFormula(b)))
        got = tokens(o.get("text", ""))
        dm = round(Descriptors.ExactMolWt(b) - Descriptors.ExactMolWt(a), 4)
        owned = got is not None
        ok = (got == want) if owned else True      # a label that is the person's text is not ours to compare
        bad += 0 if ok else 1
        rows.append({"case": case["name"], "arrow": o["id"], "label": o.get("text", ""), "ours": owned,
                     "rdkit_delta": {k: want[k] for k in sorted(want)}, "rdkit_mass_shift": dm, "agree": ok})
out = {"program": "RDKit " + RDV, "date": "2026-11-21", "rows": rows}
text = json.dumps(out, indent=1, ensure_ascii=False) + "\n"
if "--write" in sys.argv:
    open(REF, "w").write(text)
checked = sum(1 for r in rows if r["ours"])
print(f"delta: {checked - bad}/{checked} of our labels agree with RDKit's formula difference ({len(rows) - checked} are the person's text, not compared)")
sys.exit(1 if bad else 0)
