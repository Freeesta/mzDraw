//! The golden files of the editor: every `*.json` of `tests/data/log/`, one file per area, read in the order of their names.
//! A file holds some of the sections `cases`, `contexts`, `refused`, `results`, `schemas`, `settings`, `pdf`, `trasf`, `diffs`, `losses`, `values`, `feedbacks`, `histories` (a missing section is an empty one);
//! `tests/parity.mjs` reads the same folder the same way.

#![allow(dead_code)] // each test file uses the part it needs

use serde_json::Value;
use std::path::PathBuf;

const SECTIONS: [&str; 13] = [
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
    "histories",
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

/// A point a drawing occupies: an atom or an object anchor (`sample` false), or a sample along a bond or an arrow (`sample` true).
#[derive(Clone, Copy, Debug)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
    pub sample: bool,
}

fn along(out: &mut Vec<Pt>, a: (f64, f64), b: (f64, f64)) {
    let n = ((b.0 - a.0).hypot(b.1 - a.1) / 0.05).ceil().max(1.0) as usize;
    for k in 1..n {
        let t = k as f64 / n as f64;
        out.push(Pt {
            x: a.0 + (b.0 - a.0) * t,
            y: a.1 + (b.1 - a.1) * t,
            sample: true,
        });
    }
}

/// What a drawing occupies, sampled every 0.05 bond along bonds and arrows: the atoms, bonds and objects whose id is above `above`
/// (0 for everything). An independent reading of the contract of a paste: it does not use the code that places.
pub fn occupied(d: &mzdraw::Document, above: u32) -> Vec<Pt> {
    let mut v = Vec::new();
    for a in d.atoms.iter().filter(|a| a.id > above) {
        v.push(Pt {
            x: a.x,
            y: a.y,
            sample: false,
        });
    }
    for b in d.bonds.iter().filter(|b| b.a > above && b.b > above) {
        let (p, q) = (d.atom(b.a).unwrap(), d.atom(b.b).unwrap());
        along(&mut v, (p.x, p.y), (q.x, q.y));
    }
    for o in d.objects.iter().filter(|o| o.id > above) {
        v.push(Pt {
            x: o.x,
            y: o.y,
            sample: false,
        });
        if o.t == mzdraw::ObjKind::Arrow {
            v.push(Pt {
                x: o.x2,
                y: o.y2,
                sample: false,
            });
            along(&mut v, (o.x, o.y), (o.x2, o.y2));
        }
    }
    v
}

/// The contract of a paste: no point of the pasted part lies within half a bond (a quarter for a sample along a bond or an arrow, the
/// grain the placement looks at) of a point of the old drawing, in either direction of either axis.
pub fn assert_clear(old: &[Pt], new: &[Pt]) {
    assert!(!new.is_empty(), "nothing was pasted");
    for p in new {
        for q in old {
            let margin = if q.sample { 0.25 } else { 0.5 } - 1e-3;
            assert!(
                (p.x - q.x).abs() >= margin || (p.y - q.y).abs() >= margin,
                "{p:?} lies on {q:?}"
            );
        }
    }
}
