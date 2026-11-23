# History panel

Every step of the session is listed, newest first, and a click (or Enter) leaves the drawing as it was at that step. A jump is a run of undo or redo: there is no new operation, so the property test "apply then undo returns the identical document" already covers it, and the log keeps only the steps still applied.

## What it does
- `Editor::history_json()` / `out: "history"` / the page's `history()`: `history/1`, `{"format":"history/1","at":N,"start","steps":[{"say","what","index","state"}]}`, newest first. `say` and `what` are the i18n key and parameter of the step (the words the live line said when the step was made: `draw2.say.atom`, `P`); a step that nothing named, or that came with others in one action, is `draw2.history.step`; in a replay a step made by the `log` is `draw2.history.op` with the operation name. `index` is the number of the step (1 is the oldest). `state` is `current` (the newest applied step: where the drawing is), `done` or `undone` (a redo brings it back). `at` is the index of the current step, 0 at the start (then no step is `current`); `start` is `current` at the start, else `done`: the state of the page's "Start" row, said by Rust.
- Commands, the same in the page (`command("jump:3")`) and in a request (`commands: [{"command": "jump:3"}]`): `undo:<n>` (n steps back, at most as many as there are, n from 1), `redo:<n>`, `jump:<n>` (to step number n, 0 is the start). A step count that is not a plain unsigned integer is `error.draw2.command`; `undo:0`, `redo:0`, `args`, or a `jump` past the newest step is `error.draw2.request` and changes nothing. A jump that changes nothing is not an error and gives no `CHANGED` flag to the page. A redo whose operations no longer apply (only possible if the document was changed behind the history) is refused: the step stays waiting and the jump says it did not arrive.
- Steps joined by the editor into one undo step (repeated key nudges) are one row. A new step after an undo forgets the undone steps, as before: no branching history.
- The page: a collapsible "History" section of the Check palette. Rows are a listbox; arrows, Home and End walk the rows, Enter or a click jumps, the current row is marked (and `aria-selected`), the undone rows are dimmed. A "Start" row at the bottom jumps to 0. The page only paints `history()` and sends `jump:<index>`. The list is rebuilt only when the text of `history()` changed since the last paint, so a refresh between the press and the release of a click keeps the row under the pointer. Cost: `history()` and the paint are O(steps) per change of the drawing while the section is open, and there is no cap on the number of steps (a session of many thousands of steps lists them all).

## How steps get their words
The session stores a label per step (`Session::name_new_steps`), set from the live line by `Editor::name_steps`, which the bridge calls after every action and the replay after every operation and command; `undo`/`redo`/`jump` call it first. Steps made by a native caller that never calls it are named when the next one is, with `draw2.history.step` if more than one is waiting.

## Tests
`crates/mzdraw/tests/history.rs` (list order, labels, jump 5 back and 5 forward, a jump equals replaying the first n steps of the log, a new step drops the redo part, `undo:<n>` / `redo:<n>` bounds, joined nudges, random jumps against the document of each step); golden `tests/data/log/history.json` (section `histories`, parity wasm == native; refusals in `refused`); the schema golden has the new rows; `tests/e2e.mjs` (mouse, keyboard, touch, pen).

## Not built
- Branching history (a new step after an undo forgets the redo stack).
- A saved history across sessions (the history is the session's; the document autosave does not carry it).
- Labels in the history with the parameters of the live line that has several numbers (`draw2.say.pathway`, `labelled`): only `what` is kept.
