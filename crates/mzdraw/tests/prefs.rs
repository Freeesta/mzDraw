//! The preferences: one table of options with a group and a text key each, every option a setting (`--set`), a reset per group.

use mzdraw::prefs::{self, Kind, GROUPS};
use mzdraw::settings::FIELDS;
use mzdraw::Editor;
use serde_json::Value;

/// The keys of the page's English catalogue.
fn catalogue() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../web/lang/mzdraw.js"
    ))
    .unwrap()
}

fn schema() -> Value {
    serde_json::from_str(&mzdraw::settings::schema_json()).unwrap()
}

#[test]
fn every_option_has_a_group_and_a_text_key_and_every_setting_is_an_option() {
    let s = schema();
    let prefs = s["prefs"].as_array().unwrap();
    let groups: Vec<&str> = s["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap())
        .collect();
    assert_eq!(groups, GROUPS.map(|g| g.0));
    for p in prefs {
        assert!(groups.contains(&p["group"].as_str().unwrap()), "{p}");
        assert!(
            p["label"].as_str().unwrap().starts_with("draw2.prefs."),
            "{p}"
        );
    }
    let settings: Vec<&str> = prefs
        .iter()
        .map(|p| p["setting"].as_str().unwrap())
        .collect();
    // every number of the style, every word and every display option of the settings file is an option
    for f in &FIELDS {
        let want = if f.key.starts_with("pathway.") {
            f.key.to_string()
        } else {
            format!("style.{}", f.key)
        };
        assert!(settings.contains(&want.as_str()), "{want}");
    }
    for key in [
        "style.colour",
        "style.preset",
        "display.maps",
        "display.values",
        "display.adducts",
        "display.lens",
        "pathway.dir",
        "pathway.order",
        "pathway.number",
        "pathway.derive",
        "export.default",
        "export.background",
        "ui.theme",
    ] {
        assert!(settings.contains(&key), "{key}");
    }
    // each group has options, and no setting is listed twice
    for g in groups {
        assert!(prefs.iter().any(|p| p["group"] == g), "{g}");
    }
    let mut sorted = settings.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), settings.len());
}

#[test]
fn every_text_of_the_panel_is_in_the_catalogue() {
    let cat = catalogue();
    let has = |k: &str| cat.contains(&format!("\"{k}\":"));
    for (_, label) in GROUPS {
        assert!(has(label), "{label}");
    }
    for k in [
        "draw2.prefs",
        "draw2.prefs.title",
        "draw2.fly.prefs",
        "draw2.tool.prefs",
        "draw2.tool.prefs.title",
        "draw2.prefs.hint",
        "draw2.prefs.reset",
        "draw2.prefs.reset.title",
        "draw2.prefs.reset.done",
        "draw2.prefs.changed",
        "draw2.prefs.close",
        "draw2.prefs.close.title",
        "draw2.prefs.language",
    ] {
        assert!(has(k), "{k}");
    }
    for r in prefs::all() {
        assert!(has(&r.label), "{}", r.label);
        if let Kind::Words(w) = r.kind {
            for word in w {
                let k = prefs::word_key(&r.label, word);
                assert!(has(&k), "{k}");
                // no text key carries the product name
                assert!(!k.contains(mzdraw::brand::SLUG), "{k}");
            }
        }
    }
}

#[test]
fn a_setting_round_trips_through_set_and_get_for_every_word_and_number() {
    for r in prefs::all() {
        let mut ed = Editor::default();
        let def = prefs::default_of(&r.setting).unwrap();
        assert_eq!(
            ed.get(&r.setting).as_deref(),
            Some(def.as_str()),
            "{} default",
            r.setting
        );
        let values: Vec<String> = match r.kind {
            Kind::Words(w) => w.iter().map(ToString::to_string).collect(),
            Kind::Adducts => vec!["[M+Na]+,[M+K]+".into(), String::new()],
            Kind::Number => {
                let key = r.setting.strip_prefix("style.").unwrap_or(&r.setting);
                let f = FIELDS.iter().find(|f| f.key == key).unwrap();
                vec![format!("{}", (f.min + f.max) / 2.0)]
            }
        };
        for v in values {
            ed.set(&format!("{}={v}", r.setting))
                .unwrap_or_else(|e| panic!("{} = {v}: {e}", r.setting));
            let got = ed.get(&r.setting).unwrap();
            // the document kind is written under the slug whatever its older spelling
            assert_eq!(got, v, "{}", r.setting);
        }
    }
    assert!(Editor::default().get("style.nothing").is_none());
    assert!(Editor::default().get("ui.language").is_none());
}

#[test]
fn a_reset_puts_a_group_back_and_leaves_the_others() {
    let mut ed = Editor::default();
    for s in [
        "style.colour=mono",
        "display.values=mass",
        "style.bond_length=22",
        "pathway.dir=down",
        "export.default=png",
        "ui.theme=dark",
    ] {
        ed.set(s).unwrap();
    }
    let list = ed.reset_commands("display").unwrap();
    assert_eq!(
        list,
        [
            "style.colour=element",
            "display.maps=on",
            "display.values=off",
            "display.adducts=[M+H]+,[M-H]-",
            "display.lens=off"
        ]
    );
    ed.reset_group("display").unwrap();
    assert_eq!(ed.get("style.colour").unwrap(), "element");
    assert_eq!(ed.get("display.values").unwrap(), "off");
    assert_eq!(ed.get("style.bond_length").unwrap(), "22");
    assert_eq!(ed.get("pathway.dir").unwrap(), "down");
    ed.reset_group("files").unwrap();
    assert_eq!(ed.get("export.default").unwrap(), "svg");
    assert_eq!(ed.get("ui.theme").unwrap(), "dark");
    ed.reset_group("appearance").unwrap();
    assert_eq!(ed.get("ui.theme").unwrap(), "auto");
    // the group Style holds the preset, which replaces the whole style: the options of the other groups keep their values
    ed.set("display.values=formula").unwrap();
    ed.set("style.preset=acs-1996").unwrap();
    ed.set("display.maps=off").unwrap();
    ed.set("pathway.dir=down").unwrap();
    let before = ed.get("style.colour").unwrap();
    assert_eq!(before, "mono");
    ed.reset_group("style").unwrap();
    assert_eq!(ed.get("style.preset").unwrap(), "mzlab");
    assert_eq!(ed.get("style.bond_length").unwrap(), "30");
    assert_eq!(ed.get("style.colour").unwrap(), "mono");
    assert_eq!(ed.get("display.maps").unwrap(), "off");
    assert_eq!(ed.get("pathway.dir").unwrap(), "down");
    assert!(ed.reset_commands("nothing").is_none());
    assert!(ed.reset_group("nothing").is_err());
    // the list a script can send as the `set` field gives the same settings file as the page
    let mut a = Editor::default();
    a.set("style.colour=mono").unwrap();
    a.set("display.values=mass").unwrap();
    let mut b = Editor::default();
    b.set("style.colour=mono").unwrap();
    b.set("display.values=mass").unwrap();
    a.reset_group("display").unwrap();
    for s in b.reset_commands("display").unwrap() {
        b.set(&s).unwrap();
    }
    assert_eq!(a.settings_json(), b.settings_json());
    assert_eq!(a.settings_json(), Editor::default().settings_json());
}

#[test]
fn the_theme_is_kept_in_the_settings_file_with_the_keys_this_version_does_not_know() {
    let mut ed = Editor::default();
    assert!(!ed.settings_json().contains("\"ui\""));
    ed.set("ui.theme=dark").unwrap();
    let file = ed.settings_json();
    assert!(file.contains(r#""ui":{"theme":"dark"}"#), "{file}");
    let mut other = Editor::default();
    other.set_style(&file).unwrap();
    assert_eq!(other.get("ui.theme").unwrap(), "dark");
    assert_eq!(other.settings_json(), file);
    // a newer file: its unknown keys of `ui` survive a round
    let newer =
        r#"{"format":"mzdraw-settings/1","style":{},"ui":{"density":"tight","theme":"light"}}"#;
    other.set_style(newer).unwrap();
    assert_eq!(other.get("ui.theme").unwrap(), "light");
    assert!(
        other
            .settings_json()
            .contains(r#""ui":{"density":"tight","theme":"light"}"#),
        "{}",
        other.settings_json()
    );
    // refused, and nothing changes
    assert_eq!(other.set("ui.theme=sepia"), Err("error.draw2.ui"));
    assert_eq!(other.set("ui.language=it"), Err("error.draw2.set"));
    assert!(other
        .set_style(r#"{"format":"mzdraw-settings/1","ui":{"theme":3}}"#)
        .is_err());
    assert_eq!(other.get("ui.theme").unwrap(), "light");
}

#[test]
fn the_atom_colours_option_changes_the_picture_and_a_reset_brings_it_back() {
    let draw = |sets: &[&str]| {
        let mut ed = Editor::default();
        ed.import("CC(=O)N").unwrap();
        for s in sets {
            ed.set(s).unwrap();
        }
        ed.svg()
    };
    let element = draw(&[]);
    let mono = draw(&["style.colour=mono"]);
    assert_ne!(element, mono);
    assert_eq!(
        draw(&["style.colour=mono", "style.colour=element"]),
        element
    );
}

#[test]
fn a_newer_file_with_unknown_keys_is_read_and_written_back_never_an_error() {
    let newer = r#"{"format":"mzdraw-settings/1","later":{"a":1},"style":{"colour":"mono","future_key":[1,2]},"ui":{"density":"tight","theme":"dark"},"export":{"default":"png","quality":9}}"#;
    let mut ed = Editor::default();
    ed.set_style(newer).unwrap();
    let file = ed.settings_json();
    for part in [
        r#""later":{"a":1}"#,
        r#""future_key":[1,2]"#,
        r#""density":"tight""#,
        r#""quality":9"#,
        r#""colour":"mono""#,
        r#""theme":"dark""#,
    ] {
        assert!(file.contains(part), "{part} in {file}");
    }
    // the known keys act, and a second round gives the same bytes
    assert_eq!(
        (
            ed.get("style.colour").unwrap(),
            ed.get("ui.theme").unwrap(),
            ed.get("export.default").unwrap()
        ),
        ("mono".into(), "dark".into(), "png".into())
    );
    let mut again = Editor::default();
    again.set_style(&file).unwrap();
    assert_eq!(again.settings_json(), file);
    // a file of a higher version is said so, never half read
    assert_eq!(
        Editor::default().set_style(r#"{"format":"mzdraw-settings/2","ui":{"theme":"dark"}}"#),
        Err("error.draw2.styleNewer")
    );
    // the schema says its format version
    assert_eq!(schema()["format"], "mzdraw-settings/1");
}
