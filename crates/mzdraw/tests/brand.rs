//! The product name lives in `brand.toml` and nowhere else. This test holds the rest of the repository to it:
//! - `brand.toml` and `src/brand.rs` say the same (every key is checked, a key nobody reads is refused);
//! - `web/names.js` and `web/appname.js`, the only places where the page learns the name, are written from `brand.rs`
//!   (`MZDRAW_BLESS=1 cargo test -p mzdraw --test brand` rewrites them);
//! - the manifests of the crates, `CITATION.cff` and the Pages path follow it;
//! - `brand_no_literals`: the codename is written in no string of the Rust sources of the crates and in no text or string of the
//!   page (`web/`), except the few entries of `ALLOWED`, each with its reason; an entry that no longer matches fails too.
//!
//! Not covered here, on purpose: Rust identifiers and paths (the crates are named by `crate_prefix`), the manifests, the tests,
//! `tools/`, `docs/` and the names of files. They are moved together by the rename tool (`tools/rename_product.sh`, next PR).

use mzdraw::brand;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

// ---- brand.toml: the small subset it uses (sections, `key = "text"`, `key = ["a", "b"]`, `#` comments), no dependency

#[derive(Debug, PartialEq)]
enum Val {
    Text(String),
    List(Vec<String>),
}

fn toml(text: &str) -> BTreeMap<String, Val> {
    let (mut out, mut section) = (BTreeMap::new(), String::new());
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_string();
            continue;
        }
        let (key, value) = line.split_once('=').expect("key = value");
        let value = value.trim();
        let quoted = |s: &str| {
            let s = s.trim().strip_prefix('"').expect("a quoted text");
            s[..s.find('"').expect("a closing quote")].to_string()
        };
        let val = if let Some(list) = value.strip_prefix('[') {
            let list = &list[..list.find(']').expect("a closing bracket")];
            Val::List(
                list.split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(quoted)
                    .collect(),
            )
        } else {
            Val::Text(quoted(value))
        };
        assert!(
            out.insert(format!("{section}.{}", key.trim()), val)
                .is_none(),
            "{line}: twice"
        );
    }
    out
}

fn brand_toml() -> BTreeMap<String, Val> {
    toml(&read("brand.toml"))
}

fn text(t: &BTreeMap<String, Val>, key: &str) -> String {
    match t.get(key) {
        Some(Val::Text(s)) => s.clone(),
        other => panic!("brand.toml: {key} is {other:?}"),
    }
}

#[test]
fn brand_toml_and_brand_rs_say_the_same() {
    let t = brand_toml();
    let texts = [
        ("product.name", brand::NAME),
        ("product.slug", brand::SLUG),
        ("product.crate_prefix", brand::CRATE_PREFIX),
        ("product.component_tag", brand::COMPONENT_TAG),
        ("product.css_prefix", brand::CSS_PREFIX),
        ("product.app_id", brand::APP_ID),
        ("formats.document", brand::DOCUMENT_ID),
        ("formats.settings", brand::SETTINGS_ID),
        ("formats.edit", brand::EDIT_ID),
        ("formats.bridge", brand::BRIDGE_ID),
        ("formats.first_document", brand::FIRST_DOCUMENT_ID),
        ("formats.schema_key", brand::SCHEMA_KEY),
        ("site.repository", brand::REPOSITORY),
        ("site.pages_url", brand::PAGES_URL),
        ("site.pages_base", brand::PAGES_BASE),
        ("citation.title", brand::CITATION_TITLE),
    ];
    for (key, constant) in texts {
        assert_eq!(
            text(&t, key),
            constant,
            "brand.toml {key} and brand.rs differ"
        );
    }
    let legacy: Vec<String> = brand::LEGACY_IDS.iter().map(|s| s.to_string()).collect();
    assert_eq!(t["formats.legacy_ids"], Val::List(legacy), "legacy_ids");
    assert_eq!(
        t.len(),
        texts.len() + 1,
        "a key of brand.toml that this test does not read: {t:?}"
    );
}

#[test]
fn the_ids_follow_the_slug_and_the_pages_path_follows_the_repository() {
    let (slug, version) = brand::DOCUMENT_ID.split_once('/').unwrap();
    assert_eq!(slug, brand::SLUG);
    assert!(version.parse::<u32>().is_ok());
    assert!(brand::SETTINGS_ID.starts_with(&format!("{}-settings/", brand::SLUG)));
    assert!(
        brand::COMPONENT_TAG.contains('-'),
        "a custom element needs a hyphen"
    );
    // the address of the page is case-sensitive and follows the repository name
    let name = brand::REPOSITORY.split_once('/').unwrap().1;
    assert_eq!(brand::PAGES_BASE, format!("/{name}/"));
    assert!(brand::PAGES_URL.ends_with(brand::PAGES_BASE));
    assert!(brand::LEGACY_IDS
        .iter()
        .all(|s| !s.is_empty() && *s != brand::SLUG && !s.contains(['/', ' '])));
}

#[test]
fn the_crates_the_citation_and_the_tokens_follow_brand_toml() {
    for (dir, name) in [
        (
            brand::CRATE_PREFIX.to_string(),
            brand::CRATE_PREFIX.to_string(),
        ),
        (
            format!("{}-wasm", brand::CRATE_PREFIX),
            format!("{}-wasm", brand::CRATE_PREFIX),
        ),
    ] {
        let manifest = read(&format!("crates/{dir}/Cargo.toml"));
        assert!(
            manifest.lines().any(|l| l == format!("name = \"{name}\"")),
            "crates/{dir}: the package is called {name}"
        );
    }
    let cargo = read("Cargo.toml");
    let repository = format!("repository = \"https://github.com/{}\"", brand::REPOSITORY);
    assert!(
        cargo.to_lowercase().contains(&repository.to_lowercase()),
        "the workspace repository is {}",
        brand::REPOSITORY
    );
    let cff = read("CITATION.cff");
    let line = |key: &str| {
        cff.lines()
            .find_map(|l| l.strip_prefix(key))
            .unwrap_or_else(|| panic!("CITATION.cff: {key}"))
            .trim()
            .trim_matches('"')
            .to_string()
    };
    assert_eq!(line("title:"), brand::CITATION_TITLE);
    assert_eq!(
        line("repository-code:"),
        format!("https://github.com/{}", brand::REPOSITORY)
    );
    assert_eq!(line("url:"), brand::PAGES_URL);
    assert_eq!(
        line("version:"),
        env!("CARGO_PKG_VERSION"),
        "rule 14: the version of the citation is the workspace version"
    );
    assert!(
        read("web/mzdraw.css").contains(&format!("--{}-", brand::CSS_PREFIX)),
        "the colour tokens carry the css prefix"
    );
}

// ---- web/names.js and web/appname.js: written from brand.rs

fn list(names: impl Iterator<Item = String>) -> String {
    format!(
        "[{}]",
        names
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn names_js() -> String {
    names_js_with(brand::LEGACY_IDS)
}

/// `web/names.js` as it is written when the earlier slugs are `legacy` (a rename in the future).
fn names_js_with(legacy: &[&str]) -> String {
    use brand::*;
    let old = |f: &dyn Fn(&str) -> String| list(legacy.iter().map(|s| f(s)));
    let tokens = format!("--{CSS_PREFIX}-");
    let wasm = format!("{CRATE_PREFIX}_wasm");
    [
        ("@NAME@", NAME),
        ("@SLUG@", SLUG),
        ("@DOC@", DOCUMENT_ID),
        ("@BRIDGE@", BRIDGE_ID),
        ("@APP@", APP_ID),
        ("@TAG@", COMPONENT_TAG),
        ("@WASM@", &wasm),
        ("@TOKENS@", &tokens),
        ("@AUTOSAVE_OLD@", &old(&|s| format!("{s}-autosave.json"))),
        ("@SETTINGS_OLD@", &old(&|s| format!("{s}-settings.json"))),
        ("@CLIP_OLD@", &old(&|s| format!("application/x-{s}+json"))),
    ]
    .iter()
    .fold(NAMES_JS.to_string(), |t, (k, v)| t.replace(k, v))
}

const NAMES_JS: &str = r#"// GENERATED from brand.toml by crates/mzdraw/tests/brand.rs (MZDRAW_BLESS=1 cargo test -p mzdraw --test brand): do not edit.
// Every name of the product the page uses, in one place: the name will change. The ids of the page carry the prefix `d2-`
// (HTML, CSS and tests), the i18n keys `draw2.`; what the scripts write or store is named here only. The `*Old` lists are the
// names of earlier products: read when the current one is not there, never written. (The comments are above the lines, not after
// them, so that the file does not depend on the length of a name.)
export const NAMES = Object.freeze({
  // the visible name (the texts say {app}, see appname.js)
  name: "@NAME@",
  // the `out` kind of the document
  slug: "@SLUG@",
  // the document format of the day
  version: "@DOC@",
  // what the module must speak to this page
  bridge: "@BRIDGE@",
  // the editor module on window, for hosts and the tests
  global: "@APP@",
  // the custom element
  tag: "@TAG@",
  // the module of pkg/
  wasm: "@WASM@",
  // the drawing kept in the browser's private file system (OPFS), then the names of earlier products
  autosave: "@SLUG@-autosave.json",
  autosaveOld: @AUTOSAVE_OLD@,
  // the settings file kept there, and its download name
  settings: "@SLUG@-settings.json",
  settingsOld: @SETTINGS_OLD@,
  // our own clipboard type: the document
  clip: "application/x-@SLUG@+json",
  clipOld: @CLIP_OLD@,
  // the download name of the document
  doc: "drawing.@SLUG@.json",
  // the prefix of the ids the scripts make
  id: "d2-",
  // the prefix of the colour tokens (web/mzdraw.css), which a host may set
  tokens: "@TOKENS@",
});
"#;

fn appname_js() -> String {
    format!(
        "// GENERATED from brand.toml by crates/mzdraw/tests/brand.rs (MZDRAW_BLESS=1 cargo test -p mzdraw --test brand): do not edit.\n\
// The visible name of the program, a classic script in the head because i18n.js reads it before any module runs: the texts say\n\
// {{app}}. The title is set here, at once, so that the tab is never blank; i18n.js writes the same text from the catalog.\n\
window.APP_NAME = \"{}\";\n\
document.title = window.APP_NAME;\n",
        brand::NAME
    )
}

#[test]
fn the_page_learns_the_name_from_files_written_from_brand_rs() {
    for (path, want) in [
        ("web/names.js", names_js()),
        ("web/appname.js", appname_js()),
    ] {
        if std::env::var("MZDRAW_BLESS").is_ok() {
            fs::write(root().join(path), &want).unwrap();
        }
        assert_eq!(
            read(path),
            want,
            "{path} differs from brand.rs: MZDRAW_BLESS=1 cargo test -p mzdraw --test brand"
        );
    }
}

#[test]
fn the_schema_key_is_the_first_name_whatever_the_name_in_use() {
    // `out: schema` says for each operation whether this editor reads it, under a stable key for agents: the first name, never renamed
    let json = mzdraw::schema::json();
    let key = format!("\"{}\":true", brand::SCHEMA_KEY);
    assert!(json.contains(&key), "{key}");
    assert!(json.contains(&format!("\"since\":\"{}\"", brand::FIRST_DOCUMENT_ID)));
    if brand::SLUG != brand::SCHEMA_KEY {
        // a renamed tree: the name in use is not a key of the schema
        assert!(!json.contains(&format!("\"{}\":", brand::SLUG)));
    }
}

#[test]
fn the_files_of_the_page_list_the_earlier_names_when_there_are_some() {
    // a rename in the future: two earlier slugs, oldest first
    let js = names_js_with(&["first", "second"]);
    for (key, want) in [
        (
            "autosaveOld",
            r#"["first-autosave.json", "second-autosave.json"]"#,
        ),
        (
            "settingsOld",
            r#"["first-settings.json", "second-settings.json"]"#,
        ),
        (
            "clipOld",
            r#"["application/x-first+json", "application/x-second+json"]"#,
        ),
    ] {
        assert!(js.contains(&format!("  {key}: {want},\n")), "{key}:\n{js}");
    }
    // the current names are written once, from the slug in use, and none of the earlier ones leaks into them
    assert!(js.contains(&format!("autosave: \"{}-autosave.json\"", brand::SLUG)));
    // with no earlier name the lists are empty; the file of the page has the lists of the earlier names there are
    assert!(names_js_with(&[]).contains("  autosaveOld: [],\n"));
    assert_eq!(
        names_js().contains("  autosaveOld: [],\n"),
        brand::LEGACY_IDS.is_empty()
    );
}

// ---- brand_no_literals

/// The string literals of a source text, with the line of each: comments are skipped. `js` is JavaScript (three kinds of quote,
/// no lifetimes), else Rust (raw strings, char literals and lifetimes).
fn literals(src: &str, js: bool) -> Vec<(usize, String)> {
    let c: Vec<char> = src.chars().collect();
    let (mut i, mut line, mut out) = (0, 1, Vec::new());
    let eat = |from: usize, close: &[char], esc: bool| -> usize {
        let mut j = from;
        while j < c.len() && !c[j..].starts_with(close) {
            j += if esc && c[j] == '\\' { 2 } else { 1 };
        }
        j.min(c.len())
    };
    while i < c.len() {
        let rest = &c[i..];
        if c[i] == '\n' {
            line += 1;
            i += 1;
        } else if rest.starts_with(&['/', '/']) {
            i = eat(i, &['\n'], false);
        } else if rest.starts_with(&['/', '*']) {
            let end = (eat(i + 2, &['*', '/'], false) + 2).min(c.len());
            line += c[i..end].iter().filter(|&&x| x == '\n').count();
            i = end;
        } else if !js
            && c[i] == 'r'
            && (i == 0 || !(c[i - 1].is_alphanumeric() || c[i - 1] == '_'))
            && {
                let hashes = rest[1..].iter().take_while(|&&x| x == '#').count();
                rest.get(1 + hashes) == Some(&'"')
            }
        {
            let hashes = rest[1..].iter().take_while(|&&x| x == '#').count();
            let close: Vec<char> = std::iter::once('"')
                .chain(std::iter::repeat_n('#', hashes))
                .collect();
            let from = i + 2 + hashes;
            let end = eat(from, &close, false);
            let lit: String = c[from..end].iter().collect();
            out.push((line, lit.clone()));
            line += lit.matches('\n').count();
            i = end + close.len();
        } else if c[i] == '"' || (js && (c[i] == '\'' || c[i] == '`')) {
            let end = eat(i + 1, &[c[i]], true);
            let lit: String = c[i + 1..end].iter().collect();
            out.push((line, lit.clone()));
            line += lit.matches('\n').count();
            i = end + 1;
        } else if !js && c[i] == '\'' {
            // a char literal ('x', '\n', '"') is skipped whole; anything else is a lifetime
            i += match (c.get(i + 1), c.get(i + 2)) {
                (Some('\\'), _) => eat(i + 2, &['\''], true) + 1 - i,
                (Some(_), Some('\'')) => 3,
                _ => 1,
            };
        } else {
            i += 1;
        }
    }
    out
}

/// The text of an HTML file without its comments, as (line, text of the line).
fn html_lines(src: &str) -> Vec<(usize, String)> {
    let (mut out, mut open) = (Vec::new(), false);
    for (n, l) in src.lines().enumerate() {
        let (mut keep, mut rest) = (String::new(), l);
        loop {
            if open {
                match rest.find("-->") {
                    Some(e) => (open, rest) = (false, &rest[e + 3..]),
                    None => break,
                }
            } else {
                match rest.find("<!--") {
                    Some(s) => (open, keep, rest) = (true, keep + &rest[..s], &rest[s + 4..]),
                    None => {
                        keep += rest;
                        break;
                    }
                }
            }
        }
        out.push((n + 1, keep));
    }
    out
}

/// Where the codename may stay for now: (file, text of the occurrence, why). Every entry must match something.
const ALLOWED: [(&str, &str, &str); 3] = [
    (
        "web/index.html",
        "href=\"mzdraw.css\"",
        "the name of a file of the page: moved by the rename tool",
    ),
    (
        "web/index.html",
        "src=\"lang/mzdraw.js\"",
        "the name of a file of the page: moved by the rename tool",
    ),
    (
        "web/index.html",
        "src=\"mzdraw.js\"",
        "the name of a file of the page: moved by the rename tool",
    ),
];

/// Whether a text writes the codename (the slug of the name in use), in any case.
fn mentions(text: &str) -> bool {
    text.to_lowercase().contains(brand::SLUG)
}

/// The places of a file where the codename is written, as (line, what), the allowed ones taken out (they are counted in `used`).
fn found(path: &str, hits: Vec<(usize, String)>, used: &mut [bool]) -> Vec<String> {
    let mut bad = Vec::new();
    for (line, mut text) in hits.into_iter().filter(|h| mentions(&h.1)) {
        for (k, (file, needle, _)) in ALLOWED.iter().enumerate() {
            if *file == path && text.contains(needle) {
                used[k] = true;
                text = text.replace(needle, "");
            }
        }
        if mentions(&text) {
            bad.push(format!(
                "{path}:{line}: {}",
                text.trim().chars().take(100).collect::<String>()
            ));
        }
    }
    bad
}

fn files(dir: &str, ext: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root().join(dir)];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().map(Result::unwrap) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == ext) {
                out.push(
                    p.strip_prefix(root().join(""))
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    out.sort();
    out
}

#[test]
fn brand_no_literals() {
    let (mut used, mut bad) = ([false; ALLOWED.len()], Vec::new());
    let sources: Vec<String> = [
        format!("crates/{}/src", brand::CRATE_PREFIX),
        format!("crates/{}-wasm/src", brand::CRATE_PREFIX),
    ]
    .iter()
    .flat_map(|d| files(d, "rs"))
    .filter(|f| !f.ends_with("/brand.rs"))
    .collect();
    assert!(sources.len() > 30, "the Rust sources were found");
    for f in &sources {
        bad.extend(found(f, literals(&read(f), false), &mut used));
    }
    // the page: the strings of the scripts (but not the two files written from brand.rs), the texts of the catalog, the HTML text
    let scripts: Vec<String> = files("web", "js")
        .into_iter()
        .filter(|f| f != "web/names.js" && f != "web/appname.js")
        .collect();
    assert!(scripts.len() > 5, "the scripts of the page were found");
    for f in &scripts {
        bad.extend(found(f, literals(&read(f), true), &mut used));
    }
    // every page and every catalog file of the page: the text of an HTML file without its comments, a JSON file line by line
    let (pages, catalogs) = (files("web", "html"), files("web", "json"));
    assert!(
        pages.contains(&"web/index.html".to_string()),
        "the page was found"
    );
    for f in &pages {
        bad.extend(found(f, html_lines(&read(f)), &mut used));
    }
    for f in &catalogs {
        let lines = read(f)
            .lines()
            .enumerate()
            .map(|(n, l)| (n + 1, l.to_string()))
            .collect();
        bad.extend(found(f, lines, &mut used));
    }
    // the style sheets: their strings (the comments of a sheet may name the product)
    for f in files("web", "css") {
        bad.extend(found(&f, literals(&read(&f), true), &mut used));
    }
    assert!(bad.is_empty(), "the codename is written outside brand.toml (say `{{app}}` in a text, read `NAMES` in a script, use `brand::` in Rust):\n{}", bad.join("\n"));
    for (k, u) in used.iter().enumerate() {
        assert!(
            u,
            "an entry of ALLOWED matches nothing any more, remove it: {:?}",
            ALLOWED[k]
        );
    }
}

#[test]
fn the_scanner_finds_a_name_in_a_string_and_not_in_a_comment() {
    let rust = "let a = \"One\"; // Two \"Three\"\n/* Four \"Five\" */ let b = r#\"Six \" \"seven\"#; let c = '\"'; fn f<'a>(x: &'a str) -> &'a str { \"Eight\" }\n";
    let got: Vec<(usize, String)> = literals(rust, false);
    let words: Vec<&str> = got.iter().map(|g| g.1.as_str()).collect();
    assert_eq!(words, ["One", "Six \" \"seven", "Eight"]);
    assert_eq!(got[2].0, 2);
    let js =
        "const a = 'One'; // \"Two\"\nconst b = `Three ${x}`, c = \"Fo\\\"ur\"; /* 'Five' */\n";
    let words: Vec<String> = literals(js, true).into_iter().map(|g| g.1).collect();
    assert_eq!(words, ["One", "Three ${x}", "Fo\\\"ur"]);
    // the codename in a string is found, in a comment it is not, and an allowed place is taken out
    let (slug, name) = (brand::SLUG, brand::NAME.to_uppercase());
    let mut used = [false; ALLOWED.len()];
    let one = literals(&format!("let a = \"{name}/1\"; // {slug}"), false);
    assert_eq!(found("x.rs", one, &mut used).len(), 1);
    assert!(found(
        "x.rs",
        literals(&format!("// {slug}\nlet a = 1;"), false),
        &mut used
    )
    .is_empty());
    let html = format!("<!-- {slug} -->\n<link href=\"{slug}.css\">");
    assert!(found("web/index.html", html_lines(&html), &mut used).is_empty());
    assert!(used[0] && !used[1]);
    let h1 = format!("<h1>{}</h1>", brand::NAME);
    assert_eq!(found("web/index.html", html_lines(&h1), &mut used).len(), 1);
}
