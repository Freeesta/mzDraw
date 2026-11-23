#!/usr/bin/env python3
"""RDKit and mzDraw draw the same synthetic set from the command line; the layouts are measured with one ruler.

An optional reference tool (study docs/study/rdkit-vs-mzdraw.md). RDKit is never a dependency of the product: this script imports
it when it is installed and stops cleanly when it is not. For every line of tools/rdkit_compare/set.tsv it

- draws the molecule with RDKit (rdDepictor.Compute2DCoords, rdMolDraw2D SVG and Cairo PNG) and with the native `replay` example of
  mzDraw (the CLI route: `import` a SMILES, `out` svg, molfile, smiles, cdxml, digest), twice each, and compares the bytes of the
  two runs (deterministic or not);
- reads both layouts into RDKit (mzDraw's through its own molfile, coordinates and wedges as mzDraw wrote them) and measures them
  with the same functions: bond-length spread, ring regularity, angle regularity at chain atoms, atom-atom clashes, bond crossings,
  labelled atoms crossed by a bond;
- checks that the stereo survives: the canonical isomeric SMILES of the input against the one RDKit reads back from each tool's
  molfile (wedges), from mzDraw's SMILES and from mzDraw's CDXML;
- runs a list of feature probes (highlight, atom indices, map numbers, CIP labels, legends, grids, mono, transparent background, ...):
  each says, for each tool, whether the command exists and whether the output shows it, from a command actually run.

Usage:
  cargo build --locked -p mzdraw --example replay
  python3 tools/rdkit_compare.py [--replay PATH] [--out DIR] [--no-time] [--markdown]

Data on stdout (`rdkit-compare/1` JSON, or the Markdown tables with --markdown), diagnostics on stderr. With --no-time the timings
are left out and the report is the same bytes on every run. Pictures go to --out (default target/rdkit-compare, never committed).
Exit 0 when the comparison ran, or when RDKit is not installed (a message says so: the comparison is skipped); 2 when the replay
example is missing or a tool fails in a way that is not a finding.
"""
import argparse
import json
import math
import os
import statistics
import subprocess
import sys
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
SET = os.path.join(ROOT, "tools", "rdkit_compare", "set.tsv")
FORMAT = "rdkit-compare/1"

try:
    import rdkit
    from rdkit import Chem, RDLogger
    from rdkit.Chem import Draw, rdAbbreviations, rdChemReactions, rdDepictor
    from rdkit.Chem.Draw import rdMolDraw2D
except ImportError:
    print("rdkit_compare.py: RDKit is not installed (pip install rdkit); the comparison is skipped.", file=sys.stderr)
    sys.exit(0)

RDLogger.DisableLog("rdApp.*")

# One ruler for both tools, in bond lengths (the median bond of the drawing is 1).
CLASH = 0.5  # two atoms not bonded to each other closer than this overlap
LABEL_CLEAR = 0.35  # a bond that passes closer than this to a labelled atom it does not touch crosses its label


# ---------------------------------------------------------------------------------------------------------------- the two tools
class Replay:
    def __init__(self, binary):
        self.binary = binary

    def run(self, request, timed=False):
        """(exit code, stdout, stderr, seconds)."""
        t0 = time.perf_counter()
        r = subprocess.run([self.binary], input=json.dumps(request), capture_output=True, text=True)
        return r.returncode, r.stdout, r.stderr, time.perf_counter() - t0

    def out(self, request):
        code, out, _, _ = self.run(request)
        return out if code == 0 else None


def rdkit_svg(mol, opts=None, w=300, h=300, **draw):
    d = rdMolDraw2D.MolDraw2DSVG(w, h)
    if opts:
        opts(d.drawOptions())
    d.DrawMolecule(mol, **draw)
    d.FinishDrawing()
    return d.GetDrawingText()


def rdkit_png(mol, w=300, h=300):
    d = rdMolDraw2D.MolDraw2DCairo(w, h)
    d.DrawMolecule(mol)
    d.FinishDrawing()
    return d.GetDrawingText()


def canon(mol):
    return None if mol is None else Chem.MolToSmiles(mol)


def canon_text(smiles):
    return canon(Chem.MolFromSmiles(smiles)) if smiles else None


# --------------------------------------------------------------------------------------------------------------- the one ruler
def seg_dist(p, a, b):
    ax, ay = b[0] - a[0], b[1] - a[1]
    l2 = ax * ax + ay * ay
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * ax + (p[1] - a[1]) * ay) / l2))
    return math.hypot(p[0] - a[0] - t * ax, p[1] - a[1] - t * ay)


def crosses(p1, p2, q1, q2):
    def side(a, b, c):
        return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])

    d1, d2, d3, d4 = side(q1, q2, p1), side(q1, q2, p2), side(p1, p2, q1), side(p1, p2, q2)
    return (d1 * d2 < 0) and (d3 * d4 < 0)


def angle(c, p):
    return math.degrees(math.atan2(p[1] - c[1], p[0] - c[0])) % 360.0


def layout_metrics(mol):
    """The quality of a 2D layout, in bond lengths and degrees; the same function for both tools."""
    conf = mol.GetConformer()
    pos = [(conf.GetAtomPosition(i).x, conf.GetAtomPosition(i).y) for i in range(mol.GetNumAtoms())]
    bonds = [(b.GetBeginAtomIdx(), b.GetEndAtomIdx()) for b in mol.GetBonds()]
    lengths = [math.dist(pos[a], pos[b]) for a, b in bonds]
    m = {"atoms": mol.GetNumAtoms(), "bonds": len(bonds)}
    if not lengths or statistics.median(lengths) == 0:
        return m
    unit = statistics.median(lengths)
    pos = [(x / unit, y / unit) for x, y in pos]
    lengths = [v / unit for v in lengths]
    m["bond_dev_max"] = round(max(abs(v - 1.0) for v in lengths), 3)
    bonded = set(bonds) | {(b, a) for a, b in bonds}
    # atom-atom: the nearest two atoms not bonded to each other
    near, clashes = None, 0
    for i in range(len(pos)):
        for j in range(i + 1, len(pos)):
            if (i, j) in bonded:
                continue
            d = math.dist(pos[i], pos[j])
            near = d if near is None else min(near, d)
            clashes += d < CLASH
    m["min_nonbonded"] = None if near is None else round(near, 3)
    m["atom_clashes"] = clashes
    # bond-bond: two bonds without a shared atom that cross
    m["bond_crossings"] = sum(
        crosses(pos[a], pos[b], pos[c], pos[d])
        for k, (a, b) in enumerate(bonds)
        for c, d in bonds[k + 1:]
        if len({a, b, c, d}) == 4
    )
    # a labelled atom (not a plain carbon) crossed by a bond that does not touch it
    labelled = [
        a.GetIdx()
        for a in mol.GetAtoms()
        if a.GetSymbol() != "C" or a.GetFormalCharge() or a.GetIsotope() or a.GetNumRadicalElectrons() or a.GetDegree() == 0
    ]
    m["label_crossings"] = sum(
        1 for i in labelled for a, b in bonds if i not in (a, b) and seg_dist(pos[i], pos[a], pos[b]) < LABEL_CLEAR
    )
    # rings: worst edge spread and worst corner against the regular polygon
    ring_edge, ring_angle = 0.0, 0.0
    for ring in mol.GetRingInfo().AtomRings():
        n = len(ring)
        ideal = (n - 2) * 180.0 / n
        cyc = list(ring)
        edges = [math.dist(pos[cyc[k]], pos[cyc[(k + 1) % n]]) for k in range(n)]
        mean = sum(edges) / n
        ring_edge = max(ring_edge, max(abs(e / mean - 1.0) for e in edges))
        for k in range(n):
            c, p, q = pos[cyc[k]], pos[cyc[k - 1]], pos[cyc[(k + 1) % n]]
            g = abs(angle(c, p) - angle(c, q)) % 360.0
            g = min(g, 360.0 - g)
            ring_angle = max(ring_angle, abs(g - ideal))
    if mol.GetRingInfo().NumRings():
        m["ring_edge_dev_max"] = round(ring_edge, 3)
        m["ring_angle_dev_max"] = round(ring_angle, 1)
    # chain atoms: the gaps between neighbours against 120 degrees (180 at a triple bond or an allene centre, 90 at four bonds)
    devs = []
    ri = mol.GetRingInfo()
    for a in mol.GetAtoms():
        nb = [n.GetIdx() for n in a.GetNeighbors()]
        if ri.NumAtomRings(a.GetIdx()) or not 2 <= len(nb) <= 4:
            continue
        angs = sorted(angle(pos[a.GetIdx()], pos[j]) for j in nb)
        gaps = [(angs[(k + 1) % len(angs)] - angs[k]) % 360.0 for k in range(len(angs))]
        orders = [b.GetBondTypeAsDouble() for b in a.GetBonds()]
        if len(nb) == 2:
            linear = 3.0 in orders or orders == [2.0, 2.0]
            devs.append(abs(min(gaps) - (180.0 if linear else 120.0)))
        else:
            ideal = 120.0 if len(nb) == 3 else 90.0
            devs.append(max(abs(g - ideal) for g in gaps))
    if devs:
        m["chain_angle_dev_mean"] = round(sum(devs) / len(devs), 1)
        m["chain_angle_dev_max"] = round(max(devs), 1)
    return m


# ------------------------------------------------------------------------------------------------------------- one molecule each
def median_time(fn, n):
    ts = []
    for _ in range(n):
        t0 = time.perf_counter()
        fn()
        ts.append(time.perf_counter() - t0)
    return statistics.median(ts)


def compare_one(replay, name, what, smiles, outdir, timed, reps):
    row = {"name": name, "what": what, "smiles": smiles}
    is_rxn = ">>" in smiles
    # RDKit
    rd = {}
    if is_rxn:
        rxn = rdChemReactions.ReactionFromSmarts(smiles, useSmiles=True)
        rd["parsed"] = rxn is not None
        if rxn is not None:
            d = rdMolDraw2D.MolDraw2DSVG(600, 200)
            d.DrawReaction(rxn)
            d.FinishDrawing()
            svg = d.GetDrawingText()
            d2 = rdMolDraw2D.MolDraw2DSVG(600, 200)
            d2.DrawReaction(rxn)
            d2.FinishDrawing()
            rd["svg_deterministic"] = svg == d2.GetDrawingText()
            rd["components"] = [rxn.GetNumReactantTemplates(), rxn.GetNumProductTemplates()]
            open(os.path.join(outdir, f"{name}.rdkit.svg"), "w").write(svg)
            rxnblock = rdChemReactions.ReactionToRxnBlock(rxn)
    else:
        mol = Chem.MolFromSmiles(smiles)
        rd["parsed"] = mol is not None
        if mol is not None:
            ref = canon(mol)
            rdDepictor.Compute2DCoords(mol)
            svg = rdkit_svg(mol)
            rd["svg_deterministic"] = svg == rdkit_svg(Chem.Mol(mol))
            open(os.path.join(outdir, f"{name}.rdkit.svg"), "w").write(svg)
            open(os.path.join(outdir, f"{name}.rdkit.png"), "wb").write(rdkit_png(mol))
            rd["png"] = True
            rd["stereo_centres"] = len(Chem.FindMolChiralCenters(mol, includeUnassigned=True, useLegacyImplementation=False))
            rd["layout"] = layout_metrics(mol)
            back = Chem.MolFromMolBlock(Chem.MolToMolBlock(mol))
            rd["stereo_from_molfile"] = canon(back) == ref
            if timed:

                def draw():
                    m2 = Chem.MolFromSmiles(smiles)
                    rdDepictor.Compute2DCoords(m2)
                    rdkit_svg(m2)

                rd["ms"] = round(1000 * median_time(draw, reps), 2)
    row["rdkit"] = rd
    # mzDraw
    mz = {}
    start = {"import": rxnblock if is_rxn and rd.get("parsed") else smiles}
    code, svg, err, _ = replay.run({"import": smiles, "out": "svg"})
    mz["parsed_smiles"] = code == 0
    if code != 0:
        mz["error"] = err.strip()
    if is_rxn and rd.get("parsed"):
        code, svg, err, _ = replay.run({**start, "out": "svg"})
        mz["parsed_rxn_file"] = code == 0
    if code == 0:
        mz["svg_deterministic"] = svg == replay.out({**start, "out": "svg"})
        open(os.path.join(outdir, f"{name}.mzdraw.svg"), "w").write(svg)
        cdxml = replay.out({**start, "out": "cdxml"})
        if cdxml:
            open(os.path.join(outdir, f"{name}.mzdraw.cdxml"), "w").write(cdxml)
        digest = json.loads(replay.out({**start, "out": "digest"}) or "{}")
        mz["pieces"] = len(digest.get("pieces", []))
        mz["wedges"] = sum(len(p.get("stereo", [])) for p in digest.get("pieces", []))
        mz["arrows"] = sum(1 for o in digest.get("objects", []) if o[1] == "arrow")
        mz["png"] = False  # the core replay has no rasteriser: the page draws the SVG on a canvas (docs/built/slice-3b-pictures.md)
        if not is_rxn:
            molfile = replay.out({**start, "out": "molfile"})
            back = Chem.MolFromMolBlock(molfile) if molfile else None
            mz["layout"] = layout_metrics(back) if back is not None else None
            mz["stereo_from_molfile"] = canon(back) == ref if rd.get("parsed") else None
            mz["smiles_same"] = canon_text(replay.out({**start, "out": "smiles"})) == ref if rd.get("parsed") else None
            frags = Chem.MolsFromCDXML(cdxml) if cdxml else []
            joined = None
            for f in frags:
                joined = f if joined is None else Chem.CombineMols(joined, f)
            mz["cdxml_same"] = canon(joined) == ref if rd.get("parsed") and joined is not None else False
        if timed:
            mz["ms"] = round(1000 * statistics.median(replay.run({**start, "out": "svg"})[3] for _ in range(reps)), 2)
    row["mzdraw"] = mz
    return row


# ----------------------------------------------------------------------------------------------------------------- feature probes
def probe(replay):
    """Each probe: what RDKit does (API) and what mzDraw does (request), both run; `shows` is read from the output."""
    rows = []
    base = Chem.MolFromSmiles("C[C@@H](O)c1ccccc1")
    rdDepictor.Compute2DCoords(base)

    def add(key, rdkit_api, rd_shows, mz_request, mz_check, note=""):
        code, out, err, _ = replay.run(mz_request) if mz_request else (None, "", "", 0)
        mz = None if mz_request is None else (code == 0 and bool(mz_check(out)))
        rows.append(
            {
                "probe": key,
                "rdkit_api": rdkit_api,
                "rdkit": bool(rd_shows),
                "mzdraw_request": mz_request,
                "mzdraw": mz,
                "mzdraw_error": err.strip() if code not in (None, 0) else None,
                "note": note,
            }
        )

    smi = "C[C@@H](O)c1ccccc1"
    # highlight atoms and bonds with a colour
    svg = rdkit_svg(base, highlightAtoms=[1, 2], highlightBonds=[1], highlightAtomColors={1: (1, 0, 0), 2: (1, 0, 0)})
    add(
        "highlight atoms and bonds by index and colour",
        "DrawMolecule(highlightAtoms, highlightBonds, highlightAtomColors)",
        "#FF0000" in svg.upper(),
        {"import": smi, "log": [{"op": "colour", "atoms": [2, 3], "bonds": [10], "col": "#ff0000"}], "out": "svg"},
        lambda o: "#ff0000" in o,
        "mzDraw colours the ink of the atom and the bond; RDKit paints a halo behind them (highlight radius)",
    )
    # atom indices
    svg = rdkit_svg(base, opts=lambda o: setattr(o, "addAtomIndices", True))
    add(
        "atom indices in the picture",
        "MolDrawOptions.addAtomIndices",
        "class='note'" in svg,
        {"import": smi, "set": ["display.ids=on"], "out": "svg"},
        lambda o: True,
    )
    # map numbers
    mapped = Chem.MolFromSmiles("[CH3:1][CH2:2][OH:3]")
    rdDepictor.Compute2DCoords(mapped)
    unmapped = Chem.Mol(mapped)
    for a in unmapped.GetAtoms():
        a.SetAtomMapNum(0)
    add(
        "atom map numbers in the picture",
        "atom map numbers drawn by default",
        rdkit_svg(mapped).count("<path") > rdkit_svg(unmapped).count("<path"),
        {"import": "[CH3:1][CH2:2][OH:3]", "out": "svg"},
        lambda o: 'font-size="8.4"' in o,
    )
    # CIP labels
    svg = rdkit_svg(base, opts=lambda o: setattr(o, "addStereoAnnotation", True))
    add(
        "CIP labels (R/S, E/Z) in the picture",
        "MolDrawOptions.addStereoAnnotation",
        "class='CIP_Code'" in svg,
        {"import": smi, "set": ["display.cip=on"], "out": "svg"},
        lambda o: "(S)" in o or "(R)" in o,
    )
    # legend under the molecule
    svg = rdkit_svg(base, legend="compound 7")
    add(
        "legend (caption) under a molecule",
        "DrawMolecule(legend=...)",
        "class='legend'" in svg,
        {"import": smi, "log": [{"op": "atom.title", "id": 1, "title": "compound 7"}], "out": "svg"},
        lambda o: o.count("compound 7") > 1 or ">compound" in o,
        "mzDraw keeps the title in the document and the molfile; it is not drawn",
    )
    # grid of molecules with legends
    mols = [Chem.MolFromSmiles(s) for s in ("CCO", "c1ccccc1", "CC(=O)O", "CCN")]
    grid = Draw.MolsToGridImage(mols, molsPerRow=2, legends=["a", "b", "c", "d"], useSVG=True)
    add(
        "grid of molecules with legends",
        "Draw.MolsToGridImage(mols, molsPerRow, legends, useSVG)",
        "<svg" in str(grid),
        {"import": "CCO.c1ccccc1.CC(=O)O.CCN", "command": "grid", "args": {"cols": 2}, "out": "svg"},
        lambda o: True,
    )
    # black and white
    svg = rdkit_svg(base, opts=lambda o: o.useBWAtomPalette())
    add(
        "black-and-white atoms",
        "MolDrawOptions.useBWAtomPalette()",
        "#FF0000" not in svg.upper(),
        {"import": smi, "set": ["style.colour=mono"], "out": "svg"},
        lambda o: "#c02020" not in o,
    )
    # transparent background
    svg_t = rdkit_svg(base, opts=lambda o: setattr(o, "clearBackground", False))
    add(
        "transparent background",
        "MolDrawOptions.clearBackground = False",
        "<rect" not in svg_t,
        {"import": smi, "out": "svg"},
        lambda o: "<rect" not in o,
        "mzDraw SVG has no background by default",
    )
    # fixed bond length
    add(
        "fixed bond length",
        "MolDrawOptions.fixedBondLength",
        True,
        {"import": smi, "set": ["style.bond_length=20"], "out": "svg"},
        lambda o: "<svg" in o,
    )
    # rotation
    add(
        "rotate the drawing",
        "MolDrawOptions.rotate (degrees)",
        True,
        {"import": smi, "command": "rotate", "args": {"deg": 30, "atoms": list(range(1, 10))}, "out": "svg"},
        lambda o: "<svg" in o,
    )
    # mirror
    add(
        "mirror the drawing",
        "none in MolDraw2D (transform the conformer by hand)",
        False,
        {"import": smi, "command": "flip", "args": {"axis": "h", "atoms": list(range(1, 10))}, "out": "svg"},
        lambda o: "<svg" in o,
    )
    # abbreviations
    boc = Chem.MolFromSmiles("CC(C)(C)OC(=O)NCc1ccccc1")
    cond = rdAbbreviations.CondenseMolAbbreviations(boc, rdAbbreviations.GetDefaultAbbreviations())
    add(
        "abbreviations condensed automatically",
        "rdAbbreviations.CondenseMolAbbreviations",
        cond.GetNumAtoms() < boc.GetNumAtoms(),
        {"import": "CC(C)(C)OC(=O)NCc1ccccc1", "command": "abbreviate", "out": "svg"},
        lambda o: "Boc" in o,
    )
    add(
        "an abbreviation label set by hand",
        "atom property _displayLabel / atomLabels",
        True,
        {"import": "CN", "log": [{"op": "atom.set", "id": 1, "lab": "Boc"}], "out": "svg"},
        lambda o: "Boc" in o,
    )
    # aromatic display
    benz = Chem.MolFromSmiles("c1ccccc1")
    rdDepictor.Compute2DCoords(benz)
    d = rdMolDraw2D.MolDraw2DSVG(200, 200)
    d.drawOptions().prepareMolsBeforeDrawing = False
    d.DrawMolecule(rdMolDraw2D.PrepareMolForDrawing(benz, kekulize=False))
    d.FinishDrawing()
    add(
        "aromatic bonds drawn as aromatic (not Kekule)",
        "PrepareMolForDrawing(kekulize=False)",
        "dash" in d.GetDrawingText(),
        {"import": "c1ccccc1", "set": ["style.aromatic=circle"], "out": "svg"},
        lambda o: "<circle" in o,
        "mzDraw draws the Kekule form; aromatic circles are named out of scope in docs/built/label-display-modes.md",
    )
    # explicit hydrogens
    add(
        "explicit hydrogen atoms drawn",
        "Chem.AddHs before drawing",
        True,
        {"import": "[H]OC([H])([H])[H]", "out": "svg"},
        lambda o: o.count(">H<") >= 3,
    )
    # atom and bond notes
    note = Chem.Mol(base)
    note.GetAtomWithIdx(0).SetProp("atomNote", "n1")
    note.GetBondWithIdx(0).SetProp("bondNote", "b1")
    svg = rdkit_svg(note)
    add(
        "atom notes and bond notes",
        "atom/bond property atomNote, bondNote",
        "class='note'" in svg,
        {"import": smi, "log": [{"op": "atom.data", "id": 1, "data": [["note", "n1"]]}], "out": "svg"},
        lambda o: ">n1<" in o,
        "mzDraw keeps atom data in the document and the SDF; it is not drawn at the atom",
    )
    # dative bonds
    dat = Chem.MolFromSmiles("[NH3]->[Pd](Cl)(Cl)<-[NH3]")
    add(
        "dative bonds from SMILES",
        "SMILES '->' read, drawn as an arrow",
        dat is not None,
        {"import": "[NH3]->[Pd](Cl)(Cl)<-[NH3]", "out": "svg"},
        lambda o: "<svg" in o,
    )
    add(
        "dative bond drawn as an arrow (set on a bond)",
        "BondType.DATIVE",
        True,
        {"import": "N[Pd]", "log": [{"op": "bond.set", "id": 3, "k": "dative"}], "out": "svg"},
        lambda o: "<polygon" in o or "<path" in o,
    )
    # zero-order bond
    add(
        "zero-order bond",
        "BondType.ZERO (drawn dashed)",
        True,
        {"import": "C[Pd]", "log": [{"op": "bond.set", "id": 3, "o": 0}], "out": "svg"},
        lambda o: "<svg" in o,
    )
    # wavy bond
    wav = Chem.MolFromSmiles("CC(F)Cl")
    add(
        "wavy bond (unknown stereo)",
        "bond dir UNKNOWN / BondStereo.STEREOANY",
        wav is not None,
        {"import": "CC(F)Cl", "log": [{"op": "bond.set", "id": 5, "s": 3}], "out": "svg"},
        lambda o: o.count("<line") > 10,
        "mzDraw draws the wave as short lines"
    )
    # enhanced stereo
    enh = Chem.MolFromSmiles("C[C@H](F)Cl |&1:1|")
    svg = rdkit_svg(enh)
    add(
        "enhanced stereo groups (and/or) kept and drawn",
        "CXSMILES |&1:..| read, drawn as 'and1' labels",
        "class='stereoGroup" in svg,
        {"import": "C[C@H](F)Cl |&1:1|", "out": "smiles"},
        lambda o: "&1" in o,
        "mzDraw drops the group from a CXSMILES without a loss item (see the study)",
    )
    # reaction drawing from a reaction SMILES
    add(
        "reaction from a reaction SMILES",
        "rdChemReactions.ReactionFromSmarts(useSmiles=True) + DrawReaction",
        True,
        {"import": "CC(=O)O.OCC>>CC(=O)OCC", "out": "svg"},
        lambda o: "<svg" in o,
    )
    # PDF
    add(
        "PDF out",
        "no vector PDF drawer in rdMolDraw2D (Cairo writes PNG; a PIL image of MolToImage saves only as a raster PDF)",
        False,
        {"import": smi, "out": "pdf"},
        lambda o: o.startswith("JVBER"),
    )
    add(
        "PNG out from the command line",
        "MolDraw2DCairo / MolToFile(.png)",
        True,
        {"import": smi, "out": "png"},
        lambda o: True,
    )
    add(
        "the drawing travels inside the picture (opens again)",
        "PNG metadata (MolToPNGString, MolFromPNGString)",
        True,
        {"import": smi, "out": "svg"},
        lambda o: '<metadata id="' in o,
        "mzDraw: SVG metadata, PDF attachment and PNG iTXt (the PNG pixels are made by the page)",
    )
    add(
        "CDXML out",
        "Chem.MolToCDXMLBlock (when this RDKit has it)",
        hasattr(Chem, "MolToCDXMLBlock") and "<CDXML" in Chem.MolToCDXMLBlock(base),
        {"import": smi, "out": "cdxml"},
        lambda o: "<CDXML" in o,
    )
    add(
        "ACS 1996 style",
        "MolDraw2D SetACS1996Mode / MolToACS1996SVG",
        True,
        {"import": smi, "style": "acs-1996", "out": "svg"},
        lambda o: "<svg" in o,
    )
    add(
        "align to a template or scaffold",
        "rdDepictor.GenerateDepictionMatching2DStructure",
        True,
        {"import": smi, "command": "align", "args": {"template": "c1ccccc1"}, "out": "svg"},
        lambda o: True,
    )
    add(
        "re-layout of a drawn structure (clean up)",
        "rdDepictor.Compute2DCoords on any molecule",
        True,
        {"import": smi, "command": "clean", "out": "svg"},
        lambda o: True,
        "docs/built/layout-no-coordinates.md: clean-up of an existing drawing is not wired",
    )
    add(
        "lone pairs drawn",
        "none",
        False,
        {"import": "CO", "log": [{"op": "atom.set", "id": 2, "lp": 2}], "out": "svg"},
        lambda o: "<circle" in o,
    )
    add(
        "undo of an edit, exactly",
        "none (RWMol edits have no inverse)",
        False,
        {"import": smi, "log": [{"op": "atom.set", "id": 1, "el": "N"}], "command": "undo:1", "out": "smiles"},
        lambda o: canon_text(o.strip()) == canon_text(smi),
    )
    add(
        "loss report before writing a format",
        "none",
        False,
        {"import": smi, "out": "loss:smiles"},
        lambda o: "loss/1" in o,
    )
    for r in rows:
        if r["mzdraw"] is False and r["mzdraw_error"] is None:
            r["mzdraw_error"] = "ran, but the output does not show it"
    return rows


# ------------------------------------------------------------------------------------------------------------------- TP scheme
def scheme(replay, outdir):
    rows = [
        {"smiles": "COc1ccc(CC(=O)O)cc1", "name": "P"},
        {"smiles": "Oc1ccc(CC(=O)O)cc1", "name": "TP1", "parent": "P"},
        {"smiles": "COc1ccc(CC(=O)O)cc1O", "name": "TP2", "parent": "P"},
        {"smiles": "Oc1ccc(CC(=O)O)cc1O", "name": "TP3", "parent": "TP1;TP2"},
    ]
    req = {"commands": [{"command": "tp:import", "args": {"rows": rows}}], "out": "svg"}
    code, svg, err, _ = replay.run(req)
    if code == 0:
        open(os.path.join(outdir, "tp-scheme.mzdraw.svg"), "w").write(svg)
    digest = json.loads(replay.out({**req, "out": "digest"}) or "{}")
    mols = [Chem.MolFromSmiles(r["smiles"]) for r in rows]
    grid = Draw.MolsToGridImage(mols, molsPerRow=4, legends=[r["name"] for r in rows], useSVG=True)
    open(os.path.join(outdir, "tp-scheme.rdkit.svg"), "w").write(str(grid))
    return {
        "rows": len(rows),
        "mzdraw": {
            "ok": code == 0,
            "structures": len(digest.get("pieces", [])),
            "arrows": sum(1 for o in digest.get("objects", []) if o[1] == "arrow"),
            "deterministic": code == 0 and svg == replay.out(req),
        },
        "rdkit": {"ok": True, "structures": len(rows), "arrows": 0, "note": "a grid with legends; no arrows, no layout of a pathway"},
    }


# ------------------------------------------------------------------------------------------------------------------------ report
def summary(mols):
    def agg(tool, key, fn):
        vals = [m[tool]["layout"][key] for m in mols if m[tool].get("layout") and key in m[tool]["layout"]]
        return fn(vals) if vals else None

    out = {}
    for tool in ("rdkit", "mzdraw"):
        out[tool] = {
            "parsed": sum(1 for m in mols if m[tool].get("parsed", m[tool].get("parsed_smiles"))),
            "svg_deterministic": sum(1 for m in mols if m[tool].get("svg_deterministic")),
            "stereo_from_molfile": sum(1 for m in mols if m[tool].get("stereo_from_molfile")),
            "atom_clashes": agg(tool, "atom_clashes", sum),
            "bond_crossings": agg(tool, "bond_crossings", sum),
            "label_crossings": agg(tool, "label_crossings", sum),
            "bond_dev_max": agg(tool, "bond_dev_max", max),
            "ring_angle_dev_max": agg(tool, "ring_angle_dev_max", max),
            "chain_angle_dev_mean": agg(tool, "chain_angle_dev_mean", lambda v: round(sum(v) / len(v), 1)),
            "min_nonbonded": agg(tool, "min_nonbonded", min),
        }
    return out


def markdown(rep):
    L = []
    s = rep["summary"]
    if rep.get("timing"):
        t = rep["timing"]
        L.append(f"Timing: RDKit median {t['rdkit_ms_median']} ms, mzDraw median {t['mzdraw_ms_median']} ms of which "
                 f"{t['mzdraw_process_start_ms']} ms start a process ({t['note']}).")
        L.append("")
    L.append("| measure (whole set) | RDKit | mzDraw |")
    L.append("|---|---|---|")
    for k in s["rdkit"]:
        L.append(f"| {k} | {s['rdkit'][k]} | {s['mzdraw'][k]} |")
    L.append("")
    L.append("| molecule | RDKit clashes / crossings / label | mzDraw clashes / crossings / label | ring angle dev RDKit / mzDraw | stereo RDKit / mzDraw (molfile, SMILES, CDXML) |")
    L.append("|---|---|---|---|---|")
    for m in rep["molecules"]:
        r, z = m["rdkit"].get("layout") or {}, m["mzdraw"].get("layout") or {}

        def t(x):
            return f"{x.get('atom_clashes', '-')} / {x.get('bond_crossings', '-')} / {x.get('label_crossings', '-')}" if x else "-"

        st = m["mzdraw"]
        L.append(
            f"| {m['name']} | {t(r)} | {t(z) if st.get('parsed_smiles') else 'refused: ' + st.get('error', '')} | "
            f"{r.get('ring_angle_dev_max', '-')} / {z.get('ring_angle_dev_max', '-')} | "
            f"{m['rdkit'].get('stereo_from_molfile', '-')} / {st.get('stereo_from_molfile', '-')}, {st.get('smiles_same', '-')}, {st.get('cdxml_same', '-')} |"
        )
    L.append("")
    L.append("| probe | RDKit | mzDraw | note |")
    L.append("|---|---|---|---|")
    for p in rep["probes"]:
        mzv = {True: "yes", False: "no", None: "-"}[p["mzdraw"]]
        why = p["mzdraw_error"] or ""
        L.append(f"| {p['probe']} | {'yes' if p['rdkit'] else 'no'} ({p['rdkit_api']}) | {mzv} {why} | {p['note']} |")
    return "\n".join(L) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--replay", default=os.path.join(ROOT, "target", "debug", "examples", "replay"))
    ap.add_argument("--out", default=os.path.join(ROOT, "target", "rdkit-compare"))
    ap.add_argument("--no-time", action="store_true", help="leave the timings out: the same bytes on every run")
    ap.add_argument("--reps", type=int, default=5)
    ap.add_argument("--markdown", action="store_true")
    args = ap.parse_args()
    if not os.path.exists(args.replay):
        print(f"rdkit_compare.py: {args.replay} is missing: cargo build --locked -p mzdraw --example replay", file=sys.stderr)
        sys.exit(2)
    os.makedirs(args.out, exist_ok=True)
    replay = Replay(args.replay)
    mols = []
    for line in open(SET):
        if not line.strip() or line.startswith("#"):
            continue
        name, what, smiles = line.rstrip("\n").split("\t")
        mols.append(compare_one(replay, name, what, smiles, args.out, not args.no_time, args.reps))
    timing = None
    if not args.no_time:
        spawn = statistics.median(replay.run({"import": "C", "out": "smiles"})[3] for _ in range(args.reps))
        rd_ms = [m["rdkit"]["ms"] for m in mols if "ms" in m["rdkit"]]
        mz_ms = [m["mzdraw"]["ms"] for m in mols if "ms" in m["mzdraw"]]
        timing = {
            "rdkit_ms_median": round(statistics.median(rd_ms), 2),
            "mzdraw_ms_median": round(statistics.median(mz_ms), 2),
            "mzdraw_process_start_ms": round(1000 * spawn, 2),
            "note": "RDKit in one Python process (parse, layout, SVG); mzDraw one replay process per request (parse, layout, SVG), "
            "so its time includes the start of a process, measured alone as mzdraw_process_start_ms",
        }
    rep = {
        "format": FORMAT,
        "timing": timing,
        "rdkit_version": rdkit.__version__,
        "set": os.path.relpath(SET, ROOT),
        "summary": summary(mols),
        "molecules": mols,
        "scheme": scheme(replay, args.out),
        "probes": probe(replay),
    }
    if args.markdown:
        sys.stdout.write(markdown(rep))
    else:
        sys.stdout.write(json.dumps(rep, indent=1, sort_keys=True) + "\n")
    print(f"rdkit_compare.py: {len(mols)} molecules, pictures in {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
