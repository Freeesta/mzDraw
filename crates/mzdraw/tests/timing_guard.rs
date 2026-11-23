//! A guard against new absolute wall-clock assertions in the tests (`docs/built/flaky-tests-2.md`).
//!
//! A bound in milliseconds fails at random on a loaded runner, and the CPU time of a thread is inflated there too. A test of cost
//! compares two sizes (a ratio) with `support/timing.rs`. This scan catches the two ways such a test comes back; it cannot tell a
//! scaling claim from a size bound, so the files that still hold a wall-clock bound are listed, each with its reason.

use std::path::Path;

/// The test files that still measure on the wall clock (`Instant::now`), and why that is not a scaling claim: each bound is in whole
/// seconds, about two orders of magnitude over the real time, or is a refusal that must come before the work.
const WALL_CLOCK: &[(&str, &str)] = &[
    ("cdx.rs", "bounds of seconds on hostile inputs"),
    ("lens.rs", "one bound of seconds on a size at the cap"),
    (
        "loss.rs",
        "bounds of seconds on hostile inputs, refusals at once",
    ),
    (
        "orient_limits.rs",
        "bounds of seconds to minutes on hostile inputs",
    ),
    (
        "pathway.rs",
        "refusals and a printed time, no assertion on a scaling claim",
    ),
    ("pathway_tools.rs", "bounds of seconds on 2000 structures"),
    ("tp.rs", "one bound of seconds on a size at the cap"),
    ("tp_list.rs", "bounds of seconds on hostile inputs"),
    ("tptable.rs", "a refusal before the work"),
];

fn test_files() -> Vec<(String, String)> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut out = Vec::new();
    for c in std::fs::read_dir(&crates).unwrap().flatten() {
        let Ok(rd) = std::fs::read_dir(c.path().join("tests")) else {
            continue;
        };
        for f in rd.flatten() {
            let p = f.path();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            if p.extension().is_some_and(|e| e == "rs") && name != "timing_guard.rs" {
                let crate_name = c.file_name().to_string_lossy().into_owned();
                out.push((
                    format!("{crate_name}/{name}"),
                    std::fs::read_to_string(&p).unwrap(),
                ));
            }
        }
    }
    out
}

#[test]
fn no_test_asserts_a_bound_in_milliseconds() {
    // `as_millis()` is how a figure in milliseconds is made to compare with a number; a cost is a ratio of two runs instead
    let bad: Vec<String> = test_files()
        .iter()
        .flat_map(|(f, text)| {
            text.lines()
                .enumerate()
                .filter(|(_, l)| l.contains("as_millis()") && !l.trim_start().starts_with("//"))
                .map(|(i, l)| format!("{f}:{}: {}", i + 1, l.trim()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(bad.is_empty(), "absolute bounds in milliseconds: {bad:#?}");
}

#[test]
fn a_new_wall_clock_test_is_listed_with_its_reason_or_uses_the_cpu_clock() {
    let mut found: Vec<String> = test_files()
        .iter()
        .filter(|(_, t)| t.contains("Instant::now"))
        .map(|(f, _)| f.rsplit('/').next().unwrap().to_string())
        .collect();
    found.sort();
    let mut listed: Vec<String> = WALL_CLOCK.iter().map(|(f, _)| f.to_string()).collect();
    listed.sort();
    assert_eq!(
        found, listed,
        "a test file that reads `Instant::now` must be in WALL_CLOCK (with its reason) or use timing::Watch; a listed file that no \
         longer does must be taken off"
    );
}
