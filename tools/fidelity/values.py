#!/usr/bin/env python3
"""Independent check of the calculated values (arch-web-17). It reads OUR output and recomputes nothing of ours: for each synthetic SMILES it
asks the glue replay (`cargo build -p mzdraw-wasm --example replay_glue`) for `{"import": SMILES, "values": true, "set": [adducts]}`, i.e. the
`values/1` document that the page shows, and compares it with RDKit on the same SMILES:
  - the Hill formula and the element counts of each structure (RDKit CalcMolFormula on each fragment, the multiset of the fragments);
  - the monoisotopic mass of the atoms (RDKit Descriptors.ExactMolWt, which for an ion already has the electrons taken off: we add them back
    to compare with our `mass`, the sum of the atoms), tolerance 6e-5 (we print 4 decimals, so a half unit of the 4th decimal is
    5e-5 and the engine rounds to 9 decimals first);
  - the m/z of [M+H]+, [M+Na]+, [M+K]+, [M+NH4]+, [M-H]-, [M+Cl]-, [M+HCOO]- of every NEUTRAL structure: RDKit ExactMolWt plus the mass of the
    adduct species built from RDKit's own isotope masses, minus (or plus) the electron mass (CODATA 0.000548579909 u, written here);
  - the m/z of a charged structure itself, our `ionMz`: RDKit's ExactMolWt / |q| (it already holds the electron correction);
  - the average mass (Descriptors.MolWt), tolerance 0.01: RDKit uses the IUPAC weights to 3 or 4 decimals (Cl 35.453), `sel` the conventional
    weights of three decimals (Cl 35.45), so the two differ by up to a few thousandths on a structure with several halogens.
Limits, said plainly: the engine `sel` reads the structure with the graph reader of mzLab, which refuses an element without a valence rule (a
metal ion such as Na+, an explicit hydrogen atom) and a structure of more than 200 atoms: we then give the key and no number (listed as
"limit" below, not counted as an agreement). RDKit and mzLab take the same isotope masses (AME), so the masses are not independent of the data source, only of the code
that sums them, counts hydrogens and applies the adduct shifts. A structure with an R group has no number (not tested here); isotopic patterns
and fragmentation are out of scope. Selections (a part of a structure with its cut bonds closed with hydrogen) are checked against RDKit on the
fragment SMILES with the hydrogens written.
Usage: python3 tools/fidelity/values.py [path/to/replay_glue]. Without RDKit or the example it prints `skipped` on stderr and exits 0; it
exits 1 on any disagreement or when fewer than 18 structures were compared (0 of 0 is not a pass)."""
import json, os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
REPLAY = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "target", "debug", "examples", "replay_glue")
try:
    from rdkit import Chem, __version__ as RDV
    from rdkit.Chem import Descriptors, rdMolDescriptors
except ImportError:
    print("skipped: rdkit not installed (pip install rdkit)", file=sys.stderr); sys.exit(0)
if not os.path.exists(REPLAY):
    print("skipped: no replay_glue example at " + REPLAY, file=sys.stderr); sys.exit(0)

ELECTRON = 0.000548579909
PT = Chem.GetPeriodicTable()


def iso(sym):
    return PT.GetMostCommonIsotopeMass(sym)


# the adduct species, from RDKit's isotope masses: added neutral part, and the sign of the charge
ADDUCTS = {
    "[M+H]+": (iso("H"), +1),
    "[M+NH4]+": (iso("N") + 4 * iso("H"), +1),
    "[M+Na]+": (iso("Na"), +1),
    "[M+K]+": (iso("K"), +1),
    "[M-H]-": (-iso("H"), -1),
    "[M+Cl]-": (iso("Cl"), -1),
    "[M+HCOO]-": (iso("C") + iso("H") + 2 * iso("O"), -1),
}

# synthetic structures, written for this check (neutral, charged, salts, isotopes, a zwitterion, halogens, sulfur, phosphorus)
SMILES = [
    "Cn1cnc2c1c(=O)n(C)c(=O)n2C",       # caffeine
    "OCC(O)C(O)C(O)C(O)C=O",            # glucose, open chain
    "C[NH3+].[Cl-]",                    # methylammonium chloride
    "C[N+](C)(C)C",                     # tetramethylammonium
    "CC(=O)[O-].[Na+]",                 # sodium acetate
    "CC(=O)Oc1ccccc1C(=O)O",            # aspirin
    "[13CH3]O",                         # labelled methanol
    "NCC(=O)O",                         # glycine
    "[NH3+]CC(=O)[O-]",                 # glycine zwitterion
    "c1ccncc1",                         # pyridine
    "CCN(CC)CC",                        # triethylamine
    "Clc1ccc(Br)cc1",                   # a chlorine and a bromine
    "CSC",                              # dimethyl sulfide
    "CP(=O)(O)O",                       # methylphosphonic acid
    "OC(=O)C(F)(F)F",                   # trifluoroacetic acid
    "C[15NH3+]",                        # a labelled ammonium
    "C#N",                              # hydrogen cyanide
    "[O-]S(=O)(=O)[O-].[Na+].[Na+]",    # sodium sulfate (two identical ions)
    "CC(C)(C)OC(=O)N",                  # a carbamate
    "c1ccc2ccccc2c1",                   # naphthalene
]


def ask(request):
    r = subprocess.run([REPLAY], input=json.dumps(request), capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit("replay_glue failed: " + r.stderr)
    return json.loads(r.stdout)


def ours_body(formula, charge):
    """Our Hill formula without its charge (`O4S2\u2212`: the count of the charge follows the element counts, then the sign)."""
    if charge == 0:
        return formula
    return formula[: -(len(str(abs(charge))) if abs(charge) > 1 else 0) - 1]


def rdkit_body(formula):
    """RDKit's formula without its charge (`O4S-2`, `CH6N+`)."""
    return re.sub(r"[+-]\d*$", "", formula)


compared, bad, limits = 0, [], []
names = list(ADDUCTS)
for smi in SMILES:
    ours = ask({"import": smi, "values": True, "set": ["display.adducts=" + ",".join(names)]})
    assert ours["engine"] and ours["error"] is None, ours
    mol = Chem.MolFromSmiles(smi)
    frags = [Chem.AddHs(f) for f in Chem.GetMolFrags(mol, asMols=True)]
    theirs = sorted(
        ((rdkit_body(rdMolDescriptors.CalcMolFormula(f)), f) for f in frags), key=lambda t: t[0]
    )
    mine = sorted(ours["structures"], key=lambda s: ours_body(s["formula"], s["charge"]))
    if len(mine) != len(theirs):
        bad.append(f"{smi}: {len(mine)} structures, RDKit {len(theirs)}")
        continue
    for s, (rd_formula, f) in zip(mine, theirs):
        if s["error"]:
            # the engine's own limit, said by a key and no number: not counted as an agreement, not hidden either
            limits.append(f"{smi} ({s['formula']}): {s['error']}")
            if any(s[k] is not None for k in ("mass", "average", "ionMz")) or s["mz"]:
                bad.append(f"{smi}: a number next to the error {s['error']}")
            continue
        compared += 1
        # RDKit writes isotopes as [13C] inside the formula only through CalcMolFormula(separateIsotopes); compare the element counts
        q = Chem.GetFormalCharge(f)
        # RDKit's ExactMolWt of an ion already takes the electrons off (mass of the atoms minus q electrons): that is OUR ionMz for |q| = 1;
        # `mass` of ours is the sum of the atoms, so it is compared with ExactMolWt + q electrons
        rd_exact = Descriptors.ExactMolWt(f)
        mono = rd_exact + q * ELECTRON
        problems = []
        if ours_body(s["formula"], s["charge"]) != rd_formula:
            problems.append(f"formula {s['formula']} vs {rd_formula}")
        if s["charge"] != q:
            problems.append(f"charge {s['charge']} vs {q}")
        if abs(float(s["mass"]) - mono) > 6e-5:
            problems.append(f"mass {s['mass']} vs {mono:.5f}")
        if abs(float(s["average"]) - Descriptors.MolWt(f)) > 0.01:
            problems.append(f"average {s['average']} vs {Descriptors.MolWt(f):.4f}")
        if q == 0:
            if s["ionMz"] is not None:
                problems.append("a neutral structure has an ionMz")
            for name, (add, sign) in ADDUCTS.items():
                want = mono + add - sign * ELECTRON
                if s["mz"].get(name) is None or abs(float(s["mz"][name]) - want) > 6e-5:
                    problems.append(f"{name} {s['mz'].get(name)} vs {want:.5f}")
        else:
            want = rd_exact / abs(q)
            if s["ionMz"] is None or abs(float(s["ionMz"]) - want) > 6e-5:
                problems.append(f"ionMz {s['ionMz']} vs {want:.5f}")
            if any(v is not None for v in s["mz"].values()):
                problems.append("a charged structure has an adduct m/z")
        if problems:
            bad.append(f"{smi} ({s['formula']}): " + "; ".join(problems))

# selections: the first atom of an ethanol-like chain with its cut bond closed with hydrogen is the molecule CH4, and so on
SELECTIONS = [("CCO", [1], "C"), ("CCO", [1, 2], "CC"), ("CCO", [2, 3], "CO"), ("CC(=O)O", [2, 3], "C=O")]
for smi, atoms, frag in SELECTIONS:
    got = ask({"import": smi, "values": {"atoms": atoms}})["selection"]
    f = Chem.AddHs(Chem.MolFromSmiles(frag))
    compared += 1
    if got is None or abs(float(got["mass"]) - Descriptors.ExactMolWt(f)) > 6e-5:
        bad.append(f"selection {atoms} of {smi}: {got and got['mass']} vs {Descriptors.ExactMolWt(f):.5f} (RDKit on {frag} with hydrogens)")

for line in limits:
    print("limit of the engine (no number, keyed error):", line)
print(f"RDKit {RDV}: {compared - len(bad)} of {compared} structures and selections agree" if not bad else "\n".join(bad))
if bad or compared < 18:
    sys.exit(1)
