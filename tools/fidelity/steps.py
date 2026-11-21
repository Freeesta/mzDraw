#!/usr/bin/env python3
"""Independent check of the named steps (tp-10). For each row of the table `steps` and each synthetic parent below, RDKit applies ONE
reaction SMARTS (written here, in the tool only: the editor has no SMARTS, rule 13) and compares with OUR output, which RDKit reads as a
molfile:
  1. the sites: for every atom (atom steps) or bond (bond steps) of the parent, the site is marked in the parent (isotope 253 on the
     atom, or on both ends of the bond) and the SMARTS, which asks for that mark on the site atoms and for the conditions of the row,
     is run; the ids where it gives a product must be the ids `out: sites` lists (ascending, the same set);
  2. the products: at every site, the product RDKit makes (sanitised, the mark removed, canonical SMILES without stereo) must equal the
     canonical SMILES of the structures our `tp:derive` makes (the TP, all its fragments) when RDKit reads the molfile of our drawing.
What it does NOT show: that the sites are the ones a chemist would pick (no ranking exists), anything about coordinates, the arrow,
the maps, stereo-chemistry (the parents have none) or the 2-D drawing of a new atom; a tie of hydration (both ends with the same
number of hydrogens, the end is chosen by id) is only compared where the two products are one structure (the parents have none that
differs); the hydrolysis SMARTS is compared on open chains (a lactone gives one product that both product templates would claim).
The SMARTS describe the rows of the table; the rows were written first and the SMARTS after, from the names of the steps.
Usage:  cargo build -p mzdraw --example replay
        python3 tools/fidelity/steps.py --replay target/debug/examples/replay --out tests/data/fidelity/steps.json
Exit 0 all agree, 1 a disagreement, 2 tool error, 3 RDKit missing. RDKit is installed at run time (pip install rdkit), never committed;
the committed reference has the program name and version, the date and the success rate."""
import argparse, datetime, json, subprocess, sys

MARK = 253
AM = "[253N,253O;!H0;+0:1]"   # an N-H or an O-H, marked

# name -> (kind, [(reaction SMARTS)]). The marked atoms are the site (atom steps) or both ends of the site bond (bond steps).
def hydration_templates():
    out = []
    for lo in range(0, 4):
        for hi in range(lo + 1, 4):
            hs = lambda n: "H0" if n == 0 else "H%d" % n
            out.append("[253C;%s:1]=[253C;%s:2]>>[C:1](O)-[C:2]" % (hs(lo), hs(hi)))
    # equal counts: a tie is by id; only a symmetric alkene is compared, where both ends give one product
    for n in range(0, 3):
        h = "H0" if n == 0 else "H%d" % n
        out.append("[253C;%s:1]=[253C;%s:2]>>[C:1](O)-[C:2]" % (h, h))
    return out


RULES = {
    "hydroxylation": ("atom", ["[253#6;!H0:1]>>[*:1]O"]),
    "n-demethylation": ("atom", ["[N,n:1]-[253CH3:2]>>[*:1]"]),
    "o-demethylation": ("atom", ["[O,o:1]-[253CH3:2]>>[*:1]"]),
    "n-oxidation": ("atom", ["[253N;X3;H0;+0;!a:1]>>[N+:1][O-]", "[253n;X2;H0;+0:1]>>[n+:1][O-]"]),
    "dehydrogenation": ("bond", ["[253C;!H0:1]-[253C;!H0:2]>>[C:1]=[C:2]"]),
    "hydration": ("bond", hydration_templates()),
    "dechlorination": ("atom", ["[253Cl;X1;+0]-[*:1]>>[*:1]"]),
    "debromination": ("atom", ["[253Br;X1;+0]-[*:1]>>[*:1]"]),
    "hydrolysis": ("bond", ["[253C;+0:1](=[O;D1;+0:3])-[253O,253N;!D1:2]>>[C:1](=[O:3])O.[*:2]"]),
    "acetylation": ("atom", [AM + ">>[*:1]C(=O)C"]),
    "sulfation": ("atom", ["[253O;!H0;+0:1]>>[O:1]S(=O)(=O)O"]),
    "glucuronidation": ("atom", [AM + ">>[*:1][CH]1[CH](O)[CH](O)[CH](O)[CH](C(=O)O)O1"]),
    "decarboxylation": ("atom", ["[*:3]-[253C;+0;D3:1](=[O;D1])-[OH;D1]>>[*:3]"]),
}

PARENTS = [
    "CN(C)CC(=O)OCc1ccncc1Cl",
    "COc1ccc(CCBr)cc1C(=O)NC",
    "CCC(=O)O",
    "OC(=O)CC(N)C(=O)O",
    "CC=C",
    "C=CC(=O)OC",
    "CCCC(O)CC=O",
    "Cn1cccc1C(C)(C)C",
    "c1ccncc1CNC",
    "CC(=O)N(C)c1ccccc1",
    "CC(C)(C)NC(=O)C",
    "C=C",
]


def ask(replay, req):
    r = subprocess.run([replay], input=json.dumps(req), capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError("replay failed: %s %s" % (r.stderr, r.stdout[:200]))
    return r.stdout


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--replay", required=True); ap.add_argument("--out"); args = ap.parse_args()
    try:
        from rdkit import Chem, rdBase
        from rdkit.Chem import AllChem
        from rdkit import RDLogger
        RDLogger.DisableLog('rdApp.*')     # the warnings about the deleted mapped atoms of the templates are expected
    except ImportError:
        print("rdkit is not installed (pip install rdkit)", file=sys.stderr); return 3

    def smiles(m):
        return Chem.MolToSmiles(m, isomericSmiles=False)

    def clean(m):
        for a in m.GetAtoms():
            a.SetIsotope(0)
        try:
            Chem.SanitizeMol(Chem.Mol(m))      # on a copy: a failed sanitisation leaves its molecule half changed
            Chem.SanitizeMol(m)
        except Exception:
            # RDKit does not give a pyrrole-type N its hydrogen back when a substituent is removed: the aromatic N with two ring
            # neighbours and no hydrogen gets one, as the chemistry of the step says, and the product is sanitised again
            m = Chem.RWMol(m)
            for a in m.GetAtoms():
                if a.GetIsAromatic() and a.GetSymbol() == "N" and a.GetDegree() == 2 and a.GetFormalCharge() == 0:
                    a.SetNumExplicitHs(1)
            Chem.SanitizeMol(m)
        return m

    rows, bad, compared = [], 0, 0
    for step, (kind, templates) in RULES.items():
        rxns = [AllChem.ReactionFromSmarts(t) for t in templates]
        for smi in PARENTS:
            parent_doc = json.loads(ask(args.replay, {"import": smi, "out": "mzdraw"}))
            ours = json.loads(ask(args.replay, {"import": smi, "out": "sites", "args": {"of": 1, "step": step}}))["ids"]
            # the molfile of the parent: atom k is the atom with id k+1 (the import numbers the atoms from 1), bond k the k-th bond
            parent = Chem.MolFromMolBlock(ask(args.replay, {"import": smi, "out": "molfile"}), sanitize=True)
            if parent is None:
                raise RuntimeError("RDKit cannot read the parent " + smi)
            atom_ids = [a["id"] for a in parent_doc["atoms"]]
            bond_ids = [b["id"] for b in parent_doc["bonds"]]
            candidates = ([("atom", i, atom_ids[i]) for i in range(parent.GetNumAtoms())] if kind == "atom"
                          else [("bond", b.GetIdx(), bond_ids[b.GetIdx()]) for b in parent.GetBonds()])
            theirs, products = [], {}
            for _, idx, site_id in candidates:
                marked = Chem.RWMol(parent)
                if kind == "atom":
                    marked.GetAtomWithIdx(idx).SetIsotope(MARK)
                else:
                    b = marked.GetBondWithIdx(idx)
                    marked.GetAtomWithIdx(b.GetBeginAtomIdx()).SetIsotope(MARK)
                    marked.GetAtomWithIdx(b.GetEndAtomIdx()).SetIsotope(MARK)
                found = set()
                for rxn in rxns:
                    for tup in rxn.RunReactants((marked.GetMol(),)):
                        try:
                            found.add(".".join(sorted(smiles(clean(p)) for p in tup)) if len(tup) > 1
                                      else smiles(clean(tup[0])))
                        except Exception as e:     # an invalid product is no product
                            pass
                if found:
                    theirs.append(site_id)
                    products[site_id] = found
            row = {"step": step, "parent": smi, "ours": ours, "rdkit": theirs, "sites_agree": ours == sorted(theirs)}
            bad += 0 if row["sites_agree"] else 1
            row["products"] = []
            for site_id in ours:
                if site_id not in products:
                    continue
                req = {"import": smi, "commands": [{"command": "tp:derive", "args": {"of": 1, "step": step, "site": site_id}}], "out": "molfile"}
                whole = Chem.MolFromMolBlock(ask(args.replay, req), sanitize=True)
                for a in whole.GetAtoms():
                    a.SetAtomMapNum(0)       # our atom maps are our own numbering: not part of the structure compared
                frags = Chem.GetMolFrags(whole, asMols=True, sanitizeFrags=True)
                n_parent = parent.GetNumAtoms()
                members = Chem.GetMolFrags(whole)
                tp = sorted(smiles(f) for f, m in zip(frags, members) if min(m) >= n_parent)
                got = ".".join(tp)
                # RDKit may give several products at one site (a tie, a symmetric pair): ours must be one of them
                ok = got in products[site_id] or (len(products[site_id]) > 1 and any(got == p for p in products[site_id]))
                compared += 1
                bad += 0 if ok else 1
                row["products"].append({"site": site_id, "ours": got, "rdkit": sorted(products[site_id]), "agree": ok})
            rows.append(row)
    nrows = sum(1 for r in rows for _ in r["products"])
    agree = sum(1 for r in rows for p in r["products"] if p["agree"])
    sites_ok = sum(1 for r in rows if r["sites_agree"])
    out = {"format": "fidelity-steps/1", "date": datetime.date.today().isoformat(), "programs": {"rdkit": rdBase.rdkitVersion},
           "success": {"site_lists": "%d/%d" % (sites_ok, len(rows)), "products": "%d/%d" % (agree, nrows)}, "results": rows}
    text = json.dumps(out, indent=1, ensure_ascii=False) + "\n"
    if args.out:
        open(args.out, "w").write(text)
    print("steps: site lists %d/%d agree, products %d/%d agree (RDKit %s)" % (sites_ok, len(rows), agree, nrows, rdBase.rdkitVersion))
    for r in rows:
        if not r["sites_agree"]:
            print("  sites differ: %s on %s ours %s rdkit %s" % (r["step"], r["parent"], r["ours"], sorted(r["rdkit"])), file=sys.stderr)
        for p in r["products"]:
            if not p["agree"]:
                print("  product differs: %s on %s at %s ours %s rdkit %s" % (r["step"], r["parent"], p["site"], p["ours"], p["rdkit"]), file=sys.stderr)
    if nrows < 20:
        print("only %d products compared" % nrows, file=sys.stderr); return 1
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
