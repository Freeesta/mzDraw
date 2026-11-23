# Light and dark tokens, built (PR 3 of the UI work)

Every colour of the page comes from the `--mzd-*` tokens of `web/mzdraw.css` (prefix `NAMES.tokens`): a light and a dark set, chosen by `prefers-color-scheme` or by `data-theme="light|dark"` on the root, so a host maps its theme onto the tokens; the canvas is painted again when either changes. Warm neutral surfaces, ink that is never pure black or white, no yellow field. The e2e step "theme" measures WCAG 2.2 AA in both themes (text 4.5:1, icons, control borders and the focus ring 3:1), keeps the element colours at least ΔE 20 (CIE76) apart, and checks the focus ring. The golden SVGs are unchanged: the tokens colour the page, not the saved files. The side palettes' scroll shadows use `--mzd-shadow`, the grip dots `--grip` (mapped to `--mzd-ink-muted`). Screenshots at deviceScaleFactor 2: `docs/shots/theme-light.png`, `theme-dark.png` (desktop) and `palettes-tablet.png`.

## Where the specification lives, and what the test measures
The specification of the tokens is the CSS: the `:root` blocks at the top of `web/mzdraw.css` (a light set, a dark set by `prefers-color-scheme`, and `data-theme="light|dark"` on the root, which wins). There is no second table to keep in step; the e2e step "theme" reads the tokens from the computed style and measures them.
- Contrast (WCAG 2.2 AA): text 4.5:1 and icons, control borders and the focus ring 3:1, over every surface they meet, including the hover, the soft accent and the halo of a selection; the atom colours are read on the paper, the chrome and the selection halo; the danger colour on the selection. The focus ring must hold against the paper, the chrome, the hover and the selection.
- Distance: the six atom inks (carbon is the ink) are at least 20 apart in CIE76; no surface has a yellow cast (b* under 12).
- The canvas itself: in a drawing with a nitrogen, oxygen, sulfur, chlorine and phosphorus atom, a selection and a carbon with five bonds, the 2D context is read back and every mark is in its token (ink, halo, each atom colour, the colour of the mistake), and the paper under the canvas is read from a screenshot; in both themes, so a theme that did not repaint would fail.
- A disabled control is drawn at opacity .45. WCAG 1.4.3 and 1.4.11 exempt disabled controls from the contrast minimum, so it is not measured; its state is also said in words (the `disabled` attribute and the name of the control).
- Screenshots at deviceScaleFactor 2, with a drawing, a selection and a mistake: `docs/shots/theme-light.png`, `theme-dark.png`; the tablet in light and dark: `palettes-tablet.png`, `palettes-tablet-dark.png`.


## Default and soft change (ui-declutter-1)
The default theme is `light` (`ui.theme`), whatever the system prefers; `auto` is the way to follow the system or a host. The change of theme cross-fades (about 300 ms, instant with reduced motion, none on the first paint). See `ui-declutter-1.md`.
