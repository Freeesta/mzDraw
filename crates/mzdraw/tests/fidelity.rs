//! Fidelity reference (plan-03): `tests/data/fidelity/reference.json` was made by `tools/fidelity/run.py` from the files our writers give for the
//! 200 synthetic structures of `set.smi`. This test needs no RDKit, Open Babel or Python: it checks that our writers still give those very
//! bytes (an FNV-1a 64 hash per file), so a change of any writer forces a rerun of the tool and a new reference.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

fn data(name: &str) -> String {
    let p: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "tests",
        "data",
        "fidelity",
        name,
    ]
    .iter()
    .collect();
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The same hash as `fnv1a` of `tools/fidelity/run.py`.
fn fnv1a(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// The formats of the tool: name in the reference -> `out` of the replay request.
const FORMATS: [(&str, &str); 7] = [
    ("molfile-v2000", "molfile"),
    ("molfile-v3000", "molfile-v3000"),
    ("sdf", "sdf"),
    ("smiles", "smiles"),
    ("ket", "ket"),
    ("cdxml", "cdxml"),
    ("cml", "cml"),
];

fn replay(request: &Value) -> Option<String> {
    mzdraw::replay::replay(&request.to_string()).ok()
}

/// What differs between the reference and the files `write` gives (`write(request)` is the replay of a request, possibly altered by a test).
fn mismatches(
    reference: &Value,
    set: &[(String, String)],
    write: &dyn Fn(&str, &str, &Value) -> Option<String>,
) -> Vec<String> {
    let mut bad = Vec::new();
    let forward = reference["forward"].as_object().expect("forward");
    if forward.len() != set.len() {
        bad.push(format!(
            "the reference has {} structures, the set {}",
            forward.len(),
            set.len()
        ));
    }
    for (id, smiles) in set {
        let Some(entry) = forward.get(id) else {
            bad.push(format!("{id}: not in the reference"));
            continue;
        };
        if entry["input"] != json!(smiles) {
            bad.push(format!("{id}: the structure of the set changed"));
        }
        for (name, out) in FORMATS {
            let got = write(id, name, &json!({"import": smiles, "out": out})).map(|t| fnv1a(&t));
            if got.as_deref() != entry[name]["hash"].as_str() {
                bad.push(format!(
                    "{id} {name}: our file changed (hash {got:?}, reference {}): rerun tools/fidelity/run.py",
                    entry[name]["hash"]
                ));
            }
        }
    }
    for (id, entry) in reference["reactions"].as_object().expect("reactions") {
        let got = write(id, "rxn", &entry["request"]).map(|t| fnv1a(&t));
        if got.as_deref() != entry["hash"].as_str() {
            bad.push(format!("{id} rxn: our file changed (hash {got:?}, reference {}): rerun tools/fidelity/run.py", entry["hash"]));
        }
    }
    bad
}

fn load() -> (Value, Vec<(String, String)>) {
    let reference: Value = serde_json::from_str(&data("reference.json")).expect("reference.json");
    let set = data("set.smi")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (smiles, name) = l.split_once(' ').expect("`<smiles> <id>-<category>`");
            (
                name.split('-').next().unwrap().to_string(),
                smiles.to_string(),
            )
        })
        .collect();
    (reference, set)
}

#[test]
fn our_files_are_the_ones_the_reference_was_made_from() {
    let (reference, set) = load();
    assert_eq!(set.len(), 200, "the set is 200 structures");
    let bad = mismatches(&reference, &set, &|_, _, request| replay(request));
    assert!(
        bad.is_empty(),
        "{} file(s) differ:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn the_reference_names_its_programs_and_formats() {
    let (reference, _) = load();
    assert_eq!(reference["format"], "fidelity/1");
    for k in ["rdkit", "rdkit_inchi", "openbabel"] {
        assert!(
            reference["programs"][k]
                .as_str()
                .is_some_and(|v| !v.is_empty()),
            "programs.{k}"
        );
    }
    assert!(reference["date"].as_str().is_some_and(|d| d.len() == 10));
    for (id, entry) in reference["forward"].as_object().unwrap() {
        for (name, _) in FORMATS {
            let e = &entry[name];
            assert!(e["hash"].is_string() && e["lost"].is_array(), "{id} {name}");
            for k in ["inchikey_in", "inchikey_out_rdkit", "inchikey_out_obabel"] {
                assert!(e.get(k).is_some(), "{id} {name} lacks {k}");
            }
        }
    }
}

/// A deliberate change of one writer (here, one byte of one file in a test-only copy of the output) must be caught, naming structure and format.
#[test]
fn a_changed_writer_is_caught_with_structure_and_format() {
    let (reference, set) = load();
    let bad = mismatches(&reference, &set, &|id, name, request| {
        let text = replay(request)?;
        Some(if id == "s042" && name == "cdxml" {
            text.replacen("<page>", "<page> ", 1)
        } else {
            text
        })
    });
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].starts_with("s042 cdxml:"), "{}", bad[0]);
    // and a changed reaction file
    let bad = mismatches(&reference, &set, &|id, name, request| {
        let text = replay(request)?;
        Some(if name == "rxn" && id == "s007+s008" {
            text + "\n"
        } else {
            text
        })
    });
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].starts_with("s007+s008 rxn:"), "{}", bad[0]);
}
