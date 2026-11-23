//! Pathway layout: the arrows of a scheme organise themselves (tp-05). One command, `pathway:tree`, lays out the whole graph of the
//! anchored arrows of a document (tp-01): the structures that are the `from` or the `to` of an anchored arrow are the nodes, the arrows
//! the edges. The layers are the longest directed paths from the roots, every node keeps one primary parent (a forest), the forest is
//! placed across the layers by the linear tidy-tree algorithm of Buchheim, Jünger and Leipert (2002) on the box heights, the columns
//! (one x per layer and component) are spaced so that every primary arrow is not steeper than `pathway.max_angle` and not shorter
//! than `pathway.min_arrow`, and the arrows are routed again by `route`. Everything here is a function of the document and the settings
//! `pathway.*`, in bond lengths; the same document gives the same bytes on every surface.
//!
//! `layout` gives the unrounded target positions and the report; `ops` gives the rounded `move` operations and then the routing
//! operations of every edge, on the moved document. No new operation: the log of `draw_edit/1` carries moves and tp-01's fields only.
//!
//! The specification and its properties (P1 to P14) are in `docs/prompts/tp-05-pathway-layout.md`. The algorithm is Walker's tidy tree as made
//! linear by Buchheim et al. (2002); neither paper was read, the code is written from general knowledge of it and held to the properties.

use crate::conform::{Finding, Report};
use crate::doc::{round4, Document, ObjKind};
use crate::ops::Op;
use crate::route::{clip, piece_box, route_between, Rect};
use crate::settings::Style;
use std::collections::HashMap;

/// The sibling order key (`pathway.order`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// The smallest atom id of the structure.
    Id,
    /// The numeric data field `MZ` of the structure, missing values last.
    Mz,
    /// The numeric data field `RT` of the structure, missing values last.
    Rt,
    /// The reverse of `Id`; a key for tests of the mirror property (P8), not a feature.
    ReverseId,
}

impl Order {
    /// The keys a settings file may hold (not the test-only `reverse-id`).
    pub fn parse_public(v: &str) -> Option<Order> {
        match v {
            "id" => Some(Order::Id),
            "mz" => Some(Order::Mz),
            "rt" => Some(Order::Rt),
            _ => None,
        }
    }

    /// The keys `--set pathway.order=` takes: the public ones and `reverse-id`, a session-only key for the mirror tests (P8) that is
    /// never written to a settings file.
    pub fn parse(v: &str) -> Option<Order> {
        match v {
            "reverse-id" => Some(Order::ReverseId),
            _ => Order::parse_public(v),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Order::Id => "id",
            Order::Mz => "mz",
            Order::Rt => "rt",
            Order::ReverseId => "reverse-id",
        }
    }
}

/// Why a pathway is not laid out: the i18n key and the id it is about (an arrow id; 0 for none).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No anchored arrow joins two structures.
    Empty,
    /// Two arrows have the same `from` and `to` structures; the smallest arrow id of the pair.
    Duplicate(u32),
    /// A directed cycle; the smallest arrow id on a cycle.
    Cycle(u32),
    /// More than `MAX_ARROWS` anchored arrows or `MAX_NODES` structures: the layout is refused before any walk (a hostile document).
    Size,
    /// The moves are refused by the document (a coordinate out of range): the key and id of the operation.
    Apply(&'static str, u32),
}

/// The most anchored arrows a pathway may have.
pub const MAX_ARROWS: usize = 1000;
/// The most structures a pathway may have.
pub const MAX_NODES: usize = 1000;
/// The step (bond lengths) and the most steps of the rule `crossing` of the gap between two columns.
pub const CROSSING_STEP: f64 = 0.05;
pub const CROSSING_STEPS: u32 = 400;

impl Refusal {
    pub fn key(&self) -> &'static str {
        match self {
            Refusal::Empty => "error.draw2.schemeEmpty",
            Refusal::Duplicate(_) => "error.draw2.schemeDuplicate",
            Refusal::Cycle(_) => "error.draw2.schemeCycle",
            Refusal::Size => "error.draw2.schemeSize",
            Refusal::Apply(key, _) => key,
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            Refusal::Empty | Refusal::Size => 0,
            Refusal::Duplicate(i) | Refusal::Cycle(i) | Refusal::Apply(_, i) => *i,
        }
    }
}

/// A structure of the pathway.
#[derive(Clone, Debug)]
pub struct Node {
    /// The smallest atom id of the structure.
    pub id: u32,
    /// The atoms of the structure, ascending.
    pub atoms: Vec<u32>,
    /// Its box now (x0, y0, x1, y1), labels included.
    pub rect: Rect,
    /// The layer: the length of the longest directed path from a root.
    pub layer: usize,
    /// The component (weakly connected), in the order of the layout.
    pub comp: usize,
}

/// An anchored arrow `from → to` between two structures.
#[derive(Clone, Debug)]
pub struct Edge {
    pub arrow: u32,
    /// Indices into `Layout::nodes`.
    pub from: usize,
    pub to: usize,
    /// Whether the arrow is in the forest (the primary parent of `to`).
    pub primary: bool,
    /// The width of the text of the arrow as painted (bond lengths): the visible arrow must be at least that long (rule `label`).
    pub label: f64,
}

/// A raised gap between two columns of a component: `gap` replaced `pathway.col_gap` by the `rule` (`slope`, `length`, `label` or `crossing`).
#[derive(Clone, Debug, PartialEq)]
pub struct GapNote {
    pub comp: usize,
    /// The gap is between the layers `k` and `k + 1`.
    pub k: usize,
    pub gap: f64,
    pub rule: &'static str,
    /// The arrows the gap is about: the primary arrows between the two layers; for the rule `crossing` also the secondary arrows
    /// and the arrows that skip a layer and span the gap.
    pub arrows: Vec<u32>,
}

/// The result of `layout`: the target centre of every box (unrounded) and the report.
#[derive(Clone, Debug)]
pub struct Layout {
    pub nodes: Vec<Node>,
    /// By arrow id.
    pub edges: Vec<Edge>,
    /// The target box centres, one per node.
    pub centres: Vec<(f64, f64)>,
    /// The raised gaps.
    pub gaps: Vec<GapNote>,
    /// The gaps where the rule `crossing` reached its bound (`CROSSING_STEPS`) with arrows still meeting a box (primary arrows, or
    /// the secondary and skipping arrows that span the gap): `gap` is the value kept.
    pub limits: Vec<GapNote>,
    /// Objects that are not moved: the arrows without two structures, the plus signs and the texts (ids ascending).
    pub free: Vec<u32>,
    /// The arrows whose visible segment meets a route box other than its two ends: (arrow, the ids of the structures it meets).
    pub crossings: Vec<(u32, Vec<u32>)>,
    /// The components, as lists of node indices in the order of their layers and rows.
    pub comps: Vec<Vec<usize>>,
    pub opts: Opts,
}

/// The settings `pathway.*`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opts {
    pub down: bool,
    pub col_gap: f64,
    pub row_gap: f64,
    pub max_angle: f64,
    pub min_arrow: f64,
    pub order: Order,
    pub margin: f64,
}

impl Opts {
    pub fn of(style: &Style) -> Opts {
        Opts {
            down: style.pathway_down,
            col_gap: style.pathway_col_gap,
            row_gap: style.pathway_row_gap,
            max_angle: style.pathway_max_angle,
            min_arrow: style.pathway_min_arrow,
            order: style.pathway_order,
            margin: style.pathway_margin,
        }
    }
}

/// The numeric data field `name` of a structure (first one in atom id order), when it is a finite number.
fn field(doc: &Document, atoms: &[u32], name: &str) -> Option<f64> {
    for &i in atoms {
        let a = doc.atom(i)?;
        for (k, v) in a.data() {
            if k.trim().eq_ignore_ascii_case(name) {
                return v.trim().parse::<f64>().ok().filter(|x| x.is_finite());
            }
        }
    }
    None
}

/// The width of the text of an arrow as the painter sets it (bond lengths): the font of the arrow text is 0.85 of the label font.
pub(crate) fn label_width(text: &str, style: &Style) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let m = style.metrics(1.0, false);
    crate::scene::width(&crate::steplabel::display(text), m.font * 0.85)
}

/// Transposes a box (the frame of `pathway.dir=down`).
fn flip(r: Rect) -> Rect {
    [r[1], r[0], r[3], r[2]]
}

/// The layout of the whole pathway of `doc` for the settings of `style` (`pathway.*`, the label ink and the route margin).
pub fn layout(doc: &Document, style: &Style) -> Result<Layout, Refusal> {
    let o = Opts::of(style);
    let anchored = (doc.objects.iter())
        .filter(|ob| ob.t == ObjKind::Arrow && ob.from != 0 && ob.to != 0)
        .count();
    if anchored > MAX_ARROWS {
        return Err(Refusal::Size);
    }
    // structures and the anchored arrows between two of them
    let pieces = crate::layout::pieces(doc);
    let mut owner: HashMap<u32, usize> = HashMap::new();
    let mut piece_atoms: Vec<Vec<u32>> = Vec::new();
    for (i, p) in pieces.iter().enumerate() {
        let mut ids = p.clone();
        ids.sort_unstable();
        for &a in &ids {
            owner.insert(a, i);
        }
        piece_atoms.push(ids);
    }
    let mut raw: Vec<(u32, usize, usize)> = Vec::new();
    let mut free: Vec<u32> = Vec::new();
    for ob in &doc.objects {
        let ends = (ob.t == ObjKind::Arrow)
            .then(|| (owner.get(&ob.from), owner.get(&ob.to)))
            .and_then(|e| match e {
                (Some(&u), Some(&v)) if ob.from != 0 && ob.to != 0 && u != v => Some((u, v)),
                _ => None,
            });
        match ends {
            Some((u, v)) => raw.push((ob.id, u, v)),
            None => free.push(ob.id),
        }
    }
    if raw.is_empty() {
        return Err(Refusal::Empty);
    }
    raw.sort_unstable();
    free.sort_unstable();
    // nodes in the order of their smallest atom id
    let mut used: Vec<usize> = raw.iter().flat_map(|r| [r.1, r.2]).collect();
    used.sort_by_key(|&p| piece_atoms[p][0]);
    used.dedup();
    if used.len() > MAX_NODES {
        return Err(Refusal::Size);
    }
    let index: HashMap<usize, usize> = used.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    let n = used.len();
    let mut nodes: Vec<Node> = used
        .iter()
        .map(|&p| Node {
            id: piece_atoms[p][0],
            atoms: piece_atoms[p].clone(),
            rect: piece_box(doc, &piece_atoms[p], style),
            layer: 0,
            comp: 0,
        })
        .collect();
    // the width of the text of each arrow, from one pass over the objects (not a lookup per edge)
    let label_of: HashMap<u32, f64> = (doc.objects.iter())
        .filter(|o| !o.text.is_empty())
        .map(|o| (o.id, label_width(&o.text, style)))
        .collect();
    let mut edges: Vec<Edge> = raw
        .iter()
        .map(|&(arrow, u, v)| Edge {
            arrow,
            from: index[&u],
            to: index[&v],
            primary: false,
            label: label_of.get(&arrow).copied().unwrap_or(0.0),
        })
        .collect();
    // refusals: a pair joined twice, a directed cycle
    for (i, e) in edges.iter().enumerate() {
        if let Some(f) = edges[..i].iter().find(|f| (f.from, f.to) == (e.from, e.to)) {
            return Err(Refusal::Duplicate(f.arrow.min(e.arrow)));
        }
    }
    let parents: Vec<Vec<usize>> = {
        let mut p = vec![Vec::new(); n];
        for (i, e) in edges.iter().enumerate() {
            p[e.to].push(i);
        }
        p
    };
    let kids: Vec<Vec<usize>> = {
        let mut k = vec![Vec::new(); n];
        for (i, e) in edges.iter().enumerate() {
            k[e.from].push(i);
        }
        k
    };
    // layers by the order of Kahn; what is left when it stops holds the cycles
    let mut indeg: Vec<usize> = parents.iter().map(Vec::len).collect();
    let mut queue: Vec<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        for &e in &kids[u] {
            let v = edges[e].to;
            nodes[v].layer = nodes[v].layer.max(nodes[u].layer + 1);
            indeg[v] -= 1;
            if indeg[v] == 0 {
                queue.push(v);
            }
        }
    }
    if queue.len() < n {
        return Err(Refusal::Cycle(cycle_arrow(n, &edges)));
    }
    // the order key of a node
    let key_of = |v: usize| -> Option<f64> {
        match o.order {
            Order::Mz => field(doc, &nodes[v].atoms, "MZ"),
            Order::Rt => field(doc, &nodes[v].atoms, "RT"),
            _ => None,
        }
    };
    let keys: Vec<Option<f64>> = (0..n).map(key_of).collect();
    let ids: Vec<u32> = nodes.iter().map(|nd| nd.id).collect();
    let cmp = |a: usize, b: usize| -> std::cmp::Ordering {
        let by_id = ids[a].cmp(&ids[b]);
        match o.order {
            Order::Id => by_id,
            Order::ReverseId => by_id.reverse(),
            Order::Mz | Order::Rt => match (keys[a], keys[b]) {
                (Some(x), Some(y)) => x.total_cmp(&y).then(by_id),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => by_id,
            },
        }
    };
    // the primary parent: the largest layer, then the order key, then the smallest atom id
    for ps in &parents {
        let mut best: Option<usize> = None;
        for &e in ps {
            let better = match best {
                None => true,
                Some(b) => {
                    let (lu, lb) = (nodes[edges[e].from].layer, nodes[edges[b].from].layer);
                    lu > lb
                        || (lu == lb
                            && cmp(edges[e].from, edges[b].from) == std::cmp::Ordering::Less)
                }
            };
            if better {
                best = Some(e);
            }
        }
        if let Some(b) = best {
            edges[b].primary = true;
        }
    }
    // components, with their roots in the order key; the components by their first root
    let mut comp_of: Vec<usize> = (0..n).collect();
    fn find(c: &mut [usize], mut v: usize) -> usize {
        while c[v] != v {
            c[v] = c[c[v]];
            v = c[v];
        }
        v
    }
    for e in &edges {
        let (a, b) = (find(&mut comp_of, e.from), find(&mut comp_of, e.to));
        if a != b {
            comp_of[a.max(b)] = a.min(b);
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    {
        let mut slot: HashMap<usize, usize> = HashMap::new();
        for v in 0..n {
            let r = find(&mut comp_of, v);
            let s = *slot.entry(r).or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
            groups[s].push(v);
        }
    }
    let roots_of = |g: &[usize]| -> Vec<usize> {
        let mut r: Vec<usize> = g
            .iter()
            .copied()
            .filter(|&v| parents[v].is_empty())
            .collect();
        r.sort_by(|&a, &b| cmp(a, b));
        r
    };
    let mut comps: Vec<(Vec<usize>, Vec<usize>)> = groups
        .into_iter()
        .map(|g| {
            let r = roots_of(&g);
            (g, r)
        })
        .collect();
    comps.sort_by(|a, b| {
        let (x, y) = (a.1[0], b.1[0]);
        if o.order == Order::ReverseId {
            ids[x].cmp(&ids[y])
        } else {
            cmp(x, y)
        }
    });
    for (ci, (g, _)) in comps.iter().enumerate() {
        for &v in g {
            nodes[v].comp = ci;
        }
    }
    // the frame: layers go along x; `down` transposes on the way in and out
    let frame = |r: Rect| if o.down { flip(r) } else { r };
    let rects: Vec<Rect> = nodes.iter().map(|nd| frame(nd.rect)).collect();
    let (fw, fh): (Vec<f64>, Vec<f64>) = rects.iter().map(|r| (r[2] - r[0], r[3] - r[1])).unzip();
    let fc: Vec<(f64, f64)> = rects
        .iter()
        .map(|r| ((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0))
        .collect();
    let mut centre = vec![(0.0, 0.0); n];
    let mut gaps: Vec<GapNote> = Vec::new();
    let mut limits: Vec<GapNote> = Vec::new();
    let tan_max = crate::m::tan(o.max_angle.to_radians());
    let (mut anchor_x, mut bottom) = (0.0, f64::MIN);
    let mut ordered: Vec<Vec<usize>> = Vec::new();
    for (ci, (g, roots)) in comps.iter().enumerate() {
        // local indices; the virtual root is the last one
        let m = g.len();
        let local: HashMap<usize, usize> = g.iter().enumerate().map(|(i, &v)| (v, i)).collect();
        let mut ch: Vec<Vec<usize>> = vec![Vec::new(); m + 1];
        for e in edges
            .iter()
            .filter(|e| e.primary && local.contains_key(&e.from))
        {
            ch[local[&e.from]].push(local[&e.to]);
        }
        for list in ch.iter_mut().take(m) {
            list.sort_by(|&a, &b| cmp(g[a], g[b]));
        }
        ch[m] = roots.iter().map(|r| local[r]).collect();
        let mut size: Vec<f64> = g.iter().map(|&v| fh[v]).collect();
        size.push(0.0);
        let ys = tidy(&ch, &size, o.row_gap);
        // columns
        let layers = g.iter().map(|&v| nodes[v].layer).max().unwrap_or(0);
        let wk: Vec<f64> = (0..=layers)
            .map(|k| {
                g.iter()
                    .filter(|&&v| nodes[v].layer == k)
                    .map(|&v| fw[v])
                    .fold(0.0, f64::max)
            })
            .collect();
        let mut xs = vec![0.0; layers + 1];
        let mut gk: Vec<f64> = Vec::with_capacity(layers);
        for k in 0..layers {
            let between: Vec<&Edge> = edges
                .iter()
                .filter(|e| e.primary && local.contains_key(&e.from) && nodes[e.from].layer == k)
                .collect();
            let slope = between
                .iter()
                .map(|e| {
                    let dy = (ys[local[&e.to]] - ys[local[&e.from]]).abs();
                    dy / tan_max * (1.0 + 1e-12) - wk[k] / 2.0 - wk[k + 1] / 2.0
                })
                .fold(f64::MIN, f64::max);
            let mut gap = o.col_gap;
            let mut rule = "";
            if slope > gap {
                gap = slope;
                rule = "slope";
            }
            let long_enough = |gap: f64, labels: bool| {
                let x1 = xs[k] + wk[k] / 2.0 + gap + wk[k + 1] / 2.0;
                between.iter().all(|e| {
                    let (u, v) = (e.from, e.to);
                    let a = [xs[k] - fw[u] / 2.0, ys[local[&u]] - fh[u] / 2.0, 0.0, 0.0];
                    let a = [a[0], a[1], a[0] + fw[u], a[1] + fh[u]];
                    let b = [x1 - fw[v] / 2.0, ys[local[&v]] - fh[v] / 2.0, 0.0, 0.0];
                    let b = [b[0], b[1], b[0] + fw[v], b[1] + fh[v]];
                    let need = if labels {
                        o.min_arrow.max(e.label)
                    } else {
                        o.min_arrow
                    };
                    visible(a, b, o.margin) >= need
                })
            };
            // the rule that raises the gap: `length` for the least visible length, then `label` for the room of the widest text
            for (labels, name) in [(false, "length"), (true, "label")] {
                if long_enough(gap, labels) {
                    continue;
                }
                let mut hi = gap.max(1.0);
                while !long_enough(hi, labels) && hi < 1e6 {
                    hi *= 2.0;
                }
                let mut lo = gap;
                for _ in 0..80 {
                    let mid = (lo + hi) / 2.0;
                    if long_enough(mid, labels) {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                gap = hi;
                rule = name;
            }
            // the rule `crossing` (`crossing_gap`): widen further until no primary arrow between the two layers meets the route box
            // of another structure of those two layers; at the bound the gap stays and the finding `std.scheme.gapLimit` says so
            {
                let col = |l: usize| -> Vec<(usize, Col)> {
                    g.iter()
                        .filter(|&&v| nodes[v].layer == l)
                        .map(|&v| (v, (ys[local[&v]], fw[v], fh[v])))
                        .collect()
                };
                let (cl, cr) = (col(k), col(k + 1));
                let (l, r): (Vec<Col>, Vec<Col>) = (
                    cl.iter().map(|e| e.1).collect(),
                    cr.iter().map(|e| e.1).collect(),
                );
                let index = |c: &[(usize, Col)]| -> HashMap<usize, usize> {
                    c.iter().enumerate().map(|(i, e)| (e.0, i)).collect()
                };
                let (il, ir) = (index(&cl), index(&cr));
                let pairs: Vec<(usize, usize)> =
                    between.iter().map(|e| (il[&e.from], ir[&e.to])).collect();
                match crossing_gap(&l, &r, &pairs, (wk[k], wk[k + 1]), gap, o.margin) {
                    Gap::Clean(0) => {}
                    Gap::Clean(n) => {
                        gap += f64::from(n) * CROSSING_STEP;
                        rule = "crossing";
                    }
                    Gap::Bound => limits.push(GapNote {
                        comp: ci,
                        k,
                        gap,
                        rule: "crossing",
                        arrows: between.iter().map(|e| e.arrow).collect(),
                    }),
                }
            }
            if !rule.is_empty() {
                gaps.push(GapNote {
                    comp: ci,
                    k,
                    gap,
                    rule,
                    arrows: between.iter().map(|e| e.arrow).collect(),
                });
            }
            gk.push(gap);
            xs[k + 1] = xs[k] + wk[k] / 2.0 + gap + wk[k + 1] / 2.0;
        }
        // the rule `crossing` for the secondary arrows and the arrows that skip a layer (`long_gaps`), on the columns as the gaps
        // above left them: the gaps that move such an arrow off a box it meets grow together until none meets a box; at the bound the state
        // with the fewest arrows still meeting a box is kept and the rest is the finding `std.scheme.gapLimit`
        {
            let mut cols: Vec<Vec<Col>> = vec![Vec::new(); layers + 1];
            let mut at: HashMap<usize, (usize, usize)> = HashMap::new();
            for &v in g {
                let l = nodes[v].layer;
                at.insert(v, (l, cols[l].len()));
                cols[l].push((ys[local[&v]], fw[v], fh[v]));
            }
            let spans: Vec<(&Edge, Long)> = edges
                .iter()
                .filter(|e| !e.primary && local.contains_key(&e.from))
                .map(|e| (e, (at[&e.from], at[&e.to])))
                .collect();
            let long: Vec<Long> = spans.iter().map(|s| s.1).collect();
            let note = |k: usize, rule: &'static str, gap: f64, arrows: Vec<u32>| GapNote {
                comp: ci,
                k,
                gap,
                rule,
                arrows,
            };
            let res = long_gaps(&cols, &wk, &gk, &long, o.margin);
            for (k, &n) in res.add.iter().enumerate().filter(|a| *a.1 > 0) {
                gk[k] += f64::from(n) * CROSSING_STEP;
                // the arrows of a widened gap: every arrow of the component that spans it
                let mut arrows: Vec<u32> = (edges.iter())
                    .filter(|e| local.contains_key(&e.from))
                    .filter(|e| nodes[e.from].layer <= k && nodes[e.to].layer > k)
                    .map(|e| e.arrow)
                    .collect();
                arrows.sort_unstable();
                match gaps.iter_mut().find(|n| n.comp == ci && n.k == k) {
                    Some(n) => *n = note(k, "crossing", gk[k], arrows),
                    None => gaps.push(note(k, "crossing", gk[k], arrows)),
                }
            }
            for k in 0..layers {
                xs[k + 1] = xs[k] + wk[k] / 2.0 + gk[k] + wk[k + 1] / 2.0;
            }
            gaps.sort_by_key(|n| (n.comp, n.k));
            // what the widening did not clear: the finding of the bound, per gap, with the arrows that span it and still meet a box
            for (k, &gap) in gk.iter().enumerate() {
                let mut arrows: Vec<u32> = (res.left.iter())
                    .filter(|&&i| long[i].0 .0 <= k && long[i].1 .0 > k)
                    .map(|&i| spans[i].0.arrow)
                    .collect();
                if !arrows.is_empty() {
                    arrows.sort_unstable();
                    limits.push(note(k, "crossing", gap, arrows));
                }
            }
        }
        // anchor and stacking: the first root of the first component keeps its place, the others go below
        let r0 = local[&roots[0]];
        let top = g
            .iter()
            .enumerate()
            .map(|(i, &v)| ys[i] - fh[v] / 2.0)
            .fold(f64::MAX, f64::min);
        let (dx, dy) = if ci == 0 {
            let c = fc[roots[0]];
            anchor_x = c.0;
            (c.0 - xs[nodes[roots[0]].layer], c.1 - ys[r0])
        } else {
            (anchor_x - xs[0], bottom + 2.0 * o.row_gap - top)
        };
        for (i, &v) in g.iter().enumerate() {
            centre[v] = (xs[nodes[v].layer] + dx, ys[i] + dy);
            bottom = bottom.max(centre[v].1 + fh[v] / 2.0);
        }
        // row order for the report: layer, then y
        let mut ord = g.clone();
        ord.sort_by(|&a, &b| (nodes[a].layer, nodes[a].id).cmp(&(nodes[b].layer, nodes[b].id)));
        ordered.push(ord);
    }
    // crossings, on the target positions
    let boxes: Vec<Rect> = (0..n)
        .map(|v| {
            [
                centre[v].0 - fw[v] / 2.0,
                centre[v].1 - fh[v] / 2.0,
                centre[v].0 + fw[v] / 2.0,
                centre[v].1 + fh[v] / 2.0,
            ]
        })
        .collect();
    let mut crossings = Vec::new();
    for e in &edges {
        let Some((p, q)) = clip(boxes[e.from], boxes[e.to], o.margin) else {
            continue;
        };
        let hit: Vec<u32> = (0..n)
            .filter(|&v| v != e.from && v != e.to)
            .filter(|&v| meets(p, q, boxes[v], o.margin))
            .map(|v| nodes[v].id)
            .collect();
        if !hit.is_empty() {
            crossings.push((e.arrow, hit));
        }
    }
    // back out of the frame
    let centres: Vec<(f64, f64)> = centre
        .into_iter()
        .map(|c| if o.down { (c.1, c.0) } else { c })
        .collect();
    Ok(Layout {
        nodes,
        edges,
        centres,
        gaps,
        limits,
        free,
        crossings,
        comps: ordered,
        opts: o,
    })
}

/// A box of a column of the rule `crossing`: the centre y, the width and the height.
pub type Col = (f64, f64, f64);

/// What `crossing_gap` found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gap {
    /// No primary arrow meets another box after this many steps of `CROSSING_STEP` (0: none to begin with).
    Clean(u32),
    /// `CROSSING_STEPS` steps did not clear it: the gap must stay as it was.
    Bound,
}

/// The rule `crossing` of the gap between two columns, on its own so that its cost can be tested: the boxes `left` are centred at
/// x = 0, the boxes `right` at x = `widths.0 / 2 + gap + widths.1 / 2` (`widths`: the widest box of each column); `edges` are the
/// primary arrows as (index in `left`, index in `right`). For `n = 0, 1, ...` it checks that no arrow, clipped at the route boxes
/// of its two ends, meets the route box (inflated by `margin`) of another box of the two columns, and returns the first `n` that
/// does, up to `CROSSING_STEPS`. The columns are sorted by y once; an arrow is tested only against the boxes whose y range meets its
/// own range inside the x slab of a column (a binary search), so one check costs O(E (log P + hits)) and stops at the first arrow
/// that meets a box; the whole rule costs at most `CROSSING_STEPS + 1` checks plus one sort, O(P log P).
pub fn crossing_gap(
    left: &[Col],
    right: &[Col],
    edges: &[(usize, usize)],
    widths: (f64, f64),
    gap: f64,
    margin: f64,
) -> Gap {
    let (ol, or) = (sorted_by_y(left), sorted_by_y(right));
    let tall = |c: &[Col]| c.iter().map(|b| b.2).fold(0.0, f64::max) / 2.0 + margin;
    let (hl, hr) = (tall(left), tall(right));
    let rect = |b: Col, x: f64| {
        [
            x - b.1 / 2.0,
            b.0 - b.2 / 2.0,
            x + b.1 / 2.0,
            b.0 + b.2 / 2.0,
        ]
    };
    let clean = |gap: f64| -> bool {
        let x1 = widths.0 / 2.0 + gap + widths.1 / 2.0;
        edges.iter().all(|&(u, v)| {
            let Some((p, q)) = clip(rect(left[u], 0.0), rect(right[v], x1), margin) else {
                return true;
            };
            near(left, &ol, 0.0, widths.0, hl, margin, p, q)
                .into_iter()
                .filter(|&i| i != u)
                .all(|i| !meets(p, q, rect(left[i], 0.0), margin))
                && near(right, &or, x1, widths.1, hr, margin, p, q)
                    .into_iter()
                    .filter(|&i| i != v)
                    .all(|i| !meets(p, q, rect(right[i], x1), margin))
        })
    };
    (0..=CROSSING_STEPS)
        .find(|&n| clean(gap + f64::from(n) * CROSSING_STEP))
        .map_or(Gap::Bound, Gap::Clean)
}

/// The candidates of one column: the indices of the boxes whose y is within reach (`h`: half the tallest box plus the margin) of the
/// part of the segment `p`–`q` inside the x slab of the column (centred at `x`, `w` wide), by a binary search in the column sorted
/// by y (`order`).
#[allow(clippy::too_many_arguments)]
fn near(
    col: &[Col],
    order: &[usize],
    x: f64,
    w: f64,
    h: f64,
    margin: f64,
    p: (f64, f64),
    q: (f64, f64),
) -> Vec<usize> {
    let (lo, hi) = (x - w / 2.0 - margin, x + w / 2.0 + margin);
    let dx = q.0 - p.0;
    let (t0, t1) = if dx.abs() < 1e-12 {
        if p.0 < lo || p.0 > hi {
            return Vec::new();
        }
        (0.0, 1.0)
    } else {
        let (a, b) = ((lo - p.0) / dx, (hi - p.0) / dx);
        (a.min(b).max(0.0), a.max(b).min(1.0))
    };
    if t0 > t1 {
        return Vec::new();
    }
    let (y0, y1) = (p.1 + t0 * (q.1 - p.1), p.1 + t1 * (q.1 - p.1));
    let (ylo, yhi) = (y0.min(y1) - h, y0.max(y1) + h);
    let first = order.partition_point(|&i| col[i].0 < ylo);
    order[first..]
        .iter()
        .copied()
        .take_while(|&i| col[i].0 <= yhi)
        .collect()
}

/// The indices of a column in the order of the centre y (ties by index).
fn sorted_by_y(c: &[Col]) -> Vec<usize> {
    let mut o: Vec<usize> = (0..c.len()).collect();
    o.sort_by(|&a, &b| c[a].0.total_cmp(&c[b].0).then(a.cmp(&b)));
    o
}

/// An arrow the rule `crossing` of the long arrows looks at: a secondary arrow, or one that skips a layer, as
/// (layer, index in that column) of its two ends; `a < b` always (a layer is the longest path from a root).
pub type Long = ((usize, usize), (usize, usize));

/// What `long_gaps` found: the steps to add to each gap (`add[k]` steps of `CROSSING_STEP` to the gap between layers `k` and `k + 1`,
/// all zero when nothing helps or nothing was wrong) and the long arrows (indices into the list given) that still meet a box with
/// those gaps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LongGaps {
    pub add: Vec<u32>,
    pub left: Vec<usize>,
}

/// The gaps (as a range of `k`, the gap between layers `k` and `k + 1`) that move an arrow from layer `ends.0` to layer `ends.1` off
/// the boxes it meets in layer `layer`, the nearest ones: `offs` are, for each, the height of the arrow at that layer minus the
/// centre of the box; `rise` is the arrow's rise. A box in an end column is cleared by a flatter arrow (all spanned gaps wider); a
/// box in a layer between by moving the height of the arrow at that layer away from the box centre, which is a wider part before
/// the layer (the arrow is nearer its end) or after it (nearer its start). The sign of `rise * off` is the same for an arrow and
/// its mirror image in y; a horizontal arrow, or one through the centre, has no side, so every spanned gap grows; boxes on both
/// sides give the union, so no index decides.
pub fn widen_range(
    ends: (usize, usize),
    layer: usize,
    rise: f64,
    offs: &[f64],
) -> std::ops::Range<usize> {
    let (la, lb) = ends;
    let mut range: Option<std::ops::Range<usize>> = None;
    for &off in offs {
        let r = if layer == la || layer == lb || rise.abs() < 1e-9 || off.abs() < 1e-9 {
            la..lb
        } else if rise * off > 0.0 {
            la..layer
        } else {
            layer..lb
        };
        range = Some(match range {
            None => r,
            Some(o) => o.start.min(r.start)..o.end.max(r.end),
        });
    }
    range.unwrap_or(la..la)
}

/// The y of the line through `p` and `q` at `x`.
fn line_y(p: (f64, f64), q: (f64, f64), x: f64) -> f64 {
    if (q.0 - p.0).abs() < 1e-12 {
        p.1
    } else {
        p.1 + (x - p.0) / (q.0 - p.0) * (q.1 - p.1)
    }
}

/// The rule `crossing` for the secondary arrows and the arrows that skip a layer, on its own so that its cost can be tested. `cols`
/// are the columns (layer by layer), `widths` the widest box of each, `gaps` the gaps between them as the other rules left them
/// (`gaps[k]` between `k` and `k + 1`), `long` the arrows. In each round every arrow, clipped at the route boxes of its two ends,
/// is tested against the route box (inflated by `margin`) of every other box of the columns it spans, ends included; the arrows
/// that meet one widen every gap they span by one `CROSSING_STEP` together, so the result does not depend on the order of the
/// arrows, and a layout and its mirror image in y get the same gaps. Arrows that do not meet a box are not looked at again once
/// they clear (but are checked again each round: a wider gap can bring a box into the way). At most `CROSSING_STEPS` rounds.
/// The columns are sorted by y once; a test costs O(log P + hits) per spanned column.
pub fn long_gaps(
    cols: &[Vec<Col>],
    widths: &[f64],
    gaps: &[f64],
    long: &[Long],
    margin: f64,
) -> LongGaps {
    let order: Vec<Vec<usize>> = cols.iter().map(|c| sorted_by_y(c)).collect();
    let tall: Vec<f64> = cols
        .iter()
        .map(|c| c.iter().map(|b| b.2).fold(0.0, f64::max) / 2.0 + margin)
        .collect();
    let mut add = vec![0u32; gaps.len()];
    let rect = |b: Col, x: f64| {
        [
            x - b.1 / 2.0,
            b.0 - b.2 / 2.0,
            x + b.1 / 2.0,
            b.0 + b.2 / 2.0,
        ]
    };
    let mut hits: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
    let mut best: Option<(Vec<u32>, Vec<usize>)> = None;
    for round in 0..=CROSSING_STEPS {
        let mut xs = vec![0.0; cols.len()];
        for k in 1..cols.len() {
            xs[k] = xs[k - 1]
                + widths[k - 1] / 2.0
                + gaps[k - 1]
                + f64::from(add[k - 1]) * CROSSING_STEP
                + widths[k] / 2.0;
        }
        hits.clear();
        for (i, &((la, ia), (lb, ib))) in long.iter().enumerate() {
            let Some((p, q)) = clip(
                rect(cols[la][ia], xs[la]),
                rect(cols[lb][ib], xs[lb]),
                margin,
            ) else {
                continue;
            };
            // the first layer with a box the arrow meets; in it the boxes whose centre is nearest the arrow (all of them when
            // several are equally near: no index decides, so a layout and its mirror image give the same result)
            let mut found: Vec<(usize, f64)> = Vec::new();
            let mut layer = la;
            for l in la..=lb {
                layer = l;
                for j in near(&cols[l], &order[l], xs[l], widths[l], tall[l], margin, p, q) {
                    if (l == la && j == ia) || (l == lb && j == ib) {
                        continue;
                    }
                    if meets(p, q, rect(cols[l][j], xs[l]), margin) {
                        found.push((j, line_y(p, q, xs[l]) - cols[l][j].0));
                    }
                }
                if !found.is_empty() {
                    break;
                }
            }
            let nearest = found.iter().map(|f| f.1.abs()).fold(f64::MAX, f64::min);
            let offs: Vec<f64> = (found.iter())
                .map(|f| f.1)
                .filter(|o| o.abs() <= nearest + 1e-9)
                .collect();
            let range = widen_range((la, lb), layer, q.1 - p.1, &offs);
            if !range.is_empty() {
                hits.push((i, range));
            }
        }
        // the state kept is the first with the fewest arrows still meeting a box (round 0, the gaps as they were, is a candidate)
        if best.as_ref().is_none_or(|b| hits.len() < b.1.len()) {
            best = Some((add.clone(), hits.iter().map(|h| h.0).collect()));
        }
        if hits.is_empty() || round == CROSSING_STEPS {
            break;
        }
        let mut spanned = vec![false; gaps.len()];
        for (_, r) in &hits {
            spanned[r.clone()].iter_mut().for_each(|s| *s = true);
        }
        for (a, s) in add.iter_mut().zip(&spanned) {
            *a += u32::from(*s);
        }
    }
    let (add, left) = best.unwrap_or_default();
    LongGaps { add, left }
}

/// The length of the visible segment between two boxes (0 when nothing is left).
fn visible(a: Rect, b: Rect, margin: f64) -> f64 {
    clip(a, b, margin).map_or(0.0, |(p, q)| crate::m::hypot(q.0 - p.0, q.1 - p.1))
}

/// Whether the segment `p`–`q` meets the interior of `b` inflated by `margin` (touching does not count).
fn meets(p: (f64, f64), q: (f64, f64), b: Rect, margin: f64) -> bool {
    const EPS: f64 = 1e-9;
    let lo = [b[0] - margin + EPS, b[1] - margin + EPS];
    let hi = [b[2] + margin - EPS, b[3] + margin - EPS];
    let (d, s) = ([q.0 - p.0, q.1 - p.1], [p.0, p.1]);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for i in 0..2 {
        if d[i].abs() < 1e-15 {
            if s[i] <= lo[i] || s[i] >= hi[i] {
                return false;
            }
        } else {
            let (a, c) = ((lo[i] - s[i]) / d[i], (hi[i] - s[i]) / d[i]);
            t0 = t0.max(a.min(c));
            t1 = t1.min(a.max(c));
        }
    }
    t0 < t1
}

/// The smallest arrow id on a directed cycle: the edges inside a strongly connected component of more than one node.
fn cycle_arrow(n: usize, edges: &[Edge]) -> u32 {
    struct S<'a> {
        adj: &'a [Vec<usize>],
        idx: Vec<Option<usize>>,
        low: Vec<usize>,
        on: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        comp: Vec<usize>,
        count: usize,
    }
    fn go(s: &mut S, v: usize) {
        s.idx[v] = Some(s.next);
        s.low[v] = s.next;
        s.next += 1;
        s.stack.push(v);
        s.on[v] = true;
        let adj = s.adj;
        for &w in &adj[v] {
            match s.idx[w] {
                None => {
                    go(s, w);
                    s.low[v] = s.low[v].min(s.low[w]);
                }
                Some(i) if s.on[w] => s.low[v] = s.low[v].min(i),
                _ => {}
            }
        }
        if Some(s.low[v]) == s.idx[v] {
            while let Some(w) = s.stack.pop() {
                s.on[w] = false;
                s.comp[w] = s.count;
                if w == v {
                    break;
                }
            }
            s.count += 1;
        }
    }
    let mut adj = vec![Vec::new(); n];
    for e in edges {
        adj[e.from].push(e.to);
    }
    let mut s = S {
        adj: &adj,
        idx: vec![None; n],
        low: vec![0; n],
        on: vec![false; n],
        stack: Vec::new(),
        next: 0,
        comp: vec![0; n],
        count: 0,
    };
    for v in 0..n {
        if s.idx[v].is_none() {
            go(&mut s, v);
        }
    }
    edges
        .iter()
        .filter(|e| s.comp[e.from] == s.comp[e.to])
        .map(|e| e.arrow)
        .min()
        .unwrap_or(0)
}

/// The tidy tree of Buchheim, Jünger and Leipert on a rooted ordered tree whose last node (`ch.len() - 1`) is the root: the position
/// of every node across the layers, neighbours in a layer `gap` apart between their boxes (`size` is the extent of a box across).
fn tidy(ch: &[Vec<usize>], size: &[f64], gap: f64) -> Vec<f64> {
    let n = ch.len();
    let root = n - 1;
    let mut t = Tree {
        ch,
        size,
        gap,
        par: vec![usize::MAX; n],
        num: vec![0; n],
        prelim: vec![0.0; n],
        modv: vec![0.0; n],
        shift: vec![0.0; n],
        change: vec![0.0; n],
        thread: vec![None; n],
        anc: (0..n).collect(),
    };
    for (v, list) in ch.iter().enumerate() {
        for (i, &w) in list.iter().enumerate() {
            t.par[w] = v;
            t.num[w] = i;
        }
    }
    t.first_walk(root);
    let mut y = vec![0.0; n];
    t.second_walk(root, 0.0, &mut y);
    y.truncate(n - 1);
    y
}

struct Tree<'a> {
    ch: &'a [Vec<usize>],
    size: &'a [f64],
    gap: f64,
    par: Vec<usize>,
    num: Vec<usize>,
    prelim: Vec<f64>,
    modv: Vec<f64>,
    shift: Vec<f64>,
    change: Vec<f64>,
    thread: Vec<Option<usize>>,
    anc: Vec<usize>,
}

impl Tree<'_> {
    fn dist(&self, a: usize, b: usize) -> f64 {
        (self.size[a] + self.size[b]) / 2.0 + self.gap
    }

    fn left_sibling(&self, v: usize) -> Option<usize> {
        (self.num[v] > 0).then(|| self.ch[self.par[v]][self.num[v] - 1])
    }

    fn next_left(&self, v: usize) -> Option<usize> {
        self.ch[v].first().copied().or(self.thread[v])
    }

    fn next_right(&self, v: usize) -> Option<usize> {
        self.ch[v].last().copied().or(self.thread[v])
    }

    fn first_walk(&mut self, v: usize) {
        let kids = self.ch[v].clone();
        if kids.is_empty() {
            self.prelim[v] = self
                .left_sibling(v)
                .map_or(0.0, |w| self.prelim[w] + self.dist(v, w));
            return;
        }
        let mut default = kids[0];
        for &w in &kids {
            self.first_walk(w);
            default = self.apportion(w, default);
        }
        self.execute_shifts(v);
        let mid = (self.prelim[kids[0]] + self.prelim[kids[kids.len() - 1]]) / 2.0;
        match self.left_sibling(v) {
            Some(w) => {
                self.prelim[v] = self.prelim[w] + self.dist(v, w);
                self.modv[v] = self.prelim[v] - mid;
            }
            None => self.prelim[v] = mid,
        }
    }

    fn apportion(&mut self, v: usize, mut default: usize) -> usize {
        let Some(w) = self.left_sibling(v) else {
            return default;
        };
        let (mut vir, mut vor, mut vil) = (v, v, w);
        let mut vol = self.ch[self.par[v]][0];
        let (mut sir, mut sor) = (self.modv[vir], self.modv[vor]);
        let (mut sil, mut sol) = (self.modv[vil], self.modv[vol]);
        while let (Some(a), Some(b)) = (self.next_right(vil), self.next_left(vir)) {
            vil = a;
            vir = b;
            let (Some(l), Some(r)) = (self.next_left(vol), self.next_right(vor)) else {
                break;
            };
            vol = l;
            vor = r;
            self.anc[vor] = v;
            let shift = self.prelim[vil] + sil - (self.prelim[vir] + sir) + self.dist(vil, vir);
            if shift > 0.0 {
                let wl = if self.par[self.anc[vil]] == self.par[v] {
                    self.anc[vil]
                } else {
                    default
                };
                self.move_subtree(wl, v, shift);
                sir += shift;
                sor += shift;
            }
            sil += self.modv[vil];
            sir += self.modv[vir];
            sol += self.modv[vol];
            sor += self.modv[vor];
        }
        if let (Some(nr), None) = (self.next_right(vil), self.next_right(vor)) {
            self.thread[vor] = Some(nr);
            self.modv[vor] += sil - sor;
        }
        if let (Some(nl), None) = (self.next_left(vir), self.next_left(vol)) {
            self.thread[vol] = Some(nl);
            self.modv[vol] += sir - sol;
            default = v;
        }
        default
    }

    fn move_subtree(&mut self, wl: usize, wr: usize, shift: f64) {
        let subtrees = (self.num[wr] - self.num[wl]) as f64;
        self.change[wr] -= shift / subtrees;
        self.shift[wr] += shift;
        self.change[wl] += shift / subtrees;
        self.prelim[wr] += shift;
        self.modv[wr] += shift;
    }

    fn execute_shifts(&mut self, v: usize) {
        let (mut shift, mut change) = (0.0, 0.0);
        for i in (0..self.ch[v].len()).rev() {
            let w = self.ch[v][i];
            self.prelim[w] += shift;
            self.modv[w] += shift;
            change += self.change[w];
            shift += self.shift[w] + change;
        }
    }

    fn second_walk(&self, v: usize, m: f64, y: &mut [f64]) {
        y[v] = self.prelim[v] + m;
        for &w in &self.ch[v] {
            self.second_walk(w, m + self.modv[v], y);
        }
    }
}

/// A displacement on the grid of the document (4 decimals). The centre of a box is often on a half step of the grid, so a displacement
/// can be a tie: ties go toward zero (the residual of a moved box is then at most half a step and, when it is exactly half, rounds to
/// no move), which makes the command idempotent (P4). Values within 1e-9 of a tie count as ties.
fn displacement(v: f64) -> f64 {
    let q = (v.abs() * 1e4 * 1e5).round() / 1e5;
    let n = if q - q.floor() > 0.5 {
        q.ceil()
    } else {
        q.floor()
    };
    round4(n.copysign(v) / 1e4)
}

/// The operations of a layout: one `move` per structure whose rounded displacement is not zero, ordered by the structure's smallest
/// atom id; then, ordered by arrow id, the routing operations of every edge on the moved document (so that they are exactly what a
/// later route of the arrows would compute).
pub fn ops(doc: &Document, style: &Style, lay: &Layout) -> Result<Vec<Op>, Refusal> {
    let mut out = Vec::new();
    for (nd, c) in lay.nodes.iter().zip(&lay.centres) {
        let (cx, cy) = (
            (nd.rect[0] + nd.rect[2]) / 2.0,
            (nd.rect[1] + nd.rect[3]) / 2.0,
        );
        let (dx, dy) = (displacement(c.0 - cx), displacement(c.1 - cy));
        if dx != 0.0 || dy != 0.0 {
            out.push(Op::Move {
                atoms: nd.atoms.clone(),
                objects: Vec::new(),
                dx,
                dy,
            });
        }
    }
    let mut moved = doc.clone();
    // a layout that leaves the page (a coordinate past `MAX_COORD`) is refused with the key of the operation, never dropped
    if let Err(e) = moved.apply(&out) {
        return Err(Refusal::Apply(e.key, e.id));
    }
    let boxes: Vec<Rect> = lay
        .nodes
        .iter()
        .map(|nd| piece_box(&moved, &nd.atoms, style))
        .collect();
    for e in &lay.edges {
        if let Some(o) = moved.object(e.arrow) {
            out.extend(route_between(
                o,
                boxes[e.from],
                boxes[e.to],
                lay.opts.margin,
            ));
        }
    }
    Ok(out)
}

/// The findings of a layout for the report `std/1`: `std.scheme.gapRaised` (info, one per raised gap: the first root of the
/// component and the primary arrows between the two layers), `std.scheme.free` (info, the objects that are not moved) and
/// `std.scheme.crossing` (warn, one per arrow: the arrow and the structures it meets).
pub fn findings(lay: &Layout) -> Vec<Finding> {
    let mut out = Vec::new();
    for g in &lay.gaps {
        let first = lay.comps[g.comp]
            .iter()
            .map(|&v| lay.nodes[v].id)
            .min()
            .unwrap_or(0);
        let mut f = Finding::new("std.scheme.gapRaised", &[first], &[]);
        f.objects = g.arrows.clone();
        f.objects.sort_unstable();
        out.push(f);
    }
    for g in &lay.limits {
        let first = lay.comps[g.comp]
            .iter()
            .map(|&v| lay.nodes[v].id)
            .min()
            .unwrap_or(0);
        let mut f = Finding::new("std.scheme.gapLimit", &[first], &[]);
        f.objects = g.arrows.clone();
        f.objects.sort_unstable();
        out.push(f);
    }
    if !lay.free.is_empty() {
        let mut f = Finding::new("std.scheme.free", &[], &[]);
        f.objects = lay.free.clone();
        out.push(f);
    }
    for (arrow, hit) in &lay.crossings {
        let mut f = Finding::new("std.scheme.crossing", hit, &[]);
        f.objects = vec![*arrow];
        out.push(f);
    }
    out
}

/// The report `std/1` of a drawing with the findings of its pathway layout (none when the drawing has no pathway to lay out).
pub fn report_with(doc: &Document, style: &Style) -> Report {
    let mut r = crate::conform::report_with(doc, style.hash_lines);
    if let Ok(lay) = layout(doc, style) {
        r.checks.extend(findings(&lay));
    }
    // the pairs the diff lens cannot compare (an atom without a map); a drawing too large for the lens has no such finding
    if let Ok(l) = crate::tp::lenses(doc) {
        r.checks.extend(l.findings());
    }
    crate::conform::sort_findings(&mut r.checks);
    r
}

/// The command `pathway:tree`: the operations of one undo step, or the refusal.
pub fn command(doc: &Document, style: &Style) -> Result<Vec<Op>, Refusal> {
    let lay = layout(doc, style)?;
    ops(doc, style, &lay)
}

/// `boxes/1`: the boxes the layout uses, label ink included, in bond lengths at 4 decimals: every structure by its smallest atom id,
/// every arrow with the structures it joins (0 for a free end), both ordered by id.
pub fn boxes_json(doc: &Document, style: &Style) -> String {
    use serde_json::{json, Value};
    let num = |v: f64| Value::from(round4(v));
    let pieces = crate::layout::pieces(doc);
    let mut owner: HashMap<u32, u32> = HashMap::new();
    let mut structures: Vec<(u32, Rect)> = Vec::new();
    for p in &pieces {
        let mut ids = p.clone();
        ids.sort_unstable();
        for &a in &ids {
            owner.insert(a, ids[0]);
        }
        structures.push((ids[0], piece_box(doc, &ids, style)));
    }
    structures.sort_by_key(|s| s.0);
    let structures: Vec<Value> = structures
        .iter()
        .map(|(id, r)| {
            json!({"id": id, "x0": num(r[0]), "y0": num(r[1]), "x1": num(r[2]), "y1": num(r[3])})
        })
        .collect();
    let mut arrows: Vec<&crate::doc::Obj> = doc
        .objects
        .iter()
        .filter(|o| o.t == ObjKind::Arrow)
        .collect();
    arrows.sort_by_key(|o| o.id);
    let arrows: Vec<Value> = arrows
        .iter()
        .map(|o| {
            let s = |a: u32| owner.get(&a).copied().unwrap_or(0);
            json!({"id": o.id, "from": s(o.from), "to": s(o.to),
                "x": num(o.x), "y": num(o.y), "x2": num(o.x2), "y2": num(o.y2)})
        })
        .collect();
    json!({"format": "boxes/1", "structures": structures, "arrows": arrows}).to_string()
}
