# Built state: one file per slice

What exists in mzDraw is told here, one short file per slice or feature (`<slice>.md`). The index is this folder: there is no list, counter, total or size figure that every PR must edit.

Rules:
- A PR that builds a slice adds its own file (or edits the file of the slice it changes) and no other.
- Keep each file true: what is built, where it lives, and what is not built yet.
- The size of the editor module is not written by hand. `tools/build_site.sh` prints it and fails above the `cap` of `tools/wasm-cap.toml`; `size.md` keeps the history up to the last hand-written entry.
- The golden cases are one file per area in `tests/data/log/`, not here.
