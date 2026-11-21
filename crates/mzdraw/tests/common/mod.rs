//! The golden files of the editor: every `*.json` of `tests/data/log/`, one file per area, read in the order of their names.
//! A file holds some of the sections `cases`, `contexts`, `refused`, `results`, `schemas`, `settings`, `pdf`, `trasf`, `diffs`, `losses`, `values`, `feedbacks` (a missing section is an empty one);
//! `tests/parity.mjs` reads the same folder the same way.

use serde_json::Value;
use std::path::PathBuf;

const SECTIONS: [&str; 12] = [
    "cases",
    "contexts",
    "refused",
    "results",
    "schemas",
    "settings",
    "pdf",
    "trasf",
    "diffs",
    "losses",
    "values",
    "feedbacks",
];

pub fn golden_files() -> Vec<(PathBuf, Value)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/log");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no golden file in {}", dir.display());
    paths
        .into_iter()
        .map(|p| {
            let fx: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            for key in fx.as_object().unwrap().keys() {
                assert!(
                    SECTIONS.contains(&key.as_str()),
                    "{}: unknown section {key}",
                    p.display()
                );
            }
            (p, fx)
        })
        .collect()
}

pub fn section<'a>(fx: &'a Value, key: &str) -> &'a [Value] {
    fx.get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}
