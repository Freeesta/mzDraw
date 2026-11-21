//! The time of one stereo signature of a piece of 100 atoms with 10 wedges, and of one guarded mirror of it (median of 20), for the release build:
//! `cargo run --release --locked -p mzdraw --example stereo_bench`.

use mzdraw::stereo_guard::{before, changed, pieces, signature};
use mzdraw::{Axis, Op, Session};

fn main() {
    let mut s = Session::default();
    let mut ops = Vec::new();
    for i in 0..50 {
        ops.push(Op::AtomAdd {
            id: None,
            el: "C".into(),
            x: i as f64,
            y: 0.0,
            map: None,
        });
    }
    for i in 0..50u32 {
        let (y, el) = (
            if i % 2 == 0 { 1.0 } else { -1.0 },
            if i % 5 == 0 { "F" } else { "O" },
        );
        ops.push(Op::AtomAdd {
            id: None,
            el: el.into(),
            x: i as f64,
            y,
            map: None,
        });
    }
    s.run(&ops).expect("atoms");
    let mut bonds = Vec::new();
    for i in 1..50u32 {
        bonds.push(Op::BondAdd {
            id: None,
            a: i,
            b: i + 1,
            o: 1,
            s: 0,
        });
    }
    for i in 0..50u32 {
        let wedge = u8::from(i % 5 == 0);
        bonds.push(Op::BondAdd {
            id: None,
            a: i + 1,
            b: 51 + i,
            o: 1,
            s: wedge,
        });
    }
    s.run(&bonds).expect("bonds");
    let all = pieces(&s.doc);
    let mut ms: Vec<f64> = (0..20)
        .map(|_| {
            let t = std::time::Instant::now();
            let n = signature(&s.doc, &all).len();
            assert_eq!(n, 1);
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    ms.sort_by(f64::total_cmp);
    // a whole guarded step: the elements before, the mirror of the whole piece, the comparison after
    let all_ids: Vec<u32> = s.doc.atoms.iter().map(|a| a.id).collect();
    let flip = Op::Flip {
        atoms: all_ids,
        axis: Axis::H,
        at: 0.0,
    };
    let mut step: Vec<f64> = (0..20)
        .map(|_| {
            let t = std::time::Instant::now();
            let guard = before(&s.doc, std::slice::from_ref(&flip)).expect("elements");
            s.run(std::slice::from_ref(&flip)).expect("flip");
            let _ = changed(&s.doc, &guard, false);
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    step.sort_by(f64::total_cmp);
    println!(
        "one guarded mirror of the whole piece, median of 20: {:.3} ms",
        step[10]
    );
    println!(
        "atoms {}, wedges 10, median of 20: {:.3} ms",
        s.doc.atoms.len(),
        ms[10]
    );
}
