#!/usr/bin/env python3
"""Independent check of atom-to-atom maps (plan-11): RDKit reads what mzDraw writes.

For 20 synthetic reactions (mapped reaction SMILES written here), each side is read by mzDraw (native replay example), the
reaction is written as an RXN V2000 file and as a SMILES, and RDKit (`AllChem.ReactionFromRxnBlock`, `GetAtomMapNum`) reads
those files. The map of every atom must be the one the input says, read by RDKit from the input SMILES. The committed
reference `maps-reference.json` is RDKit's reading of our files (program, version, date inside).

RDKit is fetched at run time, never committed: `pip install rdkit` (a virtual environment is fine). If it cannot be installed,
the check is "not measured" (exit code 3); this script stays runnable.

usage: cargo build -p mzdraw --example replay && python3 tools/fidelity/maps.py [--update]
"""
import json, os, subprocess, sys, datetime

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
REPLAY = os.path.join(ROOT, "target", "debug", "examples", "replay")
REFERENCE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "maps-reference.json")

# synthetic reactions: reactants >> products, maps on the atoms that are followed
REACTIONS = [
    "[CH3:1][C:2](=[O:3])[OH:4].[CH3:5][OH:6]>>[CH3:1][C:2](=[O:3])[O:6][CH3:5].[OH2:4]",
    "[CH3:1][C:2](=[O:3])[OH:4].[NH2:5][CH3:6]>>[CH3:1][C:2](=[O:3])[NH:5][CH3:6].[OH2:4]",
    "[CH3:1][CH2:2][Br:3].[OH-:4]>>[CH3:1][CH2:2][OH:4].[Br-:3]",
    "[CH2:1]=[CH2:2].[OH2:3]>>[CH3:1][CH2:2][OH:3]",
    "[CH3:1][CH:2]=[O:3].[H][H]>>[CH3:1][CH2:2][OH:3]",
    "[CH3:1][CH2:2][OH:3]>>[CH3:1][CH:2]=[O:3]",
    "[c:1]1[cH:2][cH:3][cH:4][cH:5][cH:6]1.[Cl:7][Cl:8]>>[c:1]1([Cl:7])[cH:2][cH:3][cH:4][cH:5][cH:6]1.[Cl:8]",
    "[CH3:1][C:2]#[N:3].[OH2:4]>>[CH3:1][C:2](=[O:4])[NH2:3]",
    "[CH3:1][NH2:2].[CH3:3][I:4]>>[CH3:1][NH:2][CH3:3].[I:4]",
    "[CH3:1][C:2](=[O:3])[CH3:4].[CH3:5][OH:6]>>[CH3:1][C:2]([OH:3])([CH3:4])[O:6][CH3:5]",
    "[OH:1][CH2:2][CH2:3][OH:4]>>[O:1]1[CH2:2][CH2:3]1.[OH2:4]",
    "[CH3:1][CH2:2][CH2:3][OH:4]>>[CH3:1][CH:2]=[CH2:3].[OH2:4]",
    "[N:1]#[C:2][CH3:3].[Mg:4]>>[N:1]#[C:2][CH3:3]",
    "[CH3:1][S:2][CH3:3]>>[CH3:1][S:2](=[O:4])[CH3:3]",
    "[Cl:1][CH2:2][CH2:3][Cl:4]>>[CH2:2]=[CH2:3].[Cl:1][Cl:4]",
    "[CH3:1][C:2](=[O:3])[O:4][CH3:5].[OH2:6]>>[CH3:1][C:2](=[O:3])[OH:6].[CH3:5][OH:4]",
    "[CH3:1][CH2:2][NH2:3].[CH3:4][C:5](=[O:6])[Cl:7]>>[CH3:1][CH2:2][NH:3][C:5](=[O:6])[CH3:4].[Cl:7]",
    "[CH3:1][CH:2]([OH:3])[CH3:4]>>[CH3:1][C:2](=[O:3])[CH3:4]",
    "[O:1]=[C:2]([OH:3])[CH2:4][CH2:5][CH3:6]>>[O:1]=[C:2]([O-:3])[CH2:4][CH2:5][CH3:6]",
    "[CH3:1][CH2:2][CH2:3][Br:4].[CH3:5][O-:6]>>[CH3:1][CH2:2][CH2:3][O:6][CH3:5].[Br-:4]",
]


def replay(request):
    r = subprocess.run([REPLAY], input=json.dumps(request), capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit("replay failed: " + r.stdout + r.stderr)
    return r.stdout


def our_rxn(side_reactants, side_products):
    """Our reaction file: each molecule goes through the reader and writer of mzDraw, the roles are written by `to_rxn`."""
    mols = lambda smis: [replay({"import": s, "out": "molfile"}) for s in smis]
    reactants, products = mols(side_reactants), mols(side_products)
    block = "$RXN\n\n  synthetic\n\n%3d%3d\n" % (len(reactants), len(products))
    block += "".join("$MOL\n" + m for m in reactants + products)
    # read as a reaction file by mzDraw, written again by mzDraw
    return replay({"import": block, "out": "rxn"})


def main():
    try:
        from rdkit import Chem, rdBase
        from rdkit.Chem import AllChem
    except ImportError:
        print("RDKit is not installed: not measured (pip install rdkit)")
        return 3
    if not os.path.exists(REPLAY):
        print("build the replay example first: cargo build -p mzdraw --example replay")
        return 2
    results, bad = [], 0
    for k, reaction in enumerate(REACTIONS):
        left, right = reaction.split(">>")
        sides = [left.split("."), right.split(".")]
        expected = [[sorted(a.GetAtomMapNum() for a in Chem.MolFromSmiles(s).GetAtoms() if a.GetAtomMapNum()) for s in side] for side in sides]
        rxn_text = our_rxn(*sides)
        rxn = AllChem.ReactionFromRxnBlock(rxn_text)
        got = [[sorted(a.GetAtomMapNum() for a in m.GetAtoms() if a.GetAtomMapNum()) for m in side] for side in (rxn.GetReactants(), rxn.GetProducts())]
        # SMILES out of the whole drawing: the maps as the atom class
        whole = replay({"import": rxn_text, "out": "smiles"})
        smiles_maps = sorted(a.GetAtomMapNum() for a in Chem.MolFromSmiles(whole).GetAtoms() if a.GetAtomMapNum())
        want = sorted(n for side in expected for mol in side for n in mol)
        ok = [sorted(m) for side in got for m in side] == [sorted(m) for side in expected for m in side] and smiles_maps == want
        bad += not ok
        results.append({"reaction": reaction, "rdkit_rxn": got, "rdkit_smiles": smiles_maps, "same_as_input": ok})
        print(("ok   " if ok else "FAIL ") + reaction)
    reference = {
        "program": "RDKit",
        "version": rdBase.rdkitVersion,
        "date": datetime.date.today().isoformat(),
        "success": f"{len(REACTIONS) - bad}/{len(REACTIONS)}",
        "results": results,
    }
    if "--update" in sys.argv:
        with open(REFERENCE, "w") as f:
            json.dump(reference, f, indent=1)
            f.write("\n")
    elif os.path.exists(REFERENCE):
        old = json.load(open(REFERENCE))
        if old["results"] != results:
            print("the reference differs from this run: " + REFERENCE)
            return 1
    print(f"maps: {len(REACTIONS) - bad}/{len(REACTIONS)} reactions read by RDKit {rdBase.rdkitVersion} with the maps of the input")
    return 1 if bad else 0


sys.exit(main())
