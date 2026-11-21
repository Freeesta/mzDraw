//! Atom labels: abbreviations of common groups (drawn as their name, expanded to their atoms at export) and R-group placeholders
//! (`R`, `R1` ... `R32`). The table is the one place that says what an abbreviation stands for.

/// Abbreviation and its expansion as a SMILES whose first atom is the attachment point.
pub const ABBREVIATIONS: &[(&str, &str)] = &[
    ("Me", "C"),
    ("Et", "CC"),
    ("Pr", "CCC"),
    ("iPr", "C(C)C"),
    ("Bu", "CCCC"),
    ("tBu", "C(C)(C)C"),
    ("Ph", "c1ccccc1"),
    ("Bn", "Cc1ccccc1"),
    ("Ac", "C(C)=O"),
    ("Bz", "C(=O)c1ccccc1"),
    ("Boc", "C(=O)OC(C)(C)C"),
    ("Cbz", "C(=O)OCc1ccccc1"),
    ("Fmoc", "C(=O)OCC1c2ccccc2-c2ccccc12"),
    ("Ts", "S(=O)(=O)c1ccc(C)cc1"),
    ("Ms", "S(C)(=O)=O"),
    ("Tf", "S(=O)(=O)C(F)(F)F"),
    ("OMe", "OC"),
    ("OEt", "OCC"),
    ("OAc", "OC(C)=O"),
    ("OTs", "OS(=O)(=O)c1ccc(C)cc1"),
    ("NMe2", "N(C)C"),
    ("NO2", "[N+](=O)[O-]"),
    ("CN", "C#N"),
    ("CF3", "C(F)(F)F"),
    ("COOH", "C(=O)O"),
    ("CO2H", "C(=O)O"),
    ("CO2Me", "C(=O)OC"),
    ("CO2Et", "C(=O)OCC"),
    ("SO3H", "S(=O)(=O)O"),
    ("TMS", "[Si](C)(C)C"),
    ("TBS", "[Si](C)(C)C(C)(C)C"),
    // the glucuronic acid residue as a glycoside (C6H9O6 on the atom it replaces a hydrogen of): the anomeric carbon first, no stereo
    ("GlcA", "C1OC(C(=O)O)C(O)C(O)C1O"),
];

/// What a label is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// An abbreviation, with its expansion.
    Group(&'static str),
    /// An R-group placeholder; 0 for a bare `R`.
    R(u8),
}

/// The kind of a label, `None` for a text that is neither an abbreviation of the table nor `R`, `R1` ... `R32`.
pub fn kind(label: &str) -> Option<Kind> {
    if let Some(&(_, smiles)) = ABBREVIATIONS.iter().find(|(name, _)| *name == label) {
        return Some(Kind::Group(smiles));
    }
    let n = label.strip_prefix('R')?;
    if n.is_empty() {
        return Some(Kind::R(0));
    }
    let ok = !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit());
    n.parse()
        .ok()
        .filter(|v| ok && (1..=32).contains(v))
        .map(Kind::R)
}

/// Abbreviations that are one unit inside a longer label (`OMe`, `NHBoc`, `CO2Et`): the names of the table that are not made of element
/// symbols, plus the usual prefixed butyls and propyls. Longest names first, so that `Boc` is read before `Bo...`.
const UNITS: &[&str] = &[
    "Boc", "Cbz", "Fmoc", "GlcA", "TMS", "TBS", "iPr", "nPr", "tBu", "nBu", "sBu", "iBu", "Me",
    "Et", "Pr", "Bu", "Ph", "Bn", "Ac", "Bz", "Ts", "Ms", "Tf",
];

/// One unit of a label: an element symbol with its count, an abbreviation (with a count: `Me2`), an `R` group with its number or
/// a group in parentheses with its count. A unit keeps its own order when a label is reversed.
enum Unit {
    /// Text and the length of the symbol in it (the rest is the count).
    Symbol(String, usize),
    /// Text and the length of the abbreviation in it (the rest is the count).
    Abbreviation(String, usize),
    /// The units inside the parentheses and the count after them.
    Group(Vec<Unit>, String),
}

/// A label arranged for drawing: the text to write and the characters (a range of indices into it) of its attachment atom, the
/// one that stands on the atom of the drawing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arranged {
    pub text: String,
    pub at: (usize, usize),
}

fn digits(s: &[char]) -> usize {
    s.iter().take_while(|c| c.is_ascii_digit()).count()
}

/// Reads `s` as units, `None` when something in it is not a unit (a lowercase letter alone, a sign, a space, an unclosed parenthesis).
fn parse(s: &[char]) -> Option<Vec<Unit>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        let take = |n: usize| rest[..n].iter().collect::<String>();
        if rest[0] == '(' {
            let mut depth = 0;
            let close = rest.iter().position(|&c| {
                depth += i32::from(c == '(') - i32::from(c == ')');
                depth == 0
            })?;
            let n = digits(&rest[close + 1..]);
            let inner = parse(&rest[1..close]).filter(|u| !u.is_empty())?;
            out.push(Unit::Group(
                inner,
                rest[close + 1..=close + n].iter().collect(),
            ));
            i += close + 1 + n;
            continue;
        }
        let name = UNITS.iter().find(|u| {
            let n = u.chars().count();
            rest.len() >= n && rest[..n].iter().copied().eq(u.chars())
        });
        let len = match name {
            Some(u) => u.chars().count(),
            None if !rest[0].is_ascii_uppercase() => return None,
            None => match rest.get(1) {
                Some(l) if l.is_ascii_lowercase() && mzlab_mol::is_element(&take(2)) => 2,
                _ if mzlab_mol::is_element(&take(1)) || rest[0] == 'R' => 1,
                _ => return None,
            },
        };
        let n = len + digits(&rest[len..]);
        out.push(if name.is_some() {
            Unit::Abbreviation(take(n), len)
        } else {
            Unit::Symbol(take(n), len)
        });
        i += n;
    }
    Some(out)
}

/// A unit written out and the range of its attachment atom: its symbol, the first letter of an abbreviation (the last one when
/// the label is reversed, as the abbreviation then ends against the bond), or whatever the group in parentheses attaches by.
fn write_unit(u: &Unit, reversed: bool) -> (String, (usize, usize)) {
    match u {
        Unit::Symbol(t, n) => (t.clone(), (0, *n)),
        Unit::Abbreviation(t, n) => (t.clone(), if reversed { (n - 1, *n) } else { (0, 1) }),
        Unit::Group(inner, count) => {
            let (s, at) = write(inner, reversed);
            (format!("({s}){count}"), (at.0 + 1, at.1 + 1))
        }
    }
}

/// Units written in order, or last to first, and the range of the one that came first.
fn write(units: &[Unit], reversed: bool) -> (String, (usize, usize)) {
    let mut text = String::new();
    let mut at = (0, 0);
    let order: Vec<&Unit> = if reversed {
        units.iter().rev().collect()
    } else {
        units.iter().collect()
    };
    for (k, u) in order.iter().enumerate() {
        let (s, a) = write_unit(u, reversed);
        if k == usize::from(reversed) * (units.len() - 1) {
            let before = text.chars().count();
            at = (before + a.0, before + a.1);
        }
        text.push_str(&s);
    }
    (text, at)
}

/// A label as it is drawn. The first unit of a label is its attachment atom, so a label that grows to the left of its atom (`left`:
/// the bonds leave to the right) is written with its units in the reverse order, each keeping its own: `CO2H` is drawn `HO2C`,
/// `OMe` `MeO`, `NMe2` `Me2N`, and `Boc`, `Ph` or `R1` stay as they are. A label that is not made of units is drawn as written.
pub fn arrange(label: &str, left: bool) -> Arranged {
    let chars: Vec<char> = label.chars().collect();
    match parse(&chars).filter(|u| !u.is_empty()) {
        Some(units) => {
            let (text, at) = write(&units, left);
            Arranged { text, at }
        }
        None => {
            let n = chars.len();
            let at = if left {
                (n.saturating_sub(1), n)
            } else {
                (0, n.min(1))
            };
            Arranged {
                text: label.to_string(),
                at,
            }
        }
    }
}
