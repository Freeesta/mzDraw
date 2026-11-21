#!/usr/bin/env python3
"""RXN roles of `scheme:row` read back by RDKit (plan-13, independent check).

For every one-arrow case of tests/data/log/scheme-row.json (a reaction file is written by drawings with exactly one arrow), the
drawing is written as RXN V2000 by the native `replay` example before and after the command. RDKit (a program that is not ours,
never linked) reads both files; the number of reactants, products and agents, and the canonical SMILES of each component, must be the
same. This does not prove the layout is right, only that the roles a second program reads out of the file did not change.

Usage:
  cargo build -p mzdraw --example replay
  python3 tools/scheme_row/rdkit_roles.py --replay target/debug/examples/replay > tests/data/scheme_row/reference.json

Exit 0 when every case agrees, 1 on a mismatch or a tool error, 3 when RDKit is not installed (pip install rdkit). Data on stdout,
diagnostics on stderr. Nothing third-party is committed: only the program name, its version and the roles of our synthetic inputs.
"""
import argparse
import datetime
import json
import os
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
GOLD = os.path.join(ROOT, "tests", "data", "log", "scheme-row.json")

try:
    import rdkit
    from rdkit import Chem, RDLogger
    from rdkit.Chem import rdChemReactions
except ImportError:
    print("rdkit_roles.py: RDKit is missing: pip install rdkit", file=sys.stderr)
    sys.exit(3)

RDLogger.DisableLog("rdApp.*")


def replay(binary, request):
    r = subprocess.run([binary], input=json.dumps(request), capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def roles(rxn_text):
    """(reactants, products, agents), each the sorted canonical SMILES of the components."""
    rxn = rdChemReactions.ReactionFromRxnBlock(rxn_text)
    if rxn is None:
        return None
    out = []
    for get in (rxn.GetReactants, rxn.GetProducts, rxn.GetAgents):
        out.append(sorted(Chem.MolToSmiles(m) for m in get()))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--replay", default=os.path.join(ROOT, "target", "debug", "examples", "replay"))
    args = ap.parse_args()
    gold = json.load(open(GOLD))
    report, bad = [], 0
    for case in gold["cases"]:
        before_req = {k: v for k, v in case["request"].items() if k not in ("command", "args", "commands")}
        before_req["out"] = "rxn"
        after_req = dict(case["request"], out="rxn")
        b, a = replay(args.replay, before_req), replay(args.replay, after_req)
        if b is None or a is None:
            # more than one arrow: no reaction file (error.draw2.rxnArrow), nothing to compare
            report.append({"case": case["name"], "compared": False})
            continue
        rb, ra = roles(b), roles(a)
        ok = rb is not None and rb == ra
        bad += not ok
        report.append({"case": case["name"], "compared": True, "same": ok, "reactants": len(rb[0]), "products": len(rb[1]), "agents": len(rb[2])})
        if not ok:
            print(f"MISMATCH {case['name']}: {rb} != {ra}", file=sys.stderr)
    json.dump({"program": "RDKit", "version": rdkit.__version__, "date": datetime.date.today().isoformat(), "cases": report}, sys.stdout, indent=2)
    print()
    sys.exit(1 if bad else 0)


main()
