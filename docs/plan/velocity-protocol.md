# Velocity protocol: timing the fixed tasks in another editor

`velocity/1` compares how fast a structure is drawn. The harness (`tests/velocity.mjs`) times mzDraw in a browser and replays its `draw_edit/1` patches natively; `tools/velocity/ketcher.mjs` does the same for a pinned standalone Ketcher. This page is the protocol for **a person** who times the same 12 tasks in a desktop editor they are licensed to use. No product is named as a target of fidelity: the numbers say how long a path takes, not how alike two programs are.

## What is fixed
- The tasks: `tests/data/velocity/tasks.json` (12 synthetic structures: id, goal, target SMILES, objects). A task is done when the drawing is the target: the same molecule, stereo included, written in any atom order or Kekulé form; for the two reaction/row tasks also the same arrows, plus signs and texts.
- Canvas: a window of about 1280 x 800 px, an empty drawing, default settings, default bond length, no template library beyond the ring tools the editor offers on its first screen (say which ones you used).
- The input device: say it (mouse, trackpad, touch, pen, keyboard only). Do one device per run.

## Counting actions
One action is one press-and-release (a click or a tap), one drag, one key press (a chord such as `Shift+Enter` is one), one text entry committed with Enter, or one press of a toolbar or menu button. Moving the pointer without pressing is not an action. Opening a menu and choosing an item are two actions. Pasting a structure text counts as two actions (paste, confirm) and is a different test from drawing: it measures typing, not drawing, so use the drawing path and report the paste route on a separate row if you want it.
Write down every action in order (`tool ring:6a`, `click x y`, ...); the list is the path. The shortest path you know counts, not the first one you found: try each task three times, keep the best.

## Timing
1. Screen-record the window at 30 frames per second or better (the editor's own timer is not used).
2. The clock starts at the first input event of the path and stops at the first frame in which the drawing is the target. Read both frames off the recording; the difference is `ms`.
3. Repeat five times with an empty canvas each time; the row carries the median.
4. Do not count the time to read the task or to find a tool the first time. Practise each path twice before the five timed runs, because the page timings are of a path that is already known.

## The row
One JSON object per task in the same shape as the harness, with `"measured_by": "person"`:

```json
{"format": "velocity/1", "tool": "<name and version of the editor>", "measured_by": "person", "device": "mouse",
 "date": "2026-11-21", "rows": [{"task": "benzene", "actions": 2, "ms_median": 1900, "ok": true, "path": ["tool ring", "click 0 0"]}]}
```

`ok` is false when the target was not reached; then `ms_median` is `null`. `path` is the list of actions. Keep the recordings; they are the evidence, and they stay with the person (they are not committed). Name the licence you used the editor under; if you do not hold one, do not run the test.

## Reading the results
Compare `actions` first (it does not depend on the machine or on the person's speed), then `ms_median`. A larger count in mzDraw on a task is a finding for the plan (the harness lists the three largest gaps); a larger time with the same count is the person's practice. Never average the tasks into one score: they test different things (templates, element changes, labels, stereo, objects).

## Review of an agent's proposal (plan-16)
A person's experiment, not a measurement the build makes: is reviewing a patch hunk by hunk faster, or more accurate, than looking at the result and undoing what is wrong? Nothing here is claimed until a person has run it.

1. Prepare five drawings (synthetic structures only) and, for each, a patch of `draw_edit/1` with four to six hunks, two of which are wrong on purpose (a wrong element, a bond moved the wrong way). Write the patches as files; they are the same for both ways.
2. Way A (proposal): load the patch with "Show as a proposal", walk the hunks with `]` and `[`, accept with Enter and reject with Delete. Way B (apply and undo): apply the whole patch as a log (`Open a file` or the CLI), look at the result and undo each wrong change by hand. Alternate which way comes first from drawing to drawing.
3. Record per run: the time from the first keystroke to the drawing the person calls right, the number of actions (keys and clicks), and whether the final drawing equals the intended one byte for byte (`save('mzdraw')` against the reference). Use the row of the section above with `"task": "review-A"` and `"review-B"`.
4. Compare `ok` first (a wrong drawing left in place is worse than a slow one), then `actions`, then `ms_median`. Five timed runs per way and drawing, after two practice runs.
