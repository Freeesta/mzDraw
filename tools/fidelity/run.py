#!/usr/bin/env python3
"""Format fidelity of mzDraw checked by programs that are not ours (plan-03).

For each structure of tests/data/fidelity/set.smi and each format we write (molfile V2000 and V3000, SDF, SMILES, KET, CDXML, CML,
and RXN for a few reaction drawings), the file comes from the native `replay` example. RDKit and Open Babel (a separate program, never
linked) read it, and the InChIKey of what they read is compared with the InChIKey of the input. The reverse direction: RDKit and Open
Babel write molfile, SDF and SMILES, we read them (`replay` with `import`), and the InChIKey of our SMILES is compared with the input's.

Usage:
  python3 tools/fidelity/run.py --replay target/debug/examples/replay > tests/data/fidelity/reference.json
  python3 tools/fidelity/run.py --replay target/debug/examples/replay --corpus   # optional: public ids fetched at run time, rates only

Exit 0; 1 on a tool error; diagnostics on stderr, data on stdout. Needs `rdkit` (pip) and `obabel` (Open Babel 3.1.x) on PATH; the
InChI of RDKit is the one bundled with it, and `inchi-1` (IUPAC reference binary) is used only when present on PATH or in INCHI_1.
Nothing third-party is committed: only our synthetic structures, the hashes of our files and the keys.
"""
import argparse
import datetime
import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
SET = os.path.join(ROOT, "tests", "data", "fidelity", "set.smi")

# (name in the JSON, `out` of the replay request, obabel input format)
FORMATS = [
    ("molfile-v2000", "molfile", "mol"),
    ("molfile-v3000", "molfile-v3000", "mol"),
    ("sdf", "sdf", "sdf"),
    ("smiles", "smiles", "smi"),
    ("ket", "ket", None),
    ("cdxml", "cdxml", "cdxml"),
    ("cml", "cml", "cml"),
]
LAYERS = "chqpbtmsir"  # the InChI layers after the formula (/c connectivity, /h H, /q /p charge, /b /t /m /s stereo, /i /r isotopes)


def fnv1a(text: str) -> str:
    h = 0xCBF29CE484222325
    for b in text.encode("utf-8"):
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def stable(text: str) -> str:
    """A molfile of another program without its stamp line (line 2: program, date and time), so the hash is the same on every run."""
    lines = text.split("\n")
    if len(lines) > 2 and "V2000" in text[:400] or "V3000" in text[:400]:
        lines[1] = ""
    return "\n".join(lines)


def die(msg: str) -> None:
    print(f"run.py: {msg}", file=sys.stderr)
    sys.exit(1)


try:
    from rdkit import Chem, RDLogger
    from rdkit.Chem import AllChem, inchi, rdChemReactions
    import rdkit
except ImportError:
    die("RDKit is missing: pip install rdkit")
RDLogger.DisableLog("rdApp.*")
OBABEL = shutil.which("obabel")
if not OBABEL:
    die("obabel (Open Babel 3.1.x) is missing")
INCHI1 = os.environ.get("INCHI_1") or shutil.which("inchi-1")


def version(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True)
    return (r.stdout + r.stderr).strip().splitlines()[0] if r.returncode == 0 or r.stdout else "unknown"


# ---------------------------------------------------------------- our side
REPLAY = None


def replay(request: dict):
    r = subprocess.run([REPLAY], input=json.dumps(request), capture_output=True, text=True)
    if r.returncode != 0:
        return None, r.stderr.strip()
    return r.stdout, None


# ---------------------------------------------------------------- InChI of an RDKit molecule
def normalise(m):
    """InChI has no dummy atom: an R group (or `*`) becomes the element 103 with its label as the isotope, the same for every reader."""
    m = Chem.Mol(m)
    for a in m.GetAtoms():
        if a.GetAtomicNum() == 0:
            label = a.GetAtomMapNum() or (a.GetIntProp("_MolFileRLabel") if a.HasProp("_MolFileRLabel") else 0) or a.GetIsotope()
            a.SetAtomicNum(103)
            a.SetIsotope(label)
            a.SetAtomMapNum(0)
    return m


def inchi_of(m):
    if m is None:
        return None
    try:
        s = inchi.MolToInchi(normalise(m))
    except Exception:
        return None
    return s or None


def key_of(s):
    return inchi.InchiToInchiKey(s) if s else None


def layers(s):
    out = {"f": s.split("/")[1] if s and s.count("/") >= 1 else ""}
    for part in (s or "").split("/")[2:]:
        out[part[0]] = part
    return out


def lost_between(a, b):
    """Layers of the standard InChI that differ between the input (a) and what a program read (b)."""
    if a == b:
        return []
    la, lb = layers(a), layers(b)
    return [k for k in "f" + LAYERS if la.get(k) != lb.get(k)]


# ---------------------------------------------------------------- readers
def rdkit_read(fmt, text):
    try:
        if fmt in ("molfile-v2000", "molfile-v3000"):
            return Chem.MolFromMolBlock(text, sanitize=True, removeHs=True)
        if fmt == "sdf":
            sup = Chem.SDMolSupplier()
            sup.SetData(text)
            ms = [m for m in sup]  # one record per piece of a structure: the pieces together are the structure
            if not ms or any(m is None for m in ms):
                return None
            combo = ms[0]
            for x in ms[1:]:
                combo = Chem.CombineMols(combo, x)
            return combo
        if fmt == "smiles":
            return Chem.MolFromSmiles(text.strip())
        if fmt == "cdxml":
            ms = Chem.MolsFromCDXML(text)
            if not ms:
                return None
            combo = ms[0]
            for x in ms[1:]:
                combo = Chem.CombineMols(combo, x)
            Chem.SanitizeMol(combo)
            return combo
    except Exception:
        return None
    return None


RDKIT_READS = {"molfile-v2000", "molfile-v3000", "sdf", "smiles", "cdxml"}


def tmp_file(tmp, suffix, text):
    """A file of its own per call: the runs are parallel."""
    fd, path = tempfile.mkstemp(suffix=suffix, dir=tmp)
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(text)
    return path


def obabel_inchi(fmt, text, tmp):
    """InChI of a text read by Open Babel (`obabel -i<fmt> file -oinchi`); None when it cannot read or write it."""
    path = tmp_file(tmp, "." + fmt, text)
    r = subprocess.run([OBABEL, f"-i{fmt}", path, "-oinchi"] + (["-j"] if fmt == "sdf" else []), capture_output=True, text=True)
    os.unlink(path)
    lines = [l for l in r.stdout.splitlines() if l.startswith("InChI=")]
    return lines[0] if len(lines) == 1 else None


def obabel_write(smiles, fmt, tmp):
    """The same structure written by Open Babel: molfile and SDF with 2D coordinates, or canonical SMILES."""
    args = [OBABEL, f"-:{smiles}", f"-o{fmt}"] + (["--gen2d"] if fmt in ("mol", "sdf") else [])
    r = subprocess.run(args, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 and r.stdout.strip() else None


def rdkit_write(m, fmt):
    try:
        m = Chem.Mol(m)
        if fmt == "smi":
            return Chem.MolToSmiles(m) + "\n"
        AllChem.Compute2DCoords(m)
        Chem.WedgeMolBonds(m, m.GetConformer())
        mb = Chem.MolToMolBlock(m)
        return mb if fmt == "mol" else mb + "$$$$\n"
    except Exception:
        return None


# ---------------------------------------------------------------- the run
def read_set():
    rows = []
    for line in open(SET, encoding="utf-8"):
        line = line.strip()
        if line:
            s, name = line.split()
            rows.append((name.split("-")[0], name.split("-", 1)[1], s))
    return rows


def forward(row, tmp):
    sid, cat, smiles = row
    m = Chem.MolFromSmiles(smiles)
    inc = inchi_of(m)
    entry = {"category": cat, "input": smiles, "inchikey_in": key_of(inc)}
    for name, out, obfmt in FORMATS:
        text, err = replay({"import": smiles, "out": out})
        if text is None:
            entry[name] = {"hash": None, "error": err, "inchikey_in": entry["inchikey_in"], "inchikey_out_rdkit": None, "inchikey_out_obabel": None, "lost": ["write_error"]}
            continue
        e = {"hash": fnv1a(text), "inchikey_in": entry["inchikey_in"]}
        lost = set()
        rd = None
        if name in RDKIT_READS:
            rd = inchi_of(rdkit_read(name, text))
            e["inchikey_out_rdkit"] = key_of(rd)
            lost.update(lost_between(inc, rd) if rd else ["read_error:rdkit"])
        else:
            e["inchikey_out_rdkit"] = None  # RDKit has no reader of this format
        ob = None
        if obfmt:
            ob = obabel_inchi(obfmt, text, tmp)
            # Open Babel has no InChI of a structure with a dummy atom: that is "not comparable", not a loss
            e["inchikey_out_obabel"] = key_of(ob)
            if ob:
                lost.update(lost_between(inc, ob))
            elif "R" not in cat[:1] and cat != "rgroup":
                lost.add("read_error:obabel")
        else:
            e["inchikey_out_obabel"] = None  # Open Babel has no reader of this format
        e["lost"] = sorted(lost)
        entry[name] = e
    return sid, entry


def reverse(row, tmp):
    sid, cat, smiles = row
    m = Chem.MolFromSmiles(smiles)
    inc = inchi_of(m)
    entry = {}
    for prog, writer in (("rdkit", lambda f: rdkit_write(m, f)), ("obabel", lambda f: obabel_write(smiles, f, tmp))):
        for fmt, label in (("mol", "molfile"), ("sdf", "sdf"), ("smi", "smiles")):
            text = writer(fmt)
            e = {"inchikey_in": key_of(inc)}
            if text is None:
                e.update({"hash": None, "inchikey_ours": None, "lost": ["write_error"]})
            else:
                ours, err = replay({"import": text, "out": "smiles"})
                o = inchi_of(Chem.MolFromSmiles(ours)) if ours else None
                e.update({"hash": fnv1a(stable(text)), "inchikey_ours": key_of(o), "lost": sorted(lost_between(inc, o)) if o else ["read_error:ours"]})
            entry[f"{prog}-{label}"] = e
            if fmt in ("mol", "sdf") and text and text.startswith("\n"):
                # diagnostic: the same file with a one-character title, to see what else fails once the empty title line is not the reason
                ours, _ = replay({"import": "t" + text, "out": "smiles"})
                o = inchi_of(Chem.MolFromSmiles(ours)) if ours else None
                entry[f"{prog}-{label}+title"] = {"inchikey_in": key_of(inc), "hash": fnv1a(stable("t" + text)), "inchikey_ours": key_of(o), "lost": sorted(lost_between(inc, o)) if o else ["read_error:ours"]}
    return sid, entry


# reactions: a pair of single-component structures of the set drawn with an arrow between them
def reaction_cases(rows):
    singles = [r for r in rows if "." not in r[2] and r[1] not in ("rgroup", "radical")]
    return [(singles[i], singles[i + 1]) for i in range(0, 40, 2)]


def reaction_request(a, b):
    ours, _ = replay({"import": f"{a[2]}.{b[2]}", "out": "mzdraw"})
    if ours is None:
        return None
    doc = json.loads(ours)
    n = Chem.MolFromSmiles(a[2]).GetNumAtoms()
    first = doc["atoms"][:n]
    rest = doc["atoms"][n:]
    # the editor places the pieces by its own rule: the arrow goes between them, the piece on its left is the reactant
    flip = sum(x["x"] for x in first) / len(first) > sum(x["x"] for x in rest) / len(rest)
    if flip:
        first, rest = rest, first
    left = max(x["x"] for x in first)
    right = min(x["x"] for x in rest)
    mid = round((left + right) / 2, 3)
    y = round(sum(x["y"] for x in doc["atoms"]) / len(doc["atoms"]), 3)
    return {"flip": flip, "import": f"{a[2]}.{b[2]}", "log": [{"op": "obj.add", "t": "arrow", "x": round(mid - 0.8, 3), "x2": round(mid + 0.8, 3), "y": y, "y2": y}], "out": "rxn"}


def reaction(pair, tmp):
    a, b = pair
    req = reaction_request(a, b)
    if req is None:
        return f"{a[0]}+{b[0]}", None
    flip = req.pop("flip")
    text, err = replay(req)
    e = {"request": req, "hash": fnv1a(text) if text else None}
    want = [key_of(inchi_of(Chem.MolFromSmiles(x[2]))) for x in ((b, a) if flip else (a, b))]
    e["inchikey_in"] = want
    got = None
    try:
        rxn = rdChemReactions.ReactionFromRxnBlock(text)
        sides = []
        for side in (rxn.GetReactants(), rxn.GetProducts()):
            ks = []
            for x in side:
                x = Chem.Mol(x)
                Chem.SanitizeMol(x)
                ks.append(key_of(inchi_of(x)))
            sides.append(sorted(ks))
        got = sides
    except Exception:
        pass
    e["inchikey_out_rdkit"] = got
    ok = got is not None and got == [[want[0]], [want[1]]]
    # Open Babel: reaction SMILES, then the InChI of each component by Open Babel
    ob = None
    path = tmp_file(tmp, ".rxn", text or "")
    r = subprocess.run([OBABEL, "-irxn", path, "-osmi"], capture_output=True, text=True)
    os.unlink(path)
    line = r.stdout.strip().split()[0] if r.stdout.strip() else ""
    if ">>" in line:
        lhs, rhs = line.split(">>")
        ob = [[key_of(obabel_inchi("smi", s, tmp)) for s in side.split(".")] for side in (lhs, rhs)]
    e["inchikey_out_obabel"] = ob
    e["lost"] = [] if ok and ob == [[want[0]], [want[1]]] else ["roles_or_structure"]
    return f"{a[0]}+{b[0]}", e


def wilson(k, n, z=1.96):
    if n == 0:
        return [0.0, 0.0]
    p = k / n
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return [round(c - h, 4), round(c + h, 4)]


def summarise(fwd, rev, rxn):
    s = {"forward": {}, "reverse": {}, "reactions": {}}
    for name, _, obfmt in FORMATS:
        for prog in ("rdkit", "obabel"):
            ok = tot = 0
            for e in fwd.values():
                x = e[name]
                got, ink = x.get(f"inchikey_out_{prog}"), x["inchikey_in"]
                if prog == "rdkit" and name not in RDKIT_READS or prog == "obabel" and not obfmt:
                    continue
                if prog == "obabel" and ink and got is None and e["category"] == "rgroup":
                    continue  # Open Babel writes no InChI of a dummy atom: not comparable
                if not ink:
                    continue
                tot += 1
                ok += got == ink
            if tot:
                s["forward"][f"{name}/{prog}"] = {"ok": ok, "of": tot, "rate": round(ok / tot, 4), "wilson95": wilson(ok, tot)}
    kinds = sorted({k for e in rev.values() for k in e})
    for k in kinds:
        ok = tot = 0
        for e in rev.values():
            x = e[k]
            if not x["inchikey_in"]:
                continue
            tot += 1
            ok += x["inchikey_ours"] == x["inchikey_in"]
        s["reverse"][k] = {"ok": ok, "of": tot, "rate": round(ok / tot, 4), "wilson95": wilson(ok, tot)}
    for prog in ("rdkit", "obabel"):
        ok = sum(1 for e in rxn.values() if e.get(f"inchikey_out_{prog}") == [[e["inchikey_in"][0]], [e["inchikey_in"][1]]])
        s["reactions"][prog] = {"ok": ok, "of": len(rxn)}
    return s


# ---------------------------------------------------------------- the optional public corpus (E19)
CORPUS_CIDS = [2244, 2519, 3672, 702, 887, 962, 5793, 6322, 1983, 5090, 2662, 3365, 3345, 5288826, 5280343, 5281, 5957, 5742, 2723, 1140]


def corpus(tmp):
    """Run-time fetch of a fixed set of PubChem compound ids (the ids are listed here, the structures are never stored); prints rates only."""
    import urllib.request

    rows = []
    for cid in CORPUS_CIDS:
        url = f"https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/cid/{cid}/property/IsomericSMILES/TXT"
        try:
            with urllib.request.urlopen(url, timeout=30) as r:
                rows.append((f"c{cid}", "corpus", r.read().decode().strip().split()[0]))
        except Exception as ex:
            print(f"corpus: cannot fetch {url}: {ex}", file=sys.stderr)
    if not rows:
        return {"source": "PubChem PUG REST", "status": "not measured (no id could be fetched)"}
    fwd = dict(ThreadPoolExecutor(8).map(lambda r: forward(r, tmp), rows))
    s = summarise(fwd, {}, {})
    return {"source": "PubChem PUG REST", "ids": CORPUS_CIDS, "fetched": len(rows), "date": today(), "forward": s["forward"]}


def today():
    return os.environ.get("FIDELITY_DATE") or datetime.date.today().isoformat()


def main():
    global REPLAY
    ap = argparse.ArgumentParser()
    ap.add_argument("--replay", required=True, help="the native replay example (cargo build -p mzdraw --example replay)")
    ap.add_argument("--corpus", action="store_true", help="optional: the public subset, rates only, nothing committed")
    a = ap.parse_args()
    REPLAY = os.path.abspath(a.replay)
    if not os.path.exists(REPLAY):
        die(f"{REPLAY} does not exist")
    with tempfile.TemporaryDirectory() as tmp:
        if a.corpus:
            print(json.dumps(corpus(tmp), indent=1))
            return
        rows = read_set()
        if len(rows) != 200:
            die(f"the set has {len(rows)} structures, not 200")
        with ThreadPoolExecutor(8) as ex:
            fwd = dict(ex.map(lambda r: forward(r, tmp), rows))
            rev = dict(ex.map(lambda r: reverse(r, tmp), rows))
            rxn = dict(ex.map(lambda p: reaction(p, tmp), reaction_cases(rows)))
        rxn = {k: v for k, v in rxn.items() if v}
        out = {
            "format": "fidelity/1",
            "date": today(),
            "programs": {
                "rdkit": rdkit.__version__,
                "rdkit_inchi": inchi.GetInchiVersion(),
                "openbabel": version([OBABEL, "-V"]),
                "inchi-1": version([INCHI1, "-?"]) if INCHI1 else None,
            },
            "hash": "fnv1a-64 of the text our writer gives (a change of one byte changes it)",
            "layers": "InChI layers that differ in `lost`: f formula, c connectivity, h hydrogens, q/p charge, b/t/m/s stereo, i/r isotopes",
            "summary": summarise(fwd, rev, rxn),
        }
        sys.stdout.write("{\n")
        for k, v in out.items():
            sys.stdout.write(f' "{k}": {json.dumps(v, separators=(",", ":"))},\n')
        for sec, data in (("forward", fwd), ("reverse", rev), ("reactions", rxn)):
            sys.stdout.write(f' "{sec}": {{\n')
            items = sorted(data.items())
            for i, (k, v) in enumerate(items):
                sys.stdout.write(f'  "{k}": {json.dumps(v, separators=(",", ":"))}' + ("," if i + 1 < len(items) else "") + "\n")
            sys.stdout.write(" }" + ("," if sec != "reactions" else "") + "\n")
        sys.stdout.write("}\n")


if __name__ == "__main__":
    main()
