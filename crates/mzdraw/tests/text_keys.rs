//! Every error key, import-note key and layout-note key the code writes has a text in the English catalog of the page (a missing one would show as
//! a raw key).

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// The finding keys of the pathway checks (`pathway.*`, written by `pathcheck.rs` only: other files use `pathway.` for settings) have a text.
#[test]
fn every_finding_key_of_the_pathway_check_is_in_the_english_texts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lang = std::fs::read_to_string(root.join("web/lang/mzdraw.js")).unwrap();
    let code = std::fs::read_to_string(root.join("crates/mzdraw/src/pathcheck.rs")).unwrap();
    let (mut seen, mut missing) = (0, Vec::new());
    for (i, _) in code.match_indices("\"pathway.") {
        let key: String = code[i + 1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '.')
            .collect();
        if code[i + 1 + key.len()..].starts_with('"') {
            seen += 1;
            if !lang.contains(&format!("\"{key}\":")) {
                missing.push(key);
            }
        }
    }
    assert!(seen >= 15, "{seen}");
    assert!(missing.is_empty(), "no English text for: {missing:?}");
}

#[test]
fn every_error_and_import_key_of_the_code_is_in_the_english_texts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lang = std::fs::read_to_string(root.join("web/lang/mzdraw.js")).unwrap();
    let mut files = Vec::new();
    walk(&root.join("crates/mzdraw/src"), &mut files);
    walk(&root.join("crates/mzdraw-wasm/src"), &mut files);
    let mut missing = std::collections::BTreeSet::new();
    let mut seen = 0;
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        for start in ["\"error.draw2.", "\"draw2.import.", "\"draw2.layout."] {
            for (i, _) in text.match_indices(start) {
                let key: String = text[i + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '.')
                    .collect();
                // the end of a quoted key only: a prefix the code completes is not a key
                if text[i + 1 + key.len()..].starts_with('"') && !key.ends_with('.') {
                    seen += 1;
                    if !lang.contains(&format!("\"{key}\":")) {
                        missing.insert(key);
                    }
                }
            }
        }
    }
    assert!(seen > 100, "{seen}");
    assert!(missing.is_empty(), "no English text for: {missing:?}");
}
