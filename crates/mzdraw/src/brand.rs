//! The product name and every id made from it, in one place (the file `brand.toml` at the root of the repository says the same;
//! the test `brand` fails when they differ). "mzDraw" is a codename: nothing else in the code or in the texts of the page writes
//! it. Persisted ids of earlier names stay readable (`LEGACY_IDS`); writers use the current ones.

/// The name people read.
pub const NAME: &str = "mzDraw";
/// The name in files: PNG keyword, PDF attachment names, SVG metadata id, MIME subtype, storage keys, the `out` kind of the document.
pub const SLUG: &str = "mzdraw";
/// The crates are `<prefix>` and `<prefix>-wasm`.
pub const CRATE_PREFIX: &str = "mzdraw";
/// The custom element mzLab embeds.
pub const COMPONENT_TAG: &str = "mzdraw-editor";
/// The colour tokens of the page are `--<prefix>-*`.
pub const CSS_PREFIX: &str = "mzd";
/// The global the page puts on `window`.
pub const APP_ID: &str = "MZDRAW";
/// The saved document: `<slug>/<version>`.
pub const DOCUMENT_ID: &str = "mzdraw/1";
/// The settings file: `<slug>-settings/<version>`.
pub const SETTINGS_ID: &str = "mzdraw-settings/1";
/// The command log (language of mzLab; does not depend on the name).
pub const EDIT_ID: &str = "draw_edit/1";
/// Earlier slugs, oldest first.
#[cfg(not(test))]
pub const LEGACY_IDS: &[&str] = &[];
/// The unit tests of the crate read as if one earlier name existed (as long as the current slug, so a PDF keeps its offsets), to
/// prove that every reader accepts it; the build of the crate and the test `brand` use the list of `brand.toml`.
#[cfg(test)]
pub const LEGACY_IDS: &[&str] = &["zzzzzz"];
/// GitHub `owner/name`.
pub const REPOSITORY: &str = "Freeesta/mzDraw";
/// Where the page is published (case-sensitive) and its path.
pub const PAGES_URL: &str = "https://freeesta.github.io/mzDraw/";
pub const PAGES_BASE: &str = "/mzDraw/";
/// The title of the citation file.
pub const CITATION_TITLE: &str = "mzDraw: a 2D molecule editor in Rust and WebAssembly";

/// The slug of the name in use first, then the earlier ones: what a reader accepts.
pub fn slugs() -> impl Iterator<Item = &'static str> {
    std::iter::once(SLUG).chain(LEGACY_IDS.iter().copied())
}

/// Whether `kind` names the document as an `out` kind (`save`, `clipboard`): the current slug or an earlier one.
pub fn is_doc_kind(kind: &str) -> bool {
    slugs().any(|s| s == kind)
}

/// The version `n` of `format` when it is `<slug><suffix>/<n>` for the current slug or an earlier one (no leading zero), else `Err`.
fn version(format: &str, suffix: &str) -> Result<u32, ()> {
    slugs()
        .find_map(|s| {
            format
                .strip_prefix(s)?
                .strip_prefix(suffix)?
                .strip_prefix('/')
        })
        .filter(|n| !n.is_empty() && !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|n| n.parse().ok())
        .ok_or(())
}

/// The version of a document id (`<slug>/<n>`, any slug a reader accepts).
pub(crate) fn document_version(format: &str) -> Result<u32, ()> {
    version(format, "")
}

/// The version of a settings id (`<slug>-settings/<n>`, any slug a reader accepts).
pub(crate) fn settings_version(format: &str) -> Result<u32, ()> {
    version(format, "-settings")
}

/// The name of a file attached to a PDF, `<slug>.<ext>`, for the name in use.
pub(crate) fn attachment(ext: &str) -> String {
    format!("{SLUG}.{ext}")
}

/// The same for the name in use and then for every earlier one: where a reader looks.
pub(crate) fn attachments(ext: &str) -> impl Iterator<Item = String> + '_ {
    slugs().map(move |s| format!("{s}.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pdf, svg, Document, Editor, Style};

    const OLD: &str = "zzzzzz";

    fn doc() -> Document {
        Document::from_json(
            r#"{"format":"mzdraw/1","next":4,"atoms":[{"id":1,"el":"C","x":0.0,"y":0.0},{"id":2,"el":"O","x":1.0,"y":0.0}],"bonds":[{"id":3,"a":1,"b":2,"o":2}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn the_version_rule_takes_the_name_in_use_and_every_earlier_one() {
        for slug in [SLUG, OLD] {
            assert_eq!(document_version(&format!("{slug}/1")), Ok(1));
            assert_eq!(document_version(&format!("{slug}/12")), Ok(12));
            assert_eq!(settings_version(&format!("{slug}-settings/3")), Ok(3));
        }
        for bad in [
            "",
            "other/1",
            "mzdraw/",
            "mzdraw/01",
            "mzdraw/1x",
            "mzdraw/-1",
            "mzdraw-settings/1",
            "mzdraw1",
        ] {
            assert_eq!(document_version(bad), Err(()), "{bad}");
        }
        assert_eq!(settings_version("mzdraw/1"), Err(()));
        assert_eq!(settings_version("other-settings/1"), Err(()));
        assert_eq!(DOCUMENT_ID, format!("{SLUG}/1"));
        assert_eq!(SETTINGS_ID, format!("{SLUG}-settings/1"));
    }

    #[test]
    fn a_document_and_a_settings_file_of_an_earlier_name_are_read_and_written_again_under_the_current_one(
    ) {
        let now = doc().to_json();
        let old = now.replace(DOCUMENT_ID, &format!("{OLD}/1"));
        assert_ne!(now, old);
        assert_eq!(Document::from_json(&old).unwrap().to_json(), now);
        assert_eq!(
            Document::from_json(&old.replace("/1", "/2")).err(),
            Some("error.draw2.documentNewer")
        );
        let style = format!(r#"{{"format":"{OLD}-settings/1","style":{{"preset":"slides"}}}}"#);
        assert_eq!(Style::from_text(&style).unwrap().name, "slides");
        assert_eq!(
            Style::from_text(&style.replace("/1", "/2")).err(),
            Some("error.draw2.styleNewer")
        );
        assert_eq!(
            Style::from_text(&style.replace(OLD, "other")).err(),
            Some("error.draw2.style")
        );
    }

    #[test]
    fn the_carriers_and_the_kind_of_an_earlier_name_are_read() {
        let d = doc();
        // SVG: the metadata id
        let svg = svg::carrying(svg::to_svg(&d), &d).replace(
            &format!("<metadata id=\"{SLUG}\">"),
            &format!("<metadata id=\"{OLD}\">"),
        );
        assert!(svg.contains(&format!("id=\"{OLD}\"")));
        assert_eq!(svg::carried(&svg).unwrap().unwrap().to_json(), d.to_json());
        // PDF: the names of the attachments (the same length, so the offsets hold)
        let pdf = pdf::to_pdf(&d, &Style::default());
        let (from, to) = (
            attachment("json").into_bytes(),
            format!("{OLD}.json").into_bytes(),
        );
        let mut old = pdf.clone();
        let at = pdf.windows(from.len()).position(|w| w == from).unwrap();
        old[at..at + to.len()].copy_from_slice(&to);
        assert_ne!(old, pdf);
        assert_eq!(pdf::carried(&old).unwrap().unwrap().to_json(), d.to_json());
        // the kind of `save` and `clipboard`
        assert!(is_doc_kind(SLUG) && is_doc_kind(OLD) && !is_doc_kind("molfile"));
        let mut e = Editor::default();
        e.open(&d.to_json()).unwrap();
        assert_eq!(e.save(OLD).unwrap(), e.save(SLUG).unwrap());
    }
}
