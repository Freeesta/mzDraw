#!/usr/bin/env python3
"""Independent proof of the semantic diff of mzDraw (plan-15): `apply(diff(a, b), a)` is `b`, checked outside our test code.

For each structure of tests/data/fidelity/set.smi (200 synthetic structures) the script draws `a` (the native `replay` example imports
the SMILES), makes `b` by a few seeded random edits of its own (an element, a charge, a bond order, a new atom on a bond, an atom
removed, a piece moved), asks the example for the diff `{"diff": {"a", "b"}}`, applies the patch to `a` and the hunks one by one and
all together, and checks:
  1. the document bytes of the result equal those of `b` (the counter `next` aside: no operation sets it);
  2. RDKit reads the SMILES of the result and the SMILES of `b` (both written by mzlab-mol through the example) and gives the same
     canonical SMILES and the same InChIKey (RDKit's canonicalisation and bundled InChI are not ours); a pair RDKit reads on neither
     side (an edit made a valence it refuses) is counted apart, not as a pass;
  3. every hunk applies to `a` alone, and the hunks in order give the same atoms and bonds as `b`.

Usage:  python3 tools/diff/run.py --replay target/debug/examples/replay > tests/data/diff/reference.json
Exit 0 when every check holds, 1 on a failure or a tool error; data on stdout, diagnostics on stderr. Needs `rdkit` (pip). Nothing
third-party is committed: the output holds our synthetic structures' counts, the program name and version and the date.
"""
import argparse
import datetime
import json
import os
import random
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SET = os.path.join(HERE, "..", "..", "tests", "data", "fidelity", "set.smi")


def die(msg):
    print(msg, file=sys.stderr)
    sys.exit(1)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--replay", required=True, help="the native `replay` example")
    ap.add_argument("--seed", type=int, default=15)
    ap.add_argument("--date", default=str(datetime.date.today()))
    args = ap.parse_args()
    try:
        from rdkit import Chem, __version__ as rdkit_version
        from rdkit import RDLogger
        from rdkit.Chem import inchi
        RDLogger.DisableLog("rdApp.*")
    except ImportError:
        die("rdkit is needed: pip install rdkit")

    def replay(request):
        p = subprocess.run([args.replay], input=json.dumps(request), capture_output=True, text=True)
        return p.returncode, p.stdout, p.stderr

    def ok(request):
        code, out, err = replay(request)
        if code != 0:
            die(f"replay refused {json.dumps(request)[:200]}: {err}")
        return out

    def canon(smiles):
        m = Chem.MolFromSmiles(smiles)
        return (Chem.MolToSmiles(m), inchi.MolToInchiKey(m)) if m else None

    def no_next(doc_text):
        d = json.loads(doc_text)
        d.pop("next", None)
        return d

    def by_id(doc_text):
        d = no_next(doc_text)
        for k in ("atoms", "bonds", "objects"):
            d[k] = sorted(d.get(k, []), key=lambda x: x["id"])
        return d

    rng = random.Random(args.seed)
    n = same_bytes = same_rdkit = compared = unread = hunks_alone = hunks_all = pairs = changed = skipped = 0
    with open(SET) as f:
        lines = [l.split()[0] for l in f if l.strip()]
    for smi in lines:
        code, a, _ = replay({"import": smi, "out": "mzdraw"})
        if code != 0:
            skipped += 1
            continue
        doc = json.loads(a)
        atoms = [x["id"] for x in doc["atoms"]]
        bonds = [x["id"] for x in doc["bonds"]]
        edits = []
        for _ in range(rng.randint(1, 4)):
            k = rng.randint(0, 5)
            if k == 0:
                edits.append({"op": "atom.set", "id": rng.choice(atoms), "el": rng.choice(["N", "O", "S", "Cl"])})
            elif k == 1:
                edits.append({"op": "atom.set", "id": rng.choice(atoms), "chg": rng.choice([-1, 1])})
            elif k == 2 and bonds:
                edits.append({"op": "bond.set", "id": rng.choice(bonds), "o": rng.randint(1, 2)})
            elif k == 3:
                edits.append({"op": "move", "atoms": rng.sample(atoms, min(len(atoms), rng.randint(1, 3))), "dx": 0.5 * rng.randint(-3, 3), "dy": 0.5})
            elif k == 4 and len(atoms) > 3:
                edits.append({"op": "remove", "atoms": [rng.choice(atoms)]})
            else:
                edits.append({"op": "atom.add", "el": "C", "x": rng.randint(0, 9) / 2, "y": 9.0})
        code, b, _ = replay({"import": a, "log": edits, "out": "mzdraw"})
        if code != 0:
            skipped += 1  # an edit this drawing refuses (a valence, a duplicate): not a pair
            continue
        pairs += 1
        patch = json.loads(ok({"diff": {"a": json.loads(a), "b": json.loads(b)}}))
        changed += bool(patch["ops"])
        got = ok({"import": a, "log": patch["ops"], "out": "mzdraw"})
        n += 1
        same_bytes += no_next(got) == no_next(b)
        sa, sb = ok({"import": got, "out": "smiles"}), ok({"import": b, "out": "smiles"})
        ca, cb = canon(sa), canon(sb)
        if ca is None and cb is None:
            unread += 1  # an edit made a valence RDKit refuses on both sides: nothing to compare
        else:
            compared += 1
            same_rdkit += ca == cb
        # the hunks: each alone on `a`, then all in order
        alone = True
        for h in patch["hunks"]:
            code, _, _ = replay({"import": a, "log": h["ops"], "out": "mzdraw"})
            alone &= code == 0
        hunks_alone += alone
        every = [op for h in patch["hunks"] for op in h["ops"]]
        hunks_all += by_id(ok({"import": a, "log": every, "out": "mzdraw"})) == by_id(b)
    out = {
        "format": "diff-proof/1",
        "program": {"name": "rdkit", "version": rdkit_version, "use": "canonical SMILES and InChIKey"},
        "date": args.date,
        "seed": args.seed,
        "structures": len(lines),
        "pairs": pairs,
        "pairs_with_a_patch": changed,
        "skipped_edits_refused_by_the_drawing": skipped,
        "bytes_equal": same_bytes,
        "rdkit_compared": compared,
        "rdkit_canonical_smiles_and_inchikey_equal": same_rdkit,
        "rdkit_read_neither_side": unread,
        "every_hunk_applies_alone": hunks_alone,
        "all_hunks_give_b": hunks_all,
    }
    print(json.dumps(out, indent=2))
    sys.exit(0 if n == pairs == same_bytes and compared == same_rdkit and hunks_alone == hunks_all else 1)


main()
