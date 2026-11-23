//! A paste lands in a free place: the view centre when it is free, else the nearest free place to the right and below, never on the
//! drawing, and the view does not change.

mod common;

use mzdraw::{Document, Editor};
use serde_json::json;

const BENZENE: &str = "c1ccccc1";

fn atoms(e: &Editor) -> Vec<(u32, f64, f64)> {
    e.doc().atoms.iter().map(|a| (a.id, a.x, a.y)).collect()
}

fn centre_of(e: &Editor, ids: &[u32]) -> (f64, f64) {
    let p: Vec<_> = (e.doc().atoms.iter())
        .filter(|a| ids.contains(&a.id))
        .collect();
    let n = p.len() as f64;
    (
        p.iter().map(|a| a.x).sum::<f64>() / n,
        p.iter().map(|a| a.y).sum::<f64>() / n,
    )
}

#[test]
fn a_paste_into_an_empty_view_lands_at_its_centre() {
    let mut e = Editor::default();
    e.fit(800.0, 500.0);
    e.pan(120.0, -40.0);
    let want = e.view.to_doc(400.0, 250.0);
    e.paste(Some(BENZENE));
    let ids: Vec<u32> = e.doc().atoms.iter().map(|a| a.id).collect();
    let c = centre_of(&e, &ids);
    assert!(
        (c.0 - want.0).abs() < 1e-3 && (c.1 - want.1).abs() < 1e-3,
        "{c:?} {want:?}"
    );
}

#[test]
fn a_second_paste_does_not_overlap_the_first_and_the_view_stays() {
    let mut e = Editor::default();
    e.fit(800.0, 500.0);
    e.zoom(300.0, 200.0, 1.7);
    let view = e.view;
    e.paste(Some(BENZENE));
    let first = atoms(&e);
    e.paste(Some(BENZENE));
    e.paste(Some(BENZENE));
    assert_eq!(e.doc().atoms.len(), 18);
    assert_eq!(e.view, view, "a paste does not rezoom or pan");
    let all = atoms(&e);
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            let d = (a.1 - b.1).hypot(a.2 - b.2);
            assert!(d > 0.5, "atoms {} and {} are {d} apart", a.0, b.0);
        }
    }
    // the later pastes are to the right and below the first, and each ring is intact
    for ring in all.chunks(6).skip(1) {
        assert!(ring.iter().all(|a| a.1 >= first[0].1 - 1.8));
    }
    // deterministic: the same session again gives the same document
    let mut f = Editor::default();
    f.fit(800.0, 500.0);
    f.zoom(300.0, 200.0, 1.7);
    for _ in 0..3 {
        f.paste(Some(BENZENE));
    }
    assert_eq!(f.doc().to_json(), e.doc().to_json());
}

#[test]
fn a_paste_keeps_clear_of_a_drawing_under_the_centre() {
    let mut e = Editor::default();
    e.paste(Some(BENZENE));
    assert_eq!(e.paste(None), 0, "nothing was copied");
    e.select_all();
    e.copy();
    e.paste(None);
    e.paste(None);
    let all = atoms(&e);
    assert!(all.len() >= 18);
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert!((a.1 - b.1).hypot(a.2 - b.2) > 0.5);
        }
    }
}

#[test]
fn a_paste_is_one_undo_step() {
    let mut e = Editor::default();
    e.paste(Some(BENZENE));
    let one = e.doc().to_json();
    e.paste(Some(BENZENE));
    e.undo();
    assert_eq!(e.doc().to_json(), one);
}

/// A document of atoms only, one per grid point `x0, x0 + 2, ... <= x1` by `y0, y0 + 2, ... <= y1`.
fn grid(x0: i32, x1: i32, y0: i32, y1: i32) -> String {
    let mut atoms = Vec::new();
    let mut id = 1;
    for y in (y0..=y1).step_by(2) {
        for x in (x0..=x1).step_by(2) {
            atoms.push(json!({"id": id, "el": "C", "x": x, "y": y}));
            id += 1;
        }
    }
    json!({"format": "mzdraw/1", "next": id, "atoms": atoms, "bonds": [], "objects": []})
        .to_string()
}

fn pasted_clear(e: &mut Editor, text: &str) -> Document {
    let old = e.doc().clone();
    e.paste(Some(text));
    common::assert_clear(
        &common::occupied(&old, 0),
        &common::occupied(e.doc(), old.next - 1),
    );
    e.doc().clone()
}

#[test]
fn the_first_places_are_exact_and_deterministic() {
    let mut e = Editor::default();
    e.paste(Some(BENZENE));
    let ring: Vec<(f64, f64)> = e.doc().atoms.iter().map(|a| (a.x, a.y)).collect();
    assert_eq!(
        ring[0],
        (0.0, 1.0),
        "the ring is centred on the view centre"
    );
    let old = e.doc().clone();
    let d = pasted_clear(&mut e, BENZENE);
    // the second ring: the nearest free cell to the right and below (three bonds to the right: the tie goes to the right)
    assert_eq!(
        (d.atoms[6].x - old.atoms[0].x, d.atoms[6].y - old.atoms[0].y),
        (3.0, 0.0)
    );
}

#[test]
fn a_paste_with_an_arrow_and_a_text_keeps_clear_of_a_drawing() {
    let mut e = Editor::default();
    e.paste(Some(BENZENE));
    e.paste(Some(BENZENE));
    let part = json!({"format": "mzdraw/1", "next": 10,
        "atoms": [{"id": 1, "el": "C", "x": 0, "y": 0}, {"id": 2, "el": "O", "x": 1, "y": 0}],
        "bonds": [{"id": 3, "a": 1, "b": 2, "o": 1}],
        "objects": [{"id": 4, "t": "arrow", "x": 0, "y": -1.5, "x2": 3, "y2": -1.5},
                    {"id": 5, "t": "text", "x": 1.5, "y": 1.5, "text": "label"}]})
    .to_string();
    let d = pasted_clear(&mut e, &part);
    assert_eq!((d.atoms.len(), d.objects.len()), (14, 2));
    // the limit, stated: a text is one anchor point here, its drawn extent is not reserved (a long text may reach a neighbour)
    assert_eq!(d.objects[1].text, "label");
}

#[test]
fn a_part_wider_than_eighty_bonds_lands_at_the_centre_or_clear_of_the_drawing() {
    let chain = grid(-46, 46, 0, 0); // 93 bonds across
    let mut e = Editor::default();
    e.paste(Some(&chain));
    assert_eq!(e.doc().atoms.len(), 47);
    let xs: Vec<f64> = e.doc().atoms.iter().map(|a| a.x).collect();
    let (lo, hi) = (
        xs.iter().cloned().fold(f64::MAX, f64::min),
        xs.iter().cloned().fold(f64::MIN, f64::max),
    );
    assert!(hi - lo > 80.0);
    assert!(((lo + hi) / 2.0).abs() < 1e-3, "centred on the view centre");
    // over a drawing it goes below it
    let mut f = Editor::default();
    f.paste(Some(BENZENE));
    pasted_clear(&mut f, &chain);
}

#[test]
fn when_the_quadrant_is_full_the_nearest_free_place_in_any_direction_is_taken() {
    // a net of atoms two bonds apart over the whole quadrant right and below the centre, to 42 bonds: no cell of it is free
    let mut e = Editor::default();
    e.import(&grid(-2, 42, -42, 2)).unwrap();
    let d = pasted_clear(&mut e, BENZENE);
    let c: (f64, f64) = {
        let n = d.atoms.len() - 6;
        let p = &d.atoms[n..];
        (
            p.iter().map(|a| a.x).sum::<f64>() / 6.0,
            p.iter().map(|a| a.y).sum::<f64>() / 6.0,
        )
    };
    assert!(c.0 < -2.0 || c.1 > 2.0, "left of or above the net: {c:?}");
}

#[test]
fn when_no_cell_is_free_the_part_goes_to_the_right_of_the_drawing() {
    let mut e = Editor::default();
    e.import(&grid(-44, 44, -44, 44)).unwrap();
    let right = e.doc().atoms.iter().map(|a| a.x).fold(f64::MIN, f64::max);
    let d = pasted_clear(&mut e, BENZENE);
    let new = &d.atoms[d.atoms.len() - 6..];
    let left = new.iter().map(|a| a.x).fold(f64::MAX, f64::min);
    assert!(
        (left - (right + 2.0)).abs() < 1e-3,
        "two bonds right of the drawing: {left} {right}"
    );
}
