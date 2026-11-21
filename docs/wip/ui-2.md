# UI-2 state (remove in the last UI PR)

## Done
- PR #32 (docs) merged.
- PR #35 (R1, Rust palettes): review fixes pushed, head 9e7896055f241be4f6889c3636ee164bb18c275d (main merged in; parity 472/472, e2e 18/18).
- This branch (NOT a PR; built on main e16bdb1.. + #35 at 66e8f83, before the fixes):
  - 46e468f PR 2: palettes page (`web/palettes.js`, `web/names.js`, `web/icons.js`, index.html, css, lang, e2e: 2 new steps). e2e 18/18 here.
  - 613e61c PR 3: `--mzd-*` light/dark tokens + e2e step "theme" (WCAG AA, ΔE ≥ 20, no yellow field). e2e 19/19 here.

## Left
1. When #35 is merged: `git checkout -b claude/mzdraw-ui-palettes origin/main && git cherry-pick 46e468f`.
   Expect conflicts in docs/mzdraw.md (Built state) and possibly web/ (CTX-MENU #37 page work, `#d2-menu` container is kept in index.html).
   Then: `tools/build_site.sh && cargo build --locked -p mzdraw --example replay && node tests/parity.mjs && MZDRAW_SHOTS=shots node tests/e2e.mjs`.
   Open PR 2 with screenshots palettes-desktop / palettes-moved / palettes-moved-dark / palettes-tablet* (DSF 2).
2. After PR 2 merges: branch from main, cherry-pick 613e61c (PR 3), same checks.
3. Then ONE private Artifact: before = main at e16bdb1, after = site of the branch; light/dark, 1280x720 and 768x1024. Link to COO-MZDRAW.

## Decisions taken (also for the PR bodies)
- Palette contents are page composition (SPEC in palettes.js); where/how a palette is drawn is Rust's answer only.
- Collapsed palette = head (kind last used) + flyout; expanded = all items inline, head only for typed fields.
- Side palettes are `<details>`: open/closed = the module's `collapsed`, synced both ways.
- Handles 24 px (mouse, WCAG 2.5.8) and 44 px on coarse pointers; double click collapses.
- Dock hint = the edge a held floating palette is flush with (it docks there on release).
- Side palettes capped at (100vh - 260px)/3 so the right column does not wrap into the canvas.
- Tooltips: only keys the module binds (e2e BUILT set); zoom names its gesture.
- e2e: after a synthetic CDP touch drag, the next tap's click is dropped by the test protocol (also after a canvas drag); the tablet reset is clicked.
