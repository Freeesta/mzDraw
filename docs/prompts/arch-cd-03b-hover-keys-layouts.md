# arch-cd-03b: Hover keys on layouts whose digit row types symbols

Follow-up of `arch-cd-03` (`docs/built/hover-keys.md`). Priority SHOULD. Size guess S. Depends on: arch-cd-03 on `main`.

## Role

You are a DEV chat of mzDraw (repository `Freeesta/mzDraw`). You build ONE small slice, open ONE pull request ready for review and never merge it. Obey `CLAUDE.md`.

## Scope

In:
- the hover keys `1 2 3` and `Shift+3` to `Shift+8` also work on AZERTY-like layouts, where the digits are typed with Shift: the page sends both the typed key and the physical code (`e.code`) and the table in `hotkeys.rs` matches either; no decision in JavaScript
- the replay field `keys` and the help follow; tests in Rust for both forms, a golden case, an e2e step with a layout that types `&` for the physical `Digit1`

Out: rebinding (arch-std-06), new keys.
