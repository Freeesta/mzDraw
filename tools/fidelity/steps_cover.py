#!/usr/bin/env python3
"""Kill-test counter of tp-10: of the (parent, product) pairs of a list, how many can ONE named step at ONE site make? For each pair the
table is tried at every site the editor lists (`out: sites`), each derive is read back as a molfile by RDKit, and the product is compared
as canonical SMILES (no stereo, no atom maps) with the product of the list. A step that cuts a structure in two (hydrolysis) counts when
the product of the list is the whole TP or one of its fragments (the person deletes the other); those are counted apart.
Usage:  python3 tools/fidelity/steps_cover.py --replay target/debug/examples/replay [LIST.tsv]
The list is `parent<TAB>product` lines (`#` starts a comment). Default: tests/data/fidelity/steps-cover.tsv, a stand-in written for this
tool, NOT the list of tp-08. Exit 0 and prints the count; the ratio below 0.5 is the kill test of tp-10 (CLI route only, no page field)."""
import argparse, json, os, subprocess, sys

STEPS = ["hydroxylation", "n-demethylation", "o-demethylation", "n-oxidation", "dehydrogenation", "hydration", "dechlorination",
         "debromination", "hydrolysis", "acetylation", "sulfation", "glucuronidation", "decarboxylation"]


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--replay", required=True); ap.add_argument("list", nargs="?")
    args = ap.parse_args()
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    path = args.list or os.path.join(root, "tests", "data", "fidelity", "steps-cover.tsv")
    try:
        from rdkit import Chem, RDLogger
        RDLogger.DisableLog("rdApp.*")
    except ImportError:
        print("rdkit is not installed (pip install rdkit)", file=sys.stderr); return 3

    def ask(req):
        r = subprocess.run([args.replay], input=json.dumps(req), capture_output=True, text=True)
        return r.stdout if r.returncode == 0 else None

    def canon(m):
        for a in m.GetAtoms():
            a.SetAtomMapNum(0)
        return Chem.MolToSmiles(m, isomericSmiles=False)

    whole_hits = frag_hits = rows = 0
    for line in open(path):
        if not line.strip() or line.startswith("#"):
            continue
        parent, product = line.rstrip("\n").split("\t")
        rows += 1
        want = canon(Chem.MolFromSmiles(product))
        found = None
        for step in STEPS:
            sites = ask({"import": parent, "out": "sites", "args": {"of": 1, "step": step}})
            for site in json.loads(sites)["ids"] if sites else []:
                mol = ask({"import": parent, "commands": [{"command": "tp:derive", "args": {"of": 1, "step": step, "site": site}}], "out": "molfile"})
                whole = Chem.MolFromMolBlock(mol, sanitize=True) if mol else None
                if whole is None:
                    continue
                n = Chem.MolFromSmiles(parent).GetNumAtoms()
                frags = [(m, f) for m, f in zip(Chem.GetMolFrags(whole), Chem.GetMolFrags(whole, asMols=True)) if min(m) >= n]
                parts = [canon(f) for _, f in frags]
                if ".".join(sorted(parts)) == want:
                    found = ("whole", step, site)
                elif want in parts and found is None:
                    found = ("fragment", step, site)
                if found and found[0] == "whole":
                    break
            if found and found[0] == "whole":
                break
        if found:
            whole_hits += found[0] == "whole"; frag_hits += found[0] == "fragment"
        print("%-28s -> %-28s %s" % (parent, product, "%s by %s at %d" % found if found else "no single step"))
    print("steps_cover: %d of %d products by one step at one site (%d as a whole, %d as a fragment of the TP), ratio %.2f"
          % (whole_hits + frag_hits, rows, whole_hits, frag_hits, (whole_hits + frag_hits) / rows if rows else 0))
    return 0


if __name__ == "__main__":
    sys.exit(main())
