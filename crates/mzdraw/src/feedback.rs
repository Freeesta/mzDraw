//! The report of a bug or the suggestion of an improvement: a new issue of the repository, prefilled, that the page opens in a new tab
//! (`feedback/1`). The site has no server: nothing leaves the browser until the person submits the issue on GitHub. Every word of the
//! issue is made here, so the page only opens the URL and the CLI (`feedback` request field) gives the same bytes. No time, no random.
//!
//! The URL carries the id of each field of the issue form (`.github/ISSUE_TEMPLATE/<kind>.yml`): GitHub fills a form field from the query
//! parameter of its id (it ignores `body` for a form). `body` of the answer is the same text as plain Markdown, for the clipboard.

use crate::brand::{NAME, REPOSITORY, VERSION};

/// The repository on GitHub (the one place `brand::REPOSITORY` is turned into a URL).
fn repo_url() -> String {
    format!("https://github.com/{REPOSITORY}")
}

/// The longest URL the page opens, in bytes (a browser, a proxy or GitHub may refuse a longer one). The command log goes in only below it.
pub const LIMIT: usize = 6000;

/// The kinds of report and their issue forms: `(kind, template file, title prefix)`.
const KINDS: [(&str, &str, &str); 2] = [
    ("bug", "bug.yml", "Bug: "),
    ("improvement", "improvement.yml", "Improvement: "),
];

/// What a report is made of.
pub struct Params<'a> {
    /// `bug` or `improvement`.
    pub kind: &'a str,
    /// Whether the person ticked the box: the command log of the drawing goes in (public on GitHub).
    pub with_log: bool,
    /// The command log (`draw_edit/1` JSON), whatever the box says; used only when `with_log`.
    pub log: &'a str,
    /// The user agent string of the browser, and its language (empty from the CLI).
    pub agent: &'a str,
    pub lang: &'a str,
}

/// `feedback/1`.
pub struct Feedback {
    pub url: String,
    pub template: &'static str,
    pub title: String,
    pub body: String,
    /// The log was asked for and is not in the URL (too long, or there is none): the `notes` say which.
    pub truncated: bool,
    /// Keys of the notes the page says in words (`feedback.logTooLong`, `feedback.noLog`).
    pub notes: Vec<&'static str>,
}

/// The percent-encoding of a text (UTF-8 bytes; only `A-Z a-z 0-9 - . _ ~` stay): safe in a query for `&`, `#`, `=`, `+`, a space and a line break.
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A one-line text of at most `max` characters, without control characters.
fn clean(text: &str, max: usize) -> String {
    let one: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    one.trim().chars().take(max).collect()
}

/// The report, or the key of the refusal (`error.draw2.feedback`: a kind that is not `bug` or `improvement`).
pub fn build(p: &Params) -> Result<Feedback, &'static str> {
    let (_, template, prefix) = *KINDS
        .iter()
        .find(|k| k.0 == p.kind)
        .ok_or("error.draw2.feedback")?;
    let page = format!(
        "Version: {VERSION}\nBrowser: {}\nLanguage: {}",
        clean(p.agent, 300),
        clean(p.lang, 35)
    );
    let log = p.log.trim();
    let mut notes = Vec::new();
    let mut with_log = p.with_log;
    if with_log && (log.is_empty() || log == "[]") {
        notes.push("feedback.noLog");
        with_log = false;
    }
    let title = prefix.trim_end().to_string();
    let url_of = |log: Option<&str>| {
        let mut url = format!(
            "{}/issues/new?template={template}&title={}&page={}",
            repo_url(),
            encode(&title),
            encode(&page)
        );
        if let Some(log) = log {
            url.push_str("&log=");
            url.push_str(&encode(log));
        }
        url
    };
    let mut url = url_of(with_log.then_some(log));
    if with_log && url.len() > LIMIT {
        notes.push("feedback.logTooLong");
        with_log = false;
        url = url_of(None);
    }
    let mut body = format!("### Page\n\n{page}\n");
    if with_log {
        body.push_str(&format!("\n### Command log\n\n```json\n{log}\n```\n"));
    }
    Ok(Feedback {
        url,
        template,
        title,
        body,
        truncated: p.with_log && !with_log,
        notes,
    })
}

impl Feedback {
    /// `feedback/1`, one line.
    pub fn json(&self) -> String {
        serde_json::json!({
            "format": "feedback/1", "url": self.url, "template": self.template, "title": self.title,
            "body": self.body, "truncated": self.truncated, "notes": self.notes,
        })
        .to_string()
    }
}

/// What the About box shows besides its sentences (`about/1`): the name, the version, the repository and the licence.
pub fn about_json() -> String {
    let repo = repo_url();
    serde_json::json!({
        "format": "about/1", "name": NAME, "version": VERSION,
        "repository": repo, "licence": format!("{repo}/blob/main/LICENSE"),
    })
    .to_string()
}
