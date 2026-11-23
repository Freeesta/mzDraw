# ui-declutter-1: a lighter page: white by default, no status text, no dotted grips, panels that never overlap

Study rows: the owner's try of the live page (dark theme, desktop 2000x1130 and a phone-width viewport). Priority MUST. Size guess M. Depends on: nothing. The palettes and their icons are the work of ui-declutter-2: keep out of the contents of the palettes.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`). You build ONE slice, UI only, and open ONE pull request ready for review; you never merge it. Follow `CLAUDE.md` and `docs/mzdraw-ui.md`. Logic stays in Rust; JavaScript paints and forwards events.

## The six changes

1. A first visit (nothing stored, even when the system prefers dark) opens the light theme. The dark theme stays a setting (`ui.theme`), a stored choice wins, and the theme is on the page before the first paint.
2. The text "0 atoms, 0 bonds" is no longer painted on the canvas. It stays for assistive technology (a visually hidden live region; "What is drawn" of the Check panel).
3. "About" and "Report a bug / suggest an improvement" leave the bottom bar for the Preferences panel (no new always-visible button; keyboard and touch reachable, 44 px targets).
4. The side panels never overlap at 390x844, 768x1024, 1280x800 and 2000x1130, open or collapsed. The layout is computed in Rust, with a unit test, and an e2e step measures the boxes pairwise.
5. The dotted grip row at the top of every panel and palette is gone. The panels stay movable: the title row, the empty margin of a palette and the keyboard (Alt+arrows) move them; touch and pen work; the names are kept.
6. The change of theme is a soft cross-fade of the colours (page, panels, palettes, flyouts and canvas), about 300 ms, instant with `prefers-reduced-motion`, none on the first paint.

## Acceptance

`cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, `tools/build_site.sh`, `cargo build --locked -p mzdraw --example replay && node tests/parity.mjs`, `node tests/e2e.mjs`, `node tests/velocity.mjs --check`, `tools/queue_check.sh`. A note `docs/built/ui-declutter-1.md`, screenshots at deviceScaleFactor 2 in `docs/shots`. No size figure anywhere (rule 8).
