//! The report of a bug or an improvement (`feedback/1`): the URL escaping, the log that is off by default and the limit of 6000 bytes.

use mzdraw::feedback::{build, encode, Params, LIMIT};
use mzdraw::replay::replay;
use serde_json::Value;

fn params<'a>(kind: &'a str, with_log: bool, log: &'a str) -> Params<'a> {
    Params {
        kind,
        with_log,
        log,
        agent: "Agent/1.0 (X11; Linux)",
        lang: "en-US",
    }
}

fn query(url: &str, key: &str) -> Option<String> {
    let q = url.split_once('?')?.1;
    let v = q
        .split('&')
        .find_map(|kv| kv.strip_prefix(&format!("{key}=")))?;
    let mut bytes = Vec::new();
    let b = v.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            bytes.push(u8::from_str_radix(&v[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            bytes.push(b[i]);
            i += 1;
        }
    }
    Some(String::from_utf8(bytes).unwrap())
}

#[test]
fn escaping_keeps_every_special_character_out_of_the_query() {
    assert_eq!(encode("a&b#c=d+e f"), "a%26b%23c%3Dd%2Be%20f");
    assert_eq!(encode("l1\nl2\r\n"), "l1%0Al2%0D%0A");
    assert_eq!(encode("é→日"), "%C3%A9%E2%86%92%E6%97%A5");
    assert_eq!(encode("A-z_0.9~"), "A-z_0.9~");
    // the whole report round-trips through the query, whatever the browser string holds
    let p = Params {
        kind: "bug",
        with_log: false,
        log: "",
        agent: "Ünï/1 &x=1#y\n+",
        lang: "it-IT",
    };
    let f = build(&p).unwrap();
    assert!(!f.url.contains(['#', ' ', '\n']) && f.url.matches('&').count() == 2);
    let page = query(&f.url, "page").unwrap();
    assert!(page.contains("Browser: Ünï/1 &x=1#y +") && page.contains("Language: it-IT"));
    assert!(page.contains(&format!("Version: {}", mzdraw::VERSION)));
}

#[test]
fn the_form_title_and_labels_follow_the_kind() {
    let bug = build(&params("bug", false, "[]")).unwrap();
    assert_eq!((bug.template, bug.title.as_str()), ("bug.yml", "Bug:"));
    assert!(bug.url.starts_with(
        "https://github.com/Freeesta/mzDraw/issues/new?template=bug.yml&title=Bug%3A&page="
    ));
    let imp = build(&params("improvement", false, "[]")).unwrap();
    assert_eq!(imp.template, "improvement.yml");
    assert_eq!(
        build(&params("question", false, "")).err(),
        Some("error.draw2.feedback")
    );
}

#[test]
fn the_log_is_off_by_default_and_never_silent() {
    let log = r#"[{"op":"addAtom","el":"C"}]"#;
    let off = build(&params("bug", false, log)).unwrap();
    assert!(
        !off.url.contains("log=")
            && !off.body.contains("addAtom")
            && !off.truncated
            && off.notes.is_empty()
    );
    let on = build(&params("bug", true, log)).unwrap();
    assert_eq!(query(&on.url, "log").as_deref(), Some(log));
    assert!(on.body.contains(log) && !on.truncated && on.notes.is_empty());
    // asked for, and there is none: said with a key
    let none = build(&params("bug", true, "[]")).unwrap();
    assert!(!none.url.contains("log=") && none.truncated && none.notes == ["feedback.noLog"]);
}

#[test]
fn a_log_that_makes_the_url_longer_than_the_limit_is_left_out_with_a_note() {
    let fits = format!("[{}]", "1,".repeat(1000).trim_end_matches(','));
    let ok = build(&params("bug", true, &fits)).unwrap();
    assert!(ok.url.len() <= LIMIT && !ok.truncated);
    let long = format!("[{}]", "1,".repeat(3000).trim_end_matches(','));
    let f = build(&params("bug", true, &long)).unwrap();
    assert!(f.truncated && f.notes == ["feedback.logTooLong"]);
    assert!(f.url.len() <= LIMIT && !f.url.contains("log=") && !f.body.contains("1,1,1"));
    // the limit is on the URL as sent: the line breaks of a log count three bytes each
    let nl = format!("[{}]", "\n".repeat(2100));
    assert!(build(&params("bug", true, &nl)).unwrap().truncated);
}

#[test]
fn the_request_field_gives_the_same_bytes_as_the_page_call() {
    let r = |v: &str| replay(v).map_err(|e| e.key);
    let log = r#"[{"el":"N","op":"atom.add","x":0,"y":0}]"#;
    let text = r(&format!(
        r#"{{"log":{log},"feedback":{{"kind":"bug","log":true,"agent":"A","lang":"en"}}}}"#
    ))
    .unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["format"], "feedback/1");
    assert_eq!(v["template"], "bug.yml");
    assert_eq!(
        r(&format!(
            r#"{{"log":{log},"feedback":{{"kind":"bug","log":true,"agent":"A","lang":"en"}}}}"#
        ))
        .unwrap(),
        text
    );
    // the shorthand has no log
    let s: Value = serde_json::from_str(&r(r#"{"feedback":"improvement"}"#).unwrap()).unwrap();
    assert_eq!(
        (s["template"].as_str(), s["truncated"].as_bool()),
        (Some("improvement.yml"), Some(false))
    );
    assert_eq!(
        r(r#"{"feedback":"bug","out":"svg"}"#),
        Err("error.draw2.feedbackAlone")
    );
    assert_eq!(
        r(r#"{"feedback":"bug","report":true}"#),
        Err("error.draw2.feedbackAlone")
    );
    assert_eq!(
        r(r#"{"feedback":"bug","values":true}"#),
        Err("error.draw2.feedbackAlone")
    );
    assert_eq!(
        r(r#"{"feedback":"bug","id":7}"#),
        Err("error.draw2.feedbackAlone")
    );
    assert_eq!(
        r(r#"{"feedback":"bug","propose":[]}"#),
        Err("error.draw2.feedbackAlone")
    );
    assert_eq!(
        r(r#"{"feedback":"bug","dry_run":true}"#),
        Err("error.draw2.feedbackAlone")
    );
    assert_eq!(
        r(r#"{"feedback":"bug","expect":[]}"#),
        Err("error.draw2.feedbackAlone")
    );
    assert_eq!(r(r#"{"feedback":"nope"}"#), Err("error.draw2.feedback"));
    assert_eq!(
        r(r#"{"feedback":{"kind":"bug","x":1}}"#),
        Err("error.draw2.feedback")
    );
    assert_eq!(
        r(r#"{"feedback":{"kind":"bug","log":"yes"}}"#),
        Err("error.draw2.feedback")
    );
}

#[test]
fn about_says_the_name_the_version_and_the_links() {
    let v: Value = serde_json::from_str(&mzdraw::feedback::about_json()).unwrap();
    assert_eq!(v["name"], mzdraw::NAME);
    assert_eq!(v["version"], mzdraw::VERSION);
    assert!(v["licence"]
        .as_str()
        .unwrap()
        .ends_with("/blob/main/LICENSE"));
}
