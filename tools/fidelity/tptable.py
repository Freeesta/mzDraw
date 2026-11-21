#!/usr/bin/env python3
"""Independent check of the TP table (tp-09). The list tests/data/tp/tp12.csv is read here with Python's `csv` module (not our reader).
mzDraw imports it and writes the table `tptable-csv` through the glue door (the engine `sel` of mzLab: the example `replay_glue`). RDKit is
the reference, and what it reads is OUR output: for each row of our CSV, from OUR smiles column,
  1. the Hill formula RDKit finds equals our `formula`;
  2. RDKit's monoisotopic mass equals our `monoisotopic_mass` within 1e-4;
  3. the m/z of the typed adduct, `[M+H]+` = M + m(H) - m(e) and `[M-H]-` = M - m(H) + m(e) with RDKit's most common isotope mass of
     hydrogen and the electron mass 5.485799090441e-4 u, equals our `mz_calc` within 1e-4;
  4. `ppm` recomputed from our `mz_obs` and `mz_calc` equals ours (1 decimal);
  5. for every parent-TP pair: our `form_diff` equals the difference of the two RDKit formulas, and `mass_diff` the difference of the two
     RDKit masses within 1e-4; a root row has empty parent, `form_diff` and `mass_diff`;
  6. the (tp_id, parent_id) pairs equal the `parent` column of the list (read here with `csv`, `;` between parents), no more, no fewer.
The electron mass is the CODATA 2022 value of https://physics.nist.gov/cgi-bin/cuu/Value?meu (5.485 799 090 441(97) x 10^-4 u), read
2026-11-21; it is the value `docs/plan/backlog.md` §1.1 lists ("partly" verified there). The script reads the page again at run time and
fails if the number it carries is another one (with --offline it does not read it and says so).
It also reads, at run time, the header of the S66 EAWAGTPS table of NORMAN-SLE (Zenodo, the latest version of record 3829088) and checks that
our columns map one to one onto the five S66 columns it names:
  smiles -> SMILES, formula -> MolecularFormula, monoisotopic_mass -> ExactMass, form_diff -> Form_Diff, mass_diff -> Mass_Diff,
that the five names are in that header (once each), and, for the first 40 TP rows of S66 that have a pair, that S66's own Mass_Diff and
Form_Diff are TP minus parent (RDKit on the two SMILES of S66), so our sign convention is the one of S66. Reference programs and data are
fetched at run time and never committed; what is committed is the result.
What it does NOT show: that the table is accepted by any NORMAN tool (no validator was run: the CSV is "NORMAN-style", never
"NORMAN-ready"); that the spelling of the cells is S66's: it is not (see `s66.spelling` in the reference: `form_diff` is
`+O-H2`, S66 writes `H-2O1`; `formula` has no 1 counts, S66 does for some rows; a positive `mass_diff` has a plus sign here, S66 has none; S66 has
a row for the parent as well as one for the TP; InChIKey is not a column here). The masses are the engine's table (AME) as RDKit's are, so
the check does not prove the table of nuclide masses, only the sums, the rounding and the column logic.
Usage:  cargo build -p mzdraw-wasm --example replay_glue
        python3 tools/fidelity/tptable.py --replay target/debug/examples/replay_glue --out tests/data/fidelity/tptable.json
Exit 0 all agree, 1 a disagreement, 2 tool error, 3 RDKit missing. RDKit is installed at run time (pip install rdkit), never committed."""
import argparse, csv, datetime, io, json, os, re, subprocess, sys, urllib.request

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
LIST = os.path.join(ROOT, "tests", "data", "tp", "tp12.csv")
ELECTRON = 5.485799090441e-4          # CODATA 2022, u
S66_RECORD = "https://zenodo.org/api/records/3829088/versions/latest"
S66_MAP = {"smiles": "SMILES", "formula": "MolecularFormula", "monoisotopic_mass": "ExactMass", "form_diff": "Form_Diff", "mass_diff": "Mass_Diff"}
NIST = "https://physics.nist.gov/cgi-bin/cuu/Value?meu"


def fetch(url, rng=None):
    req = urllib.request.Request(url, headers={"User-Agent": "tp-table-check"})
    if rng: req.add_header("Range", rng)
    with urllib.request.urlopen(req, timeout=120) as r: return r.read()


def counts(text):
    """Element counts of a Hill formula such as C10H14O."""
    out = {}
    for el, n in re.findall(r"([A-Z][a-z]?)(\d*)", text): out[el] = out.get(el, 0) + int(n or 1)
    return out


def delta_counts(text):
    """A delta in our spelling (`+O-H2`): a sign applies to the elements that follow it up to the next sign."""
    out, sign = {}, 0
    for tok in re.findall(r"[+-]|[A-Z][a-z]?\d*", text):
        if tok in "+-": sign = 1 if tok == "+" else -1
        else:
            el, n = re.match(r"([A-Z][a-z]?)(\d*)", tok).groups(); out[el] = out.get(el, 0) + sign * int(n or 1)
    return {k: v for k, v in out.items() if v}


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--replay", required=True); ap.add_argument("--out"); ap.add_argument("--offline", action="store_true"); args = ap.parse_args()
    try:
        from rdkit import Chem, rdBase
        from rdkit.Chem import Descriptors, rdMolDescriptors
    except ImportError:
        print("RDKit is not installed: not measured (pip install rdkit)", file=sys.stderr); return 3
    text = open(LIST, newline="").read()
    rows_in = list(csv.DictReader(io.StringIO(text)))
    request = {"commands": [{"command": "tp:import", "args": {"csv": text}}], "out": "tptable-csv"}
    r = subprocess.run([args.replay], input=json.dumps(request), capture_output=True, text=True)
    if r.returncode != 0: print("replay failed: " + r.stdout + r.stderr, file=sys.stderr); return 2
    ours = list(csv.DictReader(io.StringIO(r.stdout)))
    checks, bad = 0, 0
    def check(ok, what):
        nonlocal checks, bad
        checks += 1
        if not ok: bad += 1; print("FAIL " + what, file=sys.stderr)
    pt = Chem.GetPeriodicTable(); h = pt.GetMostCommonIsotopeMass("H")
    mol = {}
    # the electron mass
    nist = "not read (--offline)"
    if not args.offline:
        try:
            page = re.sub(r"&nbsp;|<[^>]*>|\s+", " ", fetch(NIST).decode("utf-8", "replace"))
            m = re.search(r"Numerical value\s*([\d. ]+)\s*x\s*10\s*-4", page)
            value = float(m.group(1).replace(" ", "")) * 1e-4 if m else None
            check(value is not None and abs(value - ELECTRON) < 1e-13, f"the electron mass on {NIST} is {value}, the script has {ELECTRON}")
            nist = f"{value} u, {NIST}, read {datetime.date.today().isoformat()}"
        except Exception as e:
            print("NIST page not read: " + str(e), file=sys.stderr); nist = "not read: " + str(e)
    # 1-5: row by row, from OUR smiles
    by_name = {}
    for r in ours: by_name.setdefault(r["tp_id"], r)         # every row of one tp_id has the same structure
    for r in ours:
        who = r["tp_id"] + " <- " + r["parent_id"]
        m = Chem.MolFromSmiles(r["smiles"])
        if m is None: check(False, f"{who}: RDKit cannot read our SMILES {r['smiles']}"); continue
        mol[r["tp_id"]] = m
        check(counts(rdMolDescriptors.CalcMolFormula(m)) == counts(r["formula"]), f"{who}: formula {r['formula']} vs RDKit {rdMolDescriptors.CalcMolFormula(m)}")
        mass = Descriptors.ExactMolWt(m)
        check(abs(mass - float(r["monoisotopic_mass"])) <= 1e-4, f"{who}: mass {r['monoisotopic_mass']} vs RDKit {mass:.6f}")
        shift = {"[M+H]+": h - ELECTRON, "[M-H]-": -h + ELECTRON}[r["adduct"]]
        check(abs(mass + shift - float(r["mz_calc"])) <= 1e-4, f"{who}: m/z {r['mz_calc']} vs {mass + shift:.6f}")
        want = f"{(float(r['mz_obs']) - float(r['mz_calc'])) / float(r['mz_calc']) * 1e6:.1f}"
        check(r["ppm"] == ("0.0" if want == "-0.0" else want), f"{who}: ppm {r['ppm']} vs {want}")
        if r["parent_id"] == "":
            check(r["form_diff"] == "" and r["mass_diff"] == "", f"{who}: a root has no differences")
        else:
            p = by_name[r["parent_id"]]; pm = Chem.MolFromSmiles(p["smiles"])
            d = {k: counts(rdMolDescriptors.CalcMolFormula(m)).get(k, 0) - counts(rdMolDescriptors.CalcMolFormula(pm)).get(k, 0) for k in set(counts(rdMolDescriptors.CalcMolFormula(m))) | set(counts(rdMolDescriptors.CalcMolFormula(pm)))}
            check({k: v for k, v in d.items() if v} == delta_counts(r["form_diff"]), f"{who}: form_diff {r['form_diff']} vs RDKit {d}")
            check(abs((mass - Descriptors.ExactMolWt(pm)) - float(r["mass_diff"])) <= 1e-4, f"{who}: mass_diff {r['mass_diff']}")
    # 6: the parent links of the list
    want = {(x["name"] or None, p) for x in rows_in for p in (x["parent"].split(";") if x["parent"] else [""])}
    got = {(x["tp_id"], x["parent_id"]) for x in ours}
    named = {(a, b) for a, b in want if a}      # the rows with no name were numbered by the import: compare the named ones, and the count of all
    check(named <= got, f"the named pairs of the list are in the table: missing {named - got}")
    check(len(ours) == sum(max(1, len(x["parent"].split(";")) if x["parent"] else 1) for x in rows_in), "one row per parent pair")
    # S66
    s66 = {"read": False}
    if not args.offline:
        try:
            meta = json.loads(fetch(S66_RECORD))
            f = next(f for f in meta["files"] if f["key"] == "EawagTPandParents.csv")
            body = fetch(f["links"]["self"]).decode("utf-8", "replace")
            rows = list(csv.DictReader(io.StringIO(body)))
            head = list(rows[0].keys())
            s66 = {"read": True, "record": meta["links"]["self_html"], "version": meta["metadata"]["version"], "date": meta["metadata"]["publication_date"], "file": f["key"], "columns": len(head)}
            for ours_col, theirs in S66_MAP.items():
                check(head.count(theirs) == 1 and list(ours[0].keys()).count(ours_col) == 1, f"S66 column {theirs} is in the header once and our {ours_col} too")
            # the convention of S66: Mass_Diff and Form_Diff of a TP row are TP minus its pair (the parent)
            byid = {r["Eawag_ID"]: r for r in rows if r["ParentOrTP"] == "Parent"}
            seen = agree = 0
            for r in rows:
                if seen == 40: break
                if r["ParentOrTP"] != "TP" or not r["Mass_Diff"] or r["Eawag_ID_of_Pair"] not in byid: continue
                a, b = Chem.MolFromSmiles(r["SMILES"]), Chem.MolFromSmiles(byid[r["Eawag_ID_of_Pair"]]["SMILES"])
                if a is None or b is None: continue
                seen += 1; agree += abs((Descriptors.ExactMolWt(a) - Descriptors.ExactMolWt(b)) - float(r["Mass_Diff"])) <= 2e-3
            check(seen >= 30 and agree == seen, f"S66 Mass_Diff is TP minus parent: {agree} of {seen} rows agree")
            s66.update({"mass_diff_rows_compared": seen, "mass_diff_rows_agreeing": agree})
        except Exception as e:
            print("S66 not read: " + str(e), file=sys.stderr); s66 = {"read": False, "why": str(e)}
    s66["spelling"] = {"form_diff": "ours +O-H2 (mzlab_mol::delta::print), S66 H-2O1", "formula": "ours no 1 counts, S66 writes them for some rows (C12H17N1O4S1)",
                       "mass_diff": "ours has a plus sign on a positive value, S66 none", "rows": "S66 has a row for the parent and one for the TP (ParentOrTP, Eawag_ID_of_Pair); ours one row per parent-TP pair with parent_id", "inchikey": "not a column here"}
    ref = {"tool": "tools/fidelity/tptable.py", "reference": f"RDKit {rdBase.rdkitVersion}", "date": datetime.date.today().isoformat(), "list": "tests/data/tp/tp12.csv",
           "rows": len(ours), "checks": checks, "passed": checks - bad, "electron_mass": nist, "s66": s66}
    out = json.dumps(ref, indent=2) + "\n"
    if args.out: open(args.out, "w").write(out)
    else: print(out, end="")
    print(f"tptable: {checks - bad}/{checks} checks agree with RDKit {rdBase.rdkitVersion}", file=sys.stderr)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
