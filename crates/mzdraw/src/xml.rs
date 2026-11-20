//! A small XML reader for the chemistry formats written in XML (CDXML; CML and MRV later): the element tree with attributes and
//! text, nothing else. It reads what those files use and refuses the rest in a bounded way: no DTD is followed and no entity is
//! defined beyond the five of XML and the numeric ones (so nothing expands), the depth and the number of elements are bounded,
//! and every malformed input is `None`, never a panic.

/// Elements nested deeper than this are refused.
pub const MAX_DEPTH: usize = 64;
/// Files with more elements than this are refused.
pub const MAX_ELEMENTS: usize = 400_000;

/// One element: its name, attributes in file order, child elements, and its own text (the pieces between its children, joined).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    /// The value of an attribute.
    pub fn attr(&self, name: &str) -> Option<&str> {
        let a = self.attrs.iter().find(|a| a.0 == name)?;
        Some(a.1.as_str())
    }

    /// The child elements of a name.
    pub fn each<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter(move |c| c.name == name)
    }
}

/// `&amp;`-style references of a text or attribute value replaced; `None` for a reference XML does not define.
fn unescape(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let end = rest[at..].find(';')? + at;
        let name = &rest[at + 1..end];
        out.push(match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = match name.strip_prefix("#x") {
                    Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                    None => name.strip_prefix('#')?.parse().ok()?,
                };
                char::from_u32(code)?
            }
        });
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Reads a document and gives its root element.
pub fn parse(text: &str) -> Option<Element> {
    // the open elements, innermost last; the bottom one collects the root
    let mut open = vec![Element::default()];
    let mut count = 0;
    let mut rest = text.trim_start_matches('\u{feff}');
    while let Some(at) = rest.find('<') {
        let top = open.last_mut()?;
        top.text.push_str(&unescape(&rest[..at])?);
        rest = &rest[at..];
        // what is skipped whole: comments, CDATA (kept as text), processing instructions, the DOCTYPE (with its internal subset)
        if let Some(r) = rest.strip_prefix("<!--") {
            rest = &r[r.find("-->")? + 3..];
        } else if let Some(r) = rest.strip_prefix("<![CDATA[") {
            let end = r.find("]]>")?;
            top.text.push_str(&r[..end]);
            rest = &r[end + 3..];
        } else if let Some(r) = rest.strip_prefix("<?") {
            rest = &r[r.find("?>")? + 2..];
        } else if let Some(r) = rest.strip_prefix("<!") {
            let (mut depth, mut end) = (0i32, None);
            for (i, c) in r.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => depth -= 1,
                    '>' if depth <= 0 => {
                        end = Some(i);
                        break;
                    }
                    _ => {}
                }
            }
            rest = &r[end? + 1..];
        } else if let Some(r) = rest.strip_prefix("</") {
            let end = r.find('>')?;
            let done = open.pop()?;
            if done.name != r[..end].trim_end() || open.is_empty() {
                return None;
            }
            open.last_mut()?.children.push(done);
            rest = &r[end + 1..];
        } else {
            let r = &rest[1..];
            let name_end = r.find(|c: char| c.is_whitespace() || c == '>' || c == '/')?;
            let mut e = Element {
                name: r[..name_end].to_string(),
                ..Element::default()
            };
            count += 1;
            if e.name.is_empty() || count > MAX_ELEMENTS || open.len() > MAX_DEPTH {
                return None;
            }
            let mut r = r[name_end..].trim_start();
            // attributes: name = "value" or 'value'
            while !r.starts_with('>') && !r.starts_with("/>") {
                let eq = r.find('=')?;
                let name = r[..eq].trim_end();
                let v = r[eq + 1..].trim_start();
                let quote = v.chars().next().filter(|q| *q == '"' || *q == '\'')?;
                let close = v[1..].find(quote)? + 1;
                if name.is_empty()
                    || name.contains(|c: char| c.is_whitespace() || c == '<' || c == '>')
                {
                    return None;
                }
                e.attrs.push((name.to_string(), unescape(&v[1..close])?));
                r = v[close + 1..].trim_start();
            }
            if let Some(after) = r.strip_prefix("/>") {
                open.last_mut()?.children.push(e);
                rest = after;
            } else {
                open.push(e);
                rest = &r[1..];
            }
        }
    }
    // everything closed, one root, nothing but blanks around it
    let mut top = open.pop().filter(|_| open.is_empty())?;
    let alone = top.children.len() == 1 && top.text.trim().is_empty() && rest.trim().is_empty();
    top.children.pop().filter(|_| alone)
}

/// A text as the content of an element or of a double-quoted attribute.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // a control character has no place in XML 1.0: it is written as a blank
            c if c < ' ' && c != '\n' && c != '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_is_read_as_its_tree() {
        let doc = "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE CDXML SYSTEM \"x.dtd\" [ <!ENTITY a \"b\"> ]>\n<!-- a <comment> -->\n<a x=\"1\" y = 'two &amp; &#x394; &#65;'>\n  t1 &lt;<b/>t2<c k=\"v\">in<![CDATA[<raw&>]]></c>\n  <b n=\"2\"></b>\n</a>\n";
        let a = parse(doc).unwrap();
        assert_eq!(
            (a.name.as_str(), a.attr("x"), a.attr("y"), a.attr("z")),
            ("a", Some("1"), Some("two & Δ A"), None)
        );
        assert_eq!(a.children.len(), 3);
        assert_eq!(a.text.split_whitespace().collect::<Vec<_>>(), ["t1", "<t2"]);
        assert_eq!(a.each("b").count(), 2);
        assert_eq!(a.each("b").nth(1).unwrap().attr("n"), Some("2"));
        let c = a.each("c").next().unwrap();
        assert_eq!((c.attr("k"), c.text.as_str()), (Some("v"), "in<raw&>"));
        // what is written is read back
        let odd = "a<b>&\"c\"\u{1}\n";
        let back = parse(&format!("<r v=\"{}\">{}</r>", escape(odd), escape(odd))).unwrap();
        assert_eq!(
            (back.attr("v"), back.text.as_str()),
            (Some("a<b>&\"c\" \n"), "a<b>&\"c\" \n")
        );
    }

    #[test]
    fn what_is_not_well_formed_or_too_much_is_refused() {
        for bad in [
            "",
            "text only",
            "<a>",
            "<a></b>",
            "</a>",
            "<a/><b/>",
            "<a/>tail",
            "<a x=1/>",
            "<a x=\"1/>",
            "<a x/>",
            "<a>&nope;</a>",
            "<a>&#xD800;</a>",
            "<a>&amp</a>",
            "<a><!-- open</a>",
            "<a><![CDATA[open</a>",
            "<a",
            "<>",
            "< a/>",
        ] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
        // an entity defined by the file is not expanded: it is an unknown reference
        assert_eq!(
            parse("<!DOCTYPE a [<!ENTITY big \"xxxxxxxx\">]><a>&big;</a>"),
            None
        );
        // depth and size are bounded
        let deep = "<a>".repeat(MAX_DEPTH + 2) + &"</a>".repeat(MAX_DEPTH + 2);
        assert_eq!(parse(&deep), None);
        let fine = "<a>".repeat(MAX_DEPTH) + &"</a>".repeat(MAX_DEPTH);
        assert!(parse(&fine).is_some());
        let many = format!("<r>{}</r>", "<e/>".repeat(MAX_ELEMENTS));
        assert_eq!(parse(&many), None);
    }

    #[test]
    fn damage_is_never_a_panic() {
        let good = "<?xml version=\"1.0\"?><!DOCTYPE r><r a=\"1\" b='&lt;'><x p=\"1 2\">t&amp;<y/></x><!-- c --><![CDATA[d]]>é</r>";
        let mut x = 0x2545_F491_4F6C_DD1Du64;
        let mut next = |n: usize| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % n as u64) as usize
        };
        let mut read = 0;
        for _ in 0..20_000 {
            let mut b = good.as_bytes().to_vec();
            for _ in 0..1 + next(3) {
                let at = next(b.len());
                match next(3) {
                    0 => b[at] = b"<>/&;\"'=![]-? ax\xC3"[next(16)],
                    1 => {
                        b.remove(at);
                    }
                    _ => b.insert(at, b"<>&\"'/"[next(6)]),
                }
            }
            read += usize::from(parse(&String::from_utf8_lossy(&b)).is_some());
        }
        assert!(read > 500, "{read}");
    }
}
