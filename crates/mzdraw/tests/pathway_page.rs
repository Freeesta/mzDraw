//! The page's way to the pathway layout (tp-06): `Editor::pathway_tree` (the button "Arrange pathway") and the item `pathway.tree` of the
//! context menu give the bytes of the command `pathway:tree`, in one undo step, with a line that says what was done; a refusal changes
//! nothing and says why; the two settings of the layout are in the schema the page paints. Pathways are the synthetic ones of
//! `tests/data/pathways/`.

use mzdraw::editor::{CHANGED, REPAINT};
use mzdraw::Editor;
use serde_json::{json, Value};

fn text(name: &str) -> String {
    let p = format!(
        "{}/../../tests/data/pathways/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(p).unwrap()
}

fn opened(name: &str) -> Editor {
    let mut e = Editor::default();
    e.open(&text(name)).unwrap();
    e
}

fn replayed(name: &str, out: &str, sets: &[&str]) -> String {
    let req = json!({"import": text(name), "commands": [{"command": "pathway:tree"}], "out": out, "set": sets});
    mzdraw::replay::replay(&req.to_string()).unwrap()
}

#[test]
fn the_button_gives_the_bytes_of_the_command_in_one_undo_step() {
    for (name, sets) in [
        ("messy_star", vec![]),
        ("binary3", vec!["pathway.dir=down"]),
        ("unbalanced", vec![]),
        ("dag", vec!["pathway.order=mz"]),
    ] {
        let mut e = opened(name);
        for s in &sets {
            e.set(s).unwrap();
        }
        let before = e.save("mzdraw").unwrap();
        let flags = e.pathway_tree();
        assert_eq!(flags & (REPAINT | CHANGED), REPAINT | CHANGED, "{name}");
        assert_eq!(e.error, "", "{name}");
        assert_eq!(
            e.save("mzdraw").unwrap(),
            replayed(name, "mzdraw", &sets),
            "{name}"
        );
        assert_eq!(
            e.save("svg").unwrap(),
            replayed(name, "svg", &sets),
            "{name}"
        );
        // the operations are the ones of the command, not those and the follow of the routes a second time
        let log: Value = serde_json::from_str(&e.save("log").unwrap()).unwrap();
        let want: Value = serde_json::from_str(&replayed(name, "log", &sets)).unwrap();
        let tail = |v: &Value| {
            let all = v.as_array().unwrap();
            all[all.len() - 6..].to_vec()
        };
        assert_eq!(tail(&log), tail(&want), "{name}");
        // one undo step
        e.undo();
        assert_eq!(e.save("mzdraw").unwrap(), before, "{name}");
    }
}

#[test]
fn the_line_says_what_was_done_in_numbers() {
    let mut e = opened("star3");
    e.pathway_tree();
    assert_eq!(e.said.0, "draw2.say.pathway");
    assert_eq!(
        e.said_params,
        vec![("structures", 4), ("arrows", 3), ("gaps", 0)]
    );
    // a pathway whose steep arrows widen a gap says how many
    let mut e = opened("mixed");
    e.pathway_tree();
    let lay = mzdraw::pathway::layout(e.doc(), &e.style).unwrap();
    let (n, m) = (lay.nodes.len() as u32, lay.edges.len() as u32);
    assert_eq!(e.said_params[0], ("structures", n));
    assert_eq!(e.said_params[1], ("arrows", m));
    // the numbers are the ones of the findings of the report
    let r = mzdraw::pathway::report_with(e.doc(), &e.style);
    let widened = r
        .checks
        .iter()
        .filter(|f| f.key == "std.scheme.gapRaised")
        .count() as u32;
    let mut again = opened("mixed");
    again.pathway_tree();
    assert_eq!(again.said_params[2], ("gaps", widened));
}

#[test]
fn a_refusal_says_why_and_changes_nothing() {
    let refused: Value = serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/../../tests/data/log/pathway.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    for case in refused["refused"].as_array().unwrap() {
        let mut e = Editor::default();
        e.open(case["request"]["import"].as_str().unwrap()).unwrap();
        let before = e.save("mzdraw").unwrap();
        let steps = e.session.can_undo();
        let flags = e.pathway_tree();
        assert_eq!(flags & CHANGED, 0);
        assert_eq!(e.error, case["error_key"].as_str().unwrap());
        assert_eq!(e.save("mzdraw").unwrap(), before);
        assert_eq!(e.session.can_undo(), steps, "no undo step was made");
    }
    // nothing anchored: the layout has nothing to arrange
    let mut e = Editor::default();
    e.pathway_tree();
    assert_eq!(e.error, "error.draw2.schemeEmpty");
}

#[test]
fn the_context_menu_offers_the_layout_when_an_arrow_is_anchored_at_both_ends() {
    let item = |e: &mut Editor| {
        let m: Value = serde_json::from_str(&e.context_menu(100000.0, 100000.0)).unwrap();
        assert_eq!(m["target"], "canvas");
        m["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == "pathway.tree")
            .cloned()
            .unwrap()
    };
    let mut e = Editor::default();
    assert_eq!(item(&mut e)["enabled"], false, "an empty canvas");
    let mut e = opened("messy_star");
    let it = item(&mut e);
    assert_eq!(it["enabled"], true);
    assert_eq!(it["label"], "Arrange pathway");
    let before = e.save("mzdraw").unwrap();
    assert_eq!(e.context_run("pathway.tree") & CHANGED, CHANGED);
    assert_eq!(
        e.save("mzdraw").unwrap(),
        replayed("messy_star", "mzdraw", &[])
    );
    e.undo();
    assert_eq!(e.save("mzdraw").unwrap(), before);
    // a drawing with an arrow that is anchored at no structure has none to offer
    let mut e = Editor::default();
    e.open(r#"{"format":"mzdraw/1","next":2,"atoms":[],"bonds":[],"objects":[{"id":1,"t":"arrow","x":0,"y":0,"x2":2,"y2":0}]}"#).unwrap();
    assert_eq!(item(&mut e)["enabled"], false);
}

#[test]
fn the_words_of_the_layout_are_in_the_schema_and_set_the_layout() {
    let schema: Value = serde_json::from_str(&mzdraw::settings::schema_json()).unwrap();
    assert_eq!(
        schema["choices"],
        json!([
            {"key": "pathway.dir", "values": ["right", "down"], "default": "right"},
            {"key": "pathway.order", "values": ["id", "mz", "rt"], "default": "id"},
            {"key": "pathway.number", "values": ["layout", "mz", "rt"], "default": "layout"}
        ])
    );
    // every word of the schema is accepted by `set`, and a word outside it is refused with its key
    let mut e = Editor::default();
    for c in schema["choices"].as_array().unwrap() {
        let key = c["key"].as_str().unwrap();
        for v in c["values"].as_array().unwrap() {
            e.set(&format!("{key}={}", v.as_str().unwrap())).unwrap();
        }
        assert_eq!(
            e.set(&format!("{key}=sideways")),
            Err("error.draw2.styleValue")
        );
    }
    // set to the default the settings file leaves the key out; a word that is not the default is written
    let mut e = Editor::default();
    assert!(!e.settings_json().contains("pathway.dir"));
    assert_eq!(e.style.choice("pathway.dir"), Some("right"));
    assert_eq!(e.style.choice("pathway.order"), Some("id"));
    assert_eq!(e.style.choice("colour"), None);
    e.set("pathway.dir=down").unwrap();
    assert_eq!(e.style.choice("pathway.dir"), Some("down"));
    assert!(e.settings_json().contains(r#""pathway.dir":"down""#));
    e.set("pathway.dir=right").unwrap();
    assert!(!e.settings_json().contains("pathway.dir"));
}
