//! The calculated values of a drawing with a dative or a hydrogen bond, with the real engine (arch-web-20). The engine reads the record, in
//! which both are single bonds, and counts the hydrogens of that reading (water-ammonia: H3NO, 33.0215) where the drawing says H5NO: a
//! number that disagrees with the formula is never written, so the structure has its formula and the key `error.draw2.valuesLink`, and the
//! other structures of the drawing are measured as before. Synthetic structures only.

use mzdraw_wasm::glue;
use serde_json::{json, Value};

fn values(request: Value) -> Value {
    serde_json::from_str(&glue::replay(&request.to_string()).unwrap()).unwrap()
}

#[test]
fn a_linked_structure_has_its_formula_and_no_mass_and_its_neighbours_are_measured() {
    // atoms 1 O, 2 N (water and ammonia joined by a hydrogen bond, bond id 4), 3 C and 4 O (methanol, plain)
    let v = values(json!({"import": "O.N.CO", "values": true,
        "log": [{"op":"bond.add","a":1,"b":2,"o":1},{"op":"bond.set","id":6,"k":"hbond"}]}));
    let s = &v["structures"];
    assert_eq!(s[0]["formula"], "H5NO");
    assert_eq!(s[0]["error"], "error.draw2.valuesLink");
    assert_eq!(
        (s[0]["mass"].clone(), s[0]["average"].clone()),
        (Value::Null, Value::Null)
    );
    assert_eq!(s[0]["mz"], json!({}));
    // methanol: 12 + 4 * 1.00782503207 + 15.99491461956 = 32.026215 -> 32.0262
    assert_eq!(
        (s[1]["formula"].clone(), s[1]["mass"].clone()),
        (json!("CH4O"), json!("32.0262"))
    );
    assert_eq!(s[1]["error"], Value::Null);
    // the same two molecules without the bond are two structures, each measured: water 18.0106, ammonia 17.0265
    let v = values(json!({"import": "O.N", "values": true}));
    let s = &v["structures"];
    // H2O: 2 * 1.00782503207 + 15.99491461956 = 18.010565 -> 18.0106; NH3: 3 * 1.00782503207 + 14.0030740048 = 17.026549 -> 17.0265
    assert_eq!(
        (s[0]["formula"].clone(), s[0]["mass"].clone()),
        (json!("H2O"), json!("18.0106"))
    );
    assert_eq!(
        (s[1]["formula"].clone(), s[1]["mass"].clone()),
        (json!("H3N"), json!("17.0265"))
    );
}

#[test]
fn a_dative_bond_to_boron_has_its_formula_and_the_key() {
    // NH3->BH3: boron is also outside the engine's elements, so a second reason; the key of the link wins (it is the first reason)
    let v = values(json!({"import": "N.B", "values": true,
        "log": [{"op":"bond.add","a":1,"b":2,"o":1},{"op":"bond.set","id":3,"k":"dative"}]}));
    let s = &v["structures"][0];
    assert_eq!(s["formula"], "H6BN");
    assert_eq!(s["error"], "error.draw2.valuesLink");
    assert_eq!(s["mass"], Value::Null);
}
