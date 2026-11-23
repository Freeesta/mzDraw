//! The groups of the general tools palette (`docs/built/ui-declutter-2.md`): what the one palette of tools holds, in which order, and
//! which member of each group stands for it. A group is one button; a click on it selects its face and opens the list of every
//! member; a member picked from the list becomes the face (the memory of the last one used). The page paints what is decided here
//! (`Layout::arranged_json` writes it) and decides nothing; the memory lives in the `layout` object of the settings file as
//! `heads`, so it survives a reload and has a CLI twin, `--set layout.heads.<group>=<tool>`.
//!
//! A member that is also shown in view on its own (`keep`: the kinds drawn most) is listed in the group like the others but never
//! takes the face: the face would then repeat a button that is already there.

/// One group of tools: a button that stands for several.
#[derive(Debug, PartialEq, Eq)]
pub struct Group {
    /// The name of the group: also the name of its list on the page and the key of its memory in the settings.
    pub id: &'static str,
    /// The face the button shows when nothing was used yet, and for ever when it is empty (then `icon` is the face).
    pub first: &'static str,
    /// The face of a group that has no tool of its own to show (a periodic table, a text line).
    pub icon: &'static str,
    /// Every tool of the group, in the order of its list.
    pub members: &'static [&'static str],
    /// Members that are in view on their own too: listed, but never the face.
    pub keep: &'static [&'static str],
    /// Typed entries at the end of the list (`atom`, `label`, `text` of the page).
    pub fields: &'static [&'static str],
    /// Tools that start so are the group's, and press its button (an element typed in the field, a label, a text line).
    pub owns: &'static str,
    /// The list is a grid of icons (many members that read by their shape), else rows with their names.
    pub grid: bool,
}

/// One place of the palette: a tool on its own, or a group.
#[derive(Debug, PartialEq, Eq)]
pub enum Slot {
    Tool(&'static str),
    Group(&'static Group),
}

pub const BONDS: Group = Group {
    id: "bonds",
    first: "bond:1",
    icon: "",
    members: &[
        "bond:1",
        "bond:2",
        "bond:3",
        "bond:up",
        "bond:down",
        "bond:wavy",
        "bond:dash",
        "bond:bdash",
        "bond:hash",
        "bond:bold",
        "bond:dative",
        "bond:hbond",
        "chain",
    ],
    keep: &["bond:2"],
    fields: &[],
    owns: "",
    grid: true,
};
pub const RINGS: Group = Group {
    id: "rings",
    first: "ring:6a",
    icon: "",
    members: &[
        "ring:6a", "ring:6", "ring:5", "ring:3", "ring:4", "ring:7", "ring:8",
    ],
    keep: &["ring:6", "ring:5"],
    fields: &[],
    owns: "",
    grid: true,
};
pub const ATOMS: Group = Group {
    id: "atoms",
    first: "",
    icon: "elements",
    members: &[
        "atom:S", "atom:P", "atom:F", "atom:Cl", "atom:Br", "atom:I", "atom:H", "atom:B", "atom:Si",
    ],
    keep: &[],
    fields: &["atom"],
    owns: "atom:",
    grid: true,
};
pub const CHARGES: Group = Group {
    id: "charges",
    first: "charge:+",
    icon: "",
    members: &["charge:+", "charge:-"],
    keep: &[],
    fields: &["label"],
    owns: "label:",
    grid: false,
};
pub const OBJECTS: Group = Group {
    id: "objects",
    first: "arrow",
    icon: "",
    members: &[
        "arrow",
        "plus",
        "scheme-row",
        "pathway-tree",
        "pathway-label",
        "lonepair",
        "curve:full",
        "curve:half",
        "lonepairs-place",
    ],
    keep: &["plus"],
    fields: &[],
    owns: "",
    grid: false,
};
pub const TEXT: Group = Group {
    id: "text",
    first: "",
    icon: "text",
    members: &[],
    keep: &[],
    fields: &["text"],
    owns: "text:",
    grid: false,
};

/// The places of the palette, column after column (a column holds `ROWS`): the structure tools, then the atoms and the objects.
pub const SLOTS: [Slot; 16] = [
    Slot::Tool("select"),
    Slot::Tool("move-here"),
    Slot::Tool("erase"),
    Slot::Group(&BONDS),
    Slot::Tool("bond:2"),
    Slot::Group(&RINGS),
    Slot::Tool("ring:6"),
    Slot::Tool("ring:5"),
    Slot::Tool("atom:C"),
    Slot::Tool("atom:N"),
    Slot::Tool("atom:O"),
    Slot::Group(&ATOMS),
    Slot::Group(&CHARGES),
    Slot::Group(&OBJECTS),
    Slot::Tool("plus"),
    Slot::Group(&TEXT),
];

/// Places in a column of the palette: it is two columns of 44 px, 8 rows each.
pub const ROWS: usize = 8;

pub const GROUPS: [&Group; 6] = [&BONDS, &RINGS, &ATOMS, &CHARGES, &OBJECTS, &TEXT];

impl Group {
    /// The group of that id.
    pub fn named(id: &str) -> Option<&'static Group> {
        GROUPS.into_iter().find(|g| g.id == id)
    }

    /// The group a tool belongs to (as a member).
    pub fn of(tool: &str) -> Option<&'static Group> {
        GROUPS.into_iter().find(|g| g.members.contains(&tool))
    }

    /// Whether the face of the group is the tool last used from it (else the face is its icon, always).
    pub fn remembers(&self) -> bool {
        !self.first.is_empty()
    }

    /// Whether the button of the group stands for the tool in use: a member that is not in view on its own, or a tool the group
    /// owns by its name (a typed element, a label, a text) that is not one of the places of the palette.
    pub fn pressed(&self, tool: &str) -> bool {
        (self.members.contains(&tool) && !self.keep.contains(&tool))
            || (!self.owns.is_empty()
                && tool.starts_with(self.owns)
                && !SLOTS
                    .iter()
                    .any(|s| matches!(s, Slot::Tool(t) if *t == tool)))
    }

    /// Whether the member may be the face: it belongs to the group and is not in view on its own.
    pub fn can_head(&self, tool: &str) -> bool {
        self.remembers()
            && self.members.contains(&tool)
            && !self.keep.contains(&tool)
            // a command that is not a tool (a scheme row, a pathway) never becomes a face: nothing selects it
            && crate::editor::Tool::parse(tool).is_some()
    }
}

/// What each group shows as its face: the last member used, kept as the text of the settings (`heads`), only where it differs
/// from the group's first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heads(Vec<(String, String)>);

impl Heads {
    /// The face of a group now: the member last used, else its first.
    pub fn face(&self, g: &Group) -> String {
        self.0
            .iter()
            .find(|(k, _)| k == g.id)
            .map_or_else(|| g.first.to_string(), |(_, v)| v.clone())
    }

    /// Sets the face of a group; a member that may not be a face (not of the group, in view on its own) is refused. The first
    /// member is the default, so it forgets the entry. Returns whether anything changed.
    pub fn set(&mut self, group: &str, tool: &str) -> Result<bool, &'static str> {
        let g = Group::named(group).filter(|g| g.remembers()).ok_or(ERR)?;
        if !g.can_head(tool) {
            return Err(ERR);
        }
        let was = self.face(g);
        self.0.retain(|(k, _)| k != g.id);
        if tool != g.first {
            self.0.push((g.id.to_string(), tool.to_string()));
            // a fixed order, whatever the order of use: the same faces write the same text
            self.0
                .sort_by_key(|(k, _)| GROUPS.iter().position(|g| g.id == k));
        }
        Ok(was != tool)
    }

    /// A tool has been chosen: when it is a member that may be a face, it becomes its group's face. Returns whether a face changed.
    pub fn note(&mut self, tool: &str) -> bool {
        match Group::of(tool) {
            Some(g) if g.can_head(tool) => self.set(g.id, tool).unwrap_or(false),
            _ => false,
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The entries as they are written: `(group, tool)`, in the order of the groups.
    pub fn entries(&self) -> &[(String, String)] {
        &self.0
    }
}

/// The error of a face that is not one: an i18n key.
const ERR: &str = "error.draw2.layout";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_is_in_one_place_only_and_the_order_is_fixed() {
        let mut seen: Vec<&str> = Vec::new();
        for s in &SLOTS {
            if let Slot::Tool(t) = s {
                assert!(!seen.contains(t), "{t} is in two places");
                seen.push(t);
            }
        }
        assert_eq!(SLOTS.len(), 2 * ROWS);
        // the kinds in view on their own are exactly the `keep` of their groups
        for g in GROUPS {
            for k in g.keep {
                assert!(
                    seen.contains(k) && g.members.contains(k),
                    "{k} is kept and listed"
                );
            }
        }
        // a member is in one group only, and a group's first is a member that may be the face
        let mut all: Vec<&str> = GROUPS
            .iter()
            .flat_map(|g| g.members.iter().copied())
            .collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n);
        for g in GROUPS {
            assert!(!g.remembers() || g.can_head(g.first), "{}", g.id);
        }
    }

    #[test]
    fn every_member_is_a_tool_or_a_command_of_the_page_and_only_a_tool_can_be_a_face() {
        let commands = [
            "scheme-row",
            "pathway-tree",
            "pathway-label",
            "lonepairs-place",
        ];
        for g in GROUPS {
            for m in g.members {
                let tool = crate::editor::Tool::parse(m).is_some();
                assert_eq!(tool, !commands.contains(m), "{m}");
                if g.remembers() && !g.keep.contains(m) {
                    assert_eq!(g.can_head(m), tool, "{m}");
                }
            }
        }
    }

    #[test]
    fn a_group_is_pressed_by_a_member_not_in_view_or_a_tool_it_owns() {
        assert!(BONDS.pressed("bond:wavy") && !BONDS.pressed("bond:2") && !BONDS.pressed("ring:3"));
        assert!(ATOMS.pressed("atom:S") && ATOMS.pressed("atom:Se") && !ATOMS.pressed("atom:N"));
        assert!(
            CHARGES.pressed("label:Ph") && TEXT.pressed("text:H2O") && !OBJECTS.pressed("plus")
        );
    }

    #[test]
    fn the_face_is_the_first_until_a_member_is_used_then_the_one_used_last() {
        let mut h = Heads::default();
        assert_eq!(h.face(&BONDS), "bond:1");
        assert!(h.note("bond:wavy"));
        assert_eq!(h.face(&BONDS), "bond:wavy");
        assert!(!h.note("bond:wavy"), "the same again changes nothing");
        // a kind that is in view on its own never takes the face; a tool of no group, nor one of a fixed face, neither
        assert!(!h.note("bond:2") && !h.note("ring:6") && !h.note("select") && !h.note("atom:S"));
        assert_eq!(h.face(&BONDS), "bond:wavy");
        assert!(h.note("bond:1"));
        assert!(h.is_empty(), "the first is the default: nothing to keep");
        assert!(h.note("charge:-") && !h.note("pathway-tree"));
        assert_eq!(h.face(&CHARGES), "charge:-");
        assert_eq!(h.face(&OBJECTS), "arrow");
    }

    #[test]
    fn the_entries_come_out_in_the_order_of_the_groups_whatever_the_order_of_use() {
        let (mut a, mut b) = (Heads::default(), Heads::default());
        for t in ["charge:-", "ring:3", "bond:hash"] {
            a.note(t);
        }
        for t in ["bond:hash", "ring:3", "charge:-"] {
            b.note(t);
        }
        assert_eq!(a, b);
        let keys: Vec<&str> = a.entries().iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["bonds", "rings", "charges"]);
    }

    #[test]
    fn a_face_that_is_not_a_member_is_refused() {
        let mut h = Heads::default();
        assert!(h.set("bonds", "ring:3").is_err());
        assert!(h.set("bonds", "bond:2").is_err(), "in view on its own");
        assert!(h.set("atoms", "atom:S").is_err(), "a fixed face");
        assert!(h.set("nope", "bond:1").is_err());
        assert!(h.is_empty());
    }
}
