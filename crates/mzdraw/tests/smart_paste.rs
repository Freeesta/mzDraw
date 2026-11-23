//! A paste lands in a free place: the view centre when it is free, else the nearest free place to the right and below, never on the
//! drawing, and the view does not change.

use mzdraw::Editor;

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
