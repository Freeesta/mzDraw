//! Identity of structures for the velocity benchmark (tests/velocity.mjs): reads one SMILES per line on stdin and writes, per line,
//! the `mzlab-mol` identity string (`Mol::can`, canonical atom order with stereo) or `ERR` when the line is not a SMILES `mzlab-mol` reads.
//! Two drawings of one molecule give the same line whatever their atom order or Kekulé form is.

use std::io::Read;

fn main() {
    let mut text = String::new();
    if std::io::stdin().read_to_string(&mut text).is_err() {
        std::process::exit(2);
    }
    for line in text.lines() {
        match mzlab_mol::Mol::from_smiles(line.trim()) {
            Ok(m) => println!("{}", m.can()),
            Err(_) => println!("ERR"),
        }
    }
}
