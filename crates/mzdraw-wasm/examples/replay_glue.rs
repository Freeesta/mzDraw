//! Native twin of the `.wasm` export `replay_glue`, for the parity test: a replay request plus `"engines": [{"engine", "args"}]` on stdin,
//! `{"engines": [...]}` on stdout (exit 0). A refusal goes to stderr as `{"error_key", "params"}`: exit 1 for a bad request or log,
//! 2 for `error.draw2.engineMissing`.

use std::io::Read;

fn main() {
    let mut request = String::new();
    if std::io::stdin().read_to_string(&mut request).is_err() {
        std::process::exit(2);
    }
    match mzdraw_wasm::glue::replay(&request) {
        Ok(out) => print!("{out}"),
        Err(e) => {
            eprint!("{}", e.to_json());
            std::process::exit(if e.key == "error.draw2.engineMissing" {
                2
            } else {
                1
            });
        }
    }
}
