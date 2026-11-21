#!/usr/bin/env python3
"""Writes tests/data/fidelity/set.smi: 200 synthetic structures (hand-written lists, filled up with a fixed seed).

Run: python3 tools/fidelity/gen.py > tests/data/fidelity/set.smi
Every line is `<smiles> <id>-<category>`. Only the standard library is used; the output is deterministic.
"""
import random
import sys

SEED = 20261121
TOTAL = 200

# category -> (quota, hand-written structures, parts the seeded filler combines: scaffolds with {} slots, substituents)
SUBS = ["C", "CC", "O", "OC", "N", "F", "Cl", "Br", "C(=O)O", "C#N", "[N+](=O)[O-]", "C(F)(F)F", "C=C", "S", "C(C)C", "OC(C)=O"]

RINGS = [
    "C1CC1", "C1CCC1", "C1CCCC1", "C1CCCCC1", "C1CCCCCC1", "C1CCCCCCC1", "c1ccccc1", "c1ccncc1", "c1ccoc1", "c1ccsc1",
    "c1cc[nH]c1", "c1c[nH]cn1", "c1cscn1", "C1CCOCC1", "C1CCNCC1", "C1COCCN1", "C1=CCCCC1", "C1=CC=CC1", "O1CCOCC1", "C1CCSC1",
    "c1ncncn1", "c1cnccn1", "C1CO1", "C1CN1", "C1CCOC1",
]
RING_SCAFF = ["c1ccc({})cc1", "c1ccc({})cc1{}", "C1CCC({})CC1", "c1cc({})ncc1", "c1cc({})oc1", "C1CC({})CCN1"]

FUSED = [
    "c1ccc2ccccc2c1", "c1ccc2[nH]ccc2c1", "c1ccc2ncccc2c1", "c1ccc2occc2c1", "c1ncc2nc[nH]c2n1", "c1ccc2cc3ccccc3cc2c1",
    "c1ccc2c(c1)ccc1ccccc12", "C1CCC2CCCCC2C1", "C1CCC2CCCC2C1", "c1ccc2c(c1)CCC2", "C1CC2CCC1C2", "C12CC3CC(C1)CC(C3)C2",
    "C1CCC2(CC1)CCCC2", "C1CC2(C1)CCC2", "c1ccc2c(c1)OCO2", "c1ccc2c(c1)OCCO2", "O=C1c2ccccc2C(=O)c2ccccc12",
    "C1CC2CC1C=C2", "C1CCC2C(C1)CCC1C2CCC2CCCC12", "c1cc2cccc3ccc4cccc1c4c32", "c1csc2ccccc12", "c1ccc2[nH]c3ccccc3c2c1",
    "C1=CC2=CC=CC=C2C=C1", "c1cnc2ccccc2c1", "C1CCC2NCCCC2C1",
]

CHARGES = [
    "C[NH3+]", "CC(=O)[O-]", "C[N+](C)(C)C", "[O-][N+](=O)c1ccccc1", "[NH3+]CC(=O)[O-]", "C[N+]([O-])(C)C", "CS(=O)(=O)[O-]",
    "COP(=O)([O-])[O-]", "c1cc[n+](C)cc1", "[NH4+]", "[OH3+]", "C[S+](C)C", "[O-]C(=O)C(=O)[O-]", "C[NH2+]C", "[CH3-]",
    "c1ccc[nH+]c1", "NC(=[NH2+])N", "C[P+](C)(C)C", "[O-]c1ccccc1", "[Cl-]",
]

ISOTOPES = [
    "[13CH3]C", "[13CH4]", "[2H]C([2H])([2H])O", "C[18OH]", "[15NH2]C", "CC(=O)[18OH]", "[13C]1CCCCC1", "c1ccc2ccccc2c1[2H]",
    "C[13C](=O)O", "[2H]c1ccccc1", "CC[35Cl]", "CC[37Cl]", "C[14CH3]", "[15NH3+]CC(=O)[O-]", "[13CH3][13CH2]O", "O[32P](=O)(O)O",
]

RADICALS = [
    "[CH3]", "C[CH2]", "[CH2]C=C", "[O]N1C(C)(C)CCCC1(C)C", "C[O]", "[OH]", "c1ccccc1[CH2]", "C[C](C)C", "[NH2]", "CC(C)(C)[O]",
    "C[CH]C",
]

TETRA = [
    "C[C@H](N)C(=O)O", "C[C@@H](N)C(=O)O", "C[C@H](O)C(=O)O", "C[C@@H](O)C(=O)O", "C[C@@H](O)[C@H](N)C(=O)O", "C[C@H](O)[C@H](N)C(=O)O",
    "C[C@@H](O)[C@@H](N)C(=O)O", "CN[C@@H](C)[C@H](O)c1ccccc1", "CC(C)[C@@H]1CC[C@@H](C)C[C@H]1O", "OC[C@H]1OC(O)[C@H](O)[C@@H](O)[C@@H]1O",
    "O[C@H](C(=O)O)[C@@H](O)C(=O)O", "C[C@H](O)[C@@H](O)C", "C[C@@H](Cl)Br", "C[C@H](Cl)Br", "F[C@](Cl)(Br)I", "F[C@@](Cl)(Br)I",
    "N[C@@H](Cc1ccccc1)C(=O)O", "N[C@H](Cc1ccccc1)C(=O)O", "C[C@H]1CCCC[C@@H]1C", "C[C@@H]1CCCC[C@@H]1C", "OC(=O)[C@@H]1CCCN1", "OC(=O)[C@H]1CCCN1",
    "C[C@@H]1CC[C@H](O)CC1", "C[C@H]1CC[C@H](O)CC1", "C[C@]12CCC[C@H]1CCCC2", "C[C@@]12CCC[C@H]1CCCC2",
]
TETRA_SCAFF = ["[C@H]({})(F)Cl", "[C@@H]({})(F)Cl", "[C@H]({})(N)C(=O)O", "[C@@H]({})(O)C(=O)O"]

EZ = [
    "C/C=C/C", "C/C=C\\C", "F/C=C/F", "F/C=C\\F", "C/C=C/C(=O)O", "C/C=C\\C(=O)O", "c1ccccc1/C=C/c1ccccc1", "c1ccccc1/C=C\\c1ccccc1",
    "O=C(O)/C=C/c1ccccc1", "O=C(O)/C=C\\c1ccccc1", "Cl/C=C/Cl", "Cl/C=C\\Cl", "C/C=C/C=C/C", "C/C=C/C=C\\C", "N/C=C/C", "N/C=C\\C",
    "OC(=O)/C=C/C(=O)O", "OC(=O)/C=C\\C(=O)O", "CC/C=C/CC", "CC/C=C\\CC", "C/C(F)=C(\\Cl)Br",
]
EZ_SCAFF = ["{}/C=C/C", "{}/C=C\\C"]

RGROUPS = [
    "[*:1]C", "[*:1]CC", "c1ccc([*:1])cc1", "c1ccc([*:1])c([*:2])c1", "C([*:1])([*:2])C", "[*:1]C(=O)O", "[*:1]c1ccccc1[*:2]", "[*:1]N",
    "[*:1]CCO", "C1CC([*:1])CCC1", "[*:1]C([*:2])=O", "[*:1]OC", "c1cc([*:1])ncc1", "[*:1]C[*:2]", "[*:1]C(F)(F)F",
]
RG_SCAFF = ["[*:1]c1ccc({})cc1", "[*:1]C({})C", "[*:1]CC{}"]

SALTS = [
    "[Na+].[Cl-]", "CC(=O)[O-].[Na+]", "C[NH3+].[Cl-]", "[K+].[O-]C(=O)c1ccccc1", "c1ccccc1C(=O)[O-].[Na+]", "[NH4+].[Cl-]",
    "CC(=O)[O-].[Na+].O", "[Li+].[O-]C(=O)C", "C[N+](C)(C)C.[Br-]", "[Mg+2].[Cl-].[Cl-]", "[Ca+2].CC(=O)[O-].CC(=O)[O-]",
    "OS(=O)(=O)c1ccccc1.N", "c1cc[nH+]cc1.[Cl-]", "[Na+].[O-]S(=O)(=O)C", "CCN(CC)CC.Cl", "O.O",
]
SALT_SCAFF = ["{}.[Cl-].[Na+]", "[NH3+]{}.[Br-]", "{}C(=O)[O-].[K+]"]

GROUPS = [
    "CC(C)(C)OC(=O)NC", "CC(C)(C)OC(=O)N1CCCC1", "Cc1ccc(S(=O)(=O)NC)cc1", "COc1ccccc1", "CC(=O)Nc1ccccc1", "O=C(OCc1ccccc1)NC",
    "CS(=O)(=O)OC", "C[Si](C)(C)OC", "CC(C)(C)[Si](C)(C)OC", "C(F)(F)(F)c1ccccc1", "CC(=O)OC", "O=C(c1ccccc1)OC", "CCOC(=O)C", "CN(C)C(=O)C",
    "CC(C)C(=O)O", "[O-][N+](=O)c1ccc(C)cc1",
]
GROUP_SCAFF = ["{}OC(=O)C(C)(C)C", "{}S(=O)(=O)C"]

CATS = [
    ("ring", 28, RINGS, RING_SCAFF), ("fused", 22, FUSED, []), ("charge", 20, CHARGES, []), ("isotope", 14, ISOTOPES, []),
    ("radical", 10, RADICALS, []), ("stereo", 36, TETRA, TETRA_SCAFF), ("ez", 22, EZ, EZ_SCAFF), ("rgroup", 14, RGROUPS, RG_SCAFF),
    ("salt", 16, SALTS, SALT_SCAFF), ("group", 18, GROUPS, GROUP_SCAFF),
]


def main() -> int:
    assert sum(c[1] for c in CATS) == TOTAL, sum(c[1] for c in CATS)
    rng = random.Random(SEED)
    seen, out = set(), []
    for cat, quota, hand, scaff in CATS:
        mine = []
        for s in hand:
            if s not in seen and len(mine) < quota:
                mine.append(s)
                seen.add(s)
        tries = 0
        while len(mine) < quota:
            tries += 1
            assert tries < 10000, f"cannot fill {cat}"
            if scaff:
                base = rng.choice(scaff)
                n = base.count("{}")
                s = base.format(*[rng.choice(SUBS) for _ in range(n)])
            else:
                continue
            if s not in seen:
                mine.append(s)
                seen.add(s)
        assert len(mine) == quota, (cat, len(mine), quota)
        out += [(s, cat) for s in mine]
    for i, (s, cat) in enumerate(out, 1):
        print(f"{s} s{i:03d}-{cat}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
