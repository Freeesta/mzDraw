# Touch: the "lost first tap" after a pen stroke (finding and guard)

Why: the e2e of arch-web-08 (PR #109) saw, on the shared 768x1024 touch page, a sequence Clear, click the chain tool, `touchscreen.tap` on the canvas that drew nothing (no said line, no error), while the same tap a moment later drew. A fresh page never did it.

## Finding: a harness effect, not a page or editor bug

No pointer id is left held, no gesture state or timer is stale, and the canvas capture is released. The cause is the palm rule of `editor_touch.rs` that was already documented in `touch-pen.md`: a finger that lands within 500 ms of the last pen event (the pen's lift counts) is a palm and is ignored until it lifts. On the shared page the pen step ran just before, and Playwright reaches Clear, a tool click and a tap within a few hundred milliseconds, which a person cannot. A CDP finger drag first lost its gesture for the same reason; the one after drew.

Evidence (a scratch script, not committed): on that page, a pen tap, then Clear, then the chain tool, then `touchscreen.tap` after a wait of 0, 100 and 300 ms drew nothing and after 700 ms drew. The page received `pointerdown` and `pointerup` of the touch every time (ids 11 to 14), so nothing was lost in the browser or in `web/mzdraw.js`; the tap that "drew nothing" reached Rust and was dropped as a palm. Without a pen event before it, six rounds of Clear, chain, tap on a page that had run the whole tablet loop drew every time.

No production code is changed: the rule is the wanted behaviour (the resting hand that lands as the pen lifts), and a person cannot tap a tool button and the canvas within 500 ms of a pen lift.

## What is built (tests and notes only)
- `crates/mzdraw/tests/touch.rs`: `a_tap_inside_the_palm_window_after_the_pen_is_lost_whole_and_the_first_one_after_it_draws`: after a pen stroke, finger taps with one id inside the window draw nothing and leave nothing held; the first tap after the window draws, and so does the next.
- `tests/e2e.mjs`, one step on the shared tablet page straight after the pen step: three rounds of a pen tap (chain tool), Clear, the chain tool, a wait on the page's own clock (`performance.now()`, no fixed sleep) until the palm window is over, then a finger tap that must draw, and the keyboard undo. Finger and pen alternate.

## Not built
- A change of the palm window or of its start (the pen's lift instead of its last event): not wanted without a measurement on a real tablet.
- The CLI twin: no new operation, no change of a function of the page.
