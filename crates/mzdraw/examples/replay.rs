//! Native twin of the `.wasm` export `replay`, for the parity test: reads a request on stdin, writes the output on stdout; a refusal
//! goes to stderr as `{"error_key", "params": {"id"}}` with exit code 1. A `draw-result/1` document with `ok` false is a verdict: it
//! is written on stdout and the exit code is 1 too. `MZDRAW_REQUEST_N` is the index of the request in its batch (default 0).

use std::io::Read;

fn main() {
    let mut request = String::new();
    if std::io::stdin().read_to_string(&mut request).is_err() {
        std::process::exit(2);
    }
    let n = std::env::var("MZDRAW_REQUEST_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    match mzdraw::replay::replay_at(&request, n) {
        Ok(r) => {
            print!("{}", r.text);
            if !r.ok {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprint!(
                "{{\"error_key\":\"{}\",\"params\":{{\"id\":{}}}}}",
                e.key, e.id
            );
            std::process::exit(1);
        }
    }
}
