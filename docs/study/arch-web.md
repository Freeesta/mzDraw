# mzDraw: web and embeddable editors, the complete feature list, and an embedding API (ARCH study `arch-web`)

Study only, no code. Question of the owner: mzDraw becomes a standalone molecule editor (browser, embedded in the mzLab page, desktop app for macOS and Windows) that is better than Ketcher; which features must it have, and how is it embedded? This file answers with (1) what the web and embeddable editors do and how a host page talks to them, (2) one feature inventory with a priority and a scriptable twin per feature, (3) a proposal for `<mzdraw-editor>` and its API, with the section "two hosts, one component" asked by COO-MZDRAW, (4) the DEV slices, one prompt each in `docs/prompts/arch-web-NN-slug.md`.

Rules of this file: public documentation only, summarised in our own words; no text, artwork or icon of any product is copied; every fact carries a source key of §0 (URL + version or date); `*` = not read at a primary source (a page behind login, paywall or bot wall is "not read", never guessed). Nothing here claims fidelity to any product. Read on 2026-11-20. What is built is `docs/mzdraw.md` "Built state"; the toolbar, icons and keys of the page are the UI study (`docs/mzdraw-ui.md`, PR #32) and are not repeated here.

## 0. Sources (key, what, version or date)
| key | source | version / date |
|---|---|---|
| KH | Ketcher user help and hotkeys, https://github.com/epam/ketcher/blob/v3.21.0/documentation/help.md | Ketcher 3.21.0 (npm 2026-11-18) |
| KR | Ketcher README (embedding, API, events), https://github.com/epam/ketcher/blob/v3.21.0/README.md and `packages/ketcher-react/README.md` at the same tag | 3.21.0 |
| KA | Ketcher public API surface (types only, read as documentation), https://github.com/epam/ketcher/blob/v3.21.0/packages/ketcher-core/src/application/ketcher.ts, `ketcher.types.ts`; toolbar names `packages/ketcher-react/src/script/builders/ketcher/ButtonName.ts`; settings `packages/ketcher-react/src/script/ui/data/schema/options-schema.ts` | 3.21.0 |
| KS | KET schema, https://github.com/epam/ketcher/blob/v3.21.0/packages/ketcher-core/src/domain/serializers/ket/schema.json | 3.21.0 |
| KN | npm registry metadata of `ketcher-core`, `ketcher-react`, `ketcher-standalone`, `ketcher-macromolecules`, https://registry.npmjs.org/ketcher-react | 3.21.0, 2026-11-18 |
| KI | issue numbers cited by the Ketcher regression tests (`ketcher-autotests/tests/specs/**`), anonymous clone of https://github.com/epam/ketcher at commit 7f69095; links `https://github.com/epam/ketcher/issues/N`. The tracker itself was **not read** (GitHub access to that repository is not enabled for this session): dates, states and reactions are `*` | 2026-11-20 |
| MJE | Marvin JS embedding, https://docs.chemaxon.com/latest/marvin-js_how-to-embed.html | Marvin JS 25.3.0 (2025-09-12) |
| MJH | Marvin JS history of changes, https://docs.chemaxon.com/latest/marvin-js_history-of-changes.html | 25.3.0 |
| MJK | Marvin JS keyboard shortcuts, https://docs.chemaxon.com/latest/marvin-js_keyboard-shortcuts.html | 25.3.0 |
| MJW | Marvin JS web services, https://docs.chemaxon.com/latest/marvin-js_web-services.html | 25.3.0 |
| MJL | Marvin JS end of life: Design Hub notice https://docs.chemaxon.com/latest/design-hub_marvin-js-end-of-life.html and Certara notice https://www.certara.com/announcement/marvin-js-end-of-life-and-transition-notice/ | notice published 2025-09-16, modified 2026-03-31 |
| MNM | new Marvin migration guide (PDF), https://www.certara.com/wp-content/uploads/2026/03/marvin_25.3.3.pdf | Marvin 25.3.3 (2025-09-01) |
| MNC | new Marvin vs Marvin JS, https://docs.chemaxon.com/latest/marvin_sketch-js-comparison.html | read 2026-11-20 |
| MNS | new Marvin shortcuts and keyboard navigation, https://docs.chemaxon.com/latest/marvin_shortcuts-and-tricks.html | read 2026-11-20 |
| MNI | new Marvin installation (React front end, Spring Boot back end), https://docs.chemaxon.com/latest/marvin_installation-guide.html | read 2026-11-20 |
| MNX | Chemaxon examples (`createMarvin`, `importDocument`, `exportDocument`), https://github.com/ChemAxon/marvin-examples | commit a37f5ed, 2026-06-17 (`@chemaxon/marvin` 25.3.101) |
| JH | JSME help, https://jsme-editor.github.io/help.html | JSME 2024-04-29 |
| JD | JSME options and embedding, https://jsme-editor.github.io/dist/doc.html | 2024-04-29 |
| JA | JSME API javadoc, https://jsme-editor.github.io/dist/api_javadoc/ | 2024-04-29 |
| OT | OpenChemLib JS type declarations (`openchemlib.d.ts`, `CanvasEditor`, `<openchemlib-editor>`), npm tarball, https://registry.npmjs.org/openchemlib | 9.26.0, 2026-11-11 |
| OC | OpenChemLib JS changelog, https://raw.githubusercontent.com/cheminfo/openchemlib-js/main/CHANGELOG.md | read 2026-11-20 |
| KK | Kekule.js README and widget sources (MIT, read as documentation), npm tarball, https://registry.npmjs.org/kekule | 1.0.4, 2026-05-22 |
| WM | WebMolKit README and `Sketcher.d.ts`, https://registry.npmjs.org/webmolkit | 2.0.13, 2026-11-03 |
| BU | Butlerov README, https://registry.npmjs.org/butlerov | 0.1.0, 2025-02-09 |
| CD | ChemDoodle Web sketcher tutorial (documentation only; GPL code not read), https://web.chemdoodle.com/tutorial/2d-structure-canvases/sketcher-canvas | ChemDoodle Web 11.0.0 (2025-05-17) |
| SD | SmilesDrawer README, https://registry.npmjs.org/smiles-drawer | 2.4.2, 2026-11-17 |
| PC | PubChem Sketcher page and help, https://pubchem.ncbi.nlm.nih.gov/edit3/index.html, https://pubchem.ncbi.nlm.nih.gov/docs/sketcher-help | V2.4 (page 2018-09-07; help 2022-09-14) |
| PR | PubChem PUG-REST, https://pubchem.ncbi.nlm.nih.gov/pcfe/docs/markdown/pug-rest.md | read 2026-11-20 |
| MV | MolView README, docs and history (no code read: rights withdrawn), https://github.com/molview/molview | 2.4.6, commit c92563a (2026-07-06) |
| CJ | ChemDraw JS public help centre articles (support.revvitysignals.com, articles 10746800139028, 38809304211220, 55248834779668, 37285043394964) and https://github.com/Revvity/ChemDraw-JS-Public-Examples | v25.0.1 (2025-06-27); 26.4.0 named 2026-11-03; examples c96cfbb |
| BD | BIOVIA Draw product page and datasheet, https://www.3ds.com/products/biovia/draw, https://www.3ds.com/assets/invest/2024-02/biovia-draw-ds.pdf | 2024 datasheet; developer docs on help.3ds.com not read (403) |
| WCAG | WCAG 2.2, https://www.w3.org/TR/2024/REC-WCAG22-20241212/ and its Understanding pages | Recommendation 2024-12-12 |
| ARIA | WAI-ARIA 1.2, https://www.w3.org/TR/2023/REC-wai-aria-1.2-20230606/; 1.3 draft https://www.w3.org/TR/2026/WD-wai-aria-1.3-20260604/ | REC 2023-06-06; WD 2026-06-04 |
| APG | ARIA Authoring Practices: toolbar, keyboard interface, https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/, https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/ | undated pages, read 2026-11-20 |
| PE | Pointer Events Level 3, https://www.w3.org/TR/2026/REC-pointerevents3-20260630/ | REC 2026-06-30 |
| CB | Clipboard API and events, https://www.w3.org/TR/2026/WD-clipboard-apis-20260624/ | WD 2026-06-24 |
| TAG | Web Platform Design Principles, https://www.w3.org/TR/2026/NOTE-design-principles-20261102/ | Group Note 2026-11-02 |
| MQ | Media Queries 5, https://www.w3.org/TR/2026/WD-mediaqueries-5-20260219/; CSS Color Adjustment 1, https://www.w3.org/TR/css-color-adjust-1/ | WD 2026-02-19; CRS 2025-12-16 |
| BCD | MDN browser-compat-data (support numbers), via raw.githubusercontent.com/mdn/browser-compat-data | 8.1.10, read 2026-11-20 |
| HTML | WHATWG HTML (custom elements, form-associated elements and `ElementInternals`, `postMessage`, iframe `sandbox` and `allow`), https://html.spec.whatwg.org/multipage/custom-elements.html, `/web-messaging.html`, `/iframe-embed-object.html`; DOM (`delegatesFocus`), https://dom.spec.whatwg.org/ | Living Standards, last updated 2026-11-20 (HTML) and 2026-11-15 (DOM) |
| KT | Kekule.js tutorial and API docs, https://partridgejiang.github.io/Kekule.js/ | tutorial 2023.01; API docs generated 2018-11-26 |
| OD | OpenChemLib JS docs, https://cheminfo.github.io/openchemlib-js/ | read 2026-11-20 (version match with 9.26.0 not stated*) |

Not read (blocked, login, bot wall or gone): the Ketcher issue tracker and release notes (GitHub access to that repository is not enabled for this session); epam.github.io/ketcher (404, no site); the Marvin JS API reference (its only host, marvinjs-demo.chemaxon.com, now answers 404 and redirects to the end-of-life notice), so most Marvin JS method and event names stay `*`; the new Marvin API reference and examples (licence or private registry only); molview.org; revvity.com and revvitysignals.com product pages (403) and the ChemDraw JS developer guide (licence only); help.3ds.com (403); pubs.acs.org (challenge), so no paper abstract was read; archive.org (refused by the session's permission check). Hosts first refused and then opened by the owner during the study (html.spec.whatwg.org, developer.mozilla.org, w3c.github.io, cheminfo.github.io, partridgejiang.github.io, www.certara.com) were read in a second pass.

## 1. How each editor is embedded and talks to its host
| editor | delivery | start | host → editor | editor → host | instances | server needed for |
|---|---|---|---|---|---|---|
| Ketcher 3.21.0 | React component `<Editor>` (React ^18.2 or ^19) with `staticResourcesUrl`, `structServiceProvider`, `onInit`, `errorHandler`, `buttons` to hide, `customButtons`, `disableMacromoleculesEditor` [KR, KA]; or the built app in an iframe, the host waits for a posted `init` message, then reads `contentWindow.ketcher` [KR] | async; the object does not exist right after the iframe is inserted [KR] | Promise methods on `ketcher`: `getSmiles`, `getMolfile`, `getKet`, `getRxn`, `getCml`, `getSdf`, `getCDXml`, `getCDX`, `getInchi`, `getSmarts`, `setMolecule` (replace, format detected, optional position and zoom), `addFragment`, `layout`, `calculate`, `generateImage`, `setZoom`, `setSettings`, mode switch [KR, KA] | `ketcher.editor.subscribe('change', h)`: one record per edit with an `operation` name and its ids and positions [KR]; `eventBus` for custom buttons; no documented selection, tool or mode events [KA] | one per React tree; `ketcherId` prop exists in code only [KA] | standalone: none (Indigo 1.49.0 wasm in the browser, package 111 MB unpacked) [KN]; remote: an Indigo REST service; OCR always remote [KR] |
| Marvin JS 25.3.0 | iframe on `editor.html` (no services) or `editorws.html` (services preset), at least 500×450 px [MJE] | async: `MarvinJSUtil.getEditor('#id')` returns a Promise; or `contentWindow.marvin.onReady` then `sketcherInstance` (same origin); a cross-origin postMessage example since 22.19 [MJE, MJH] | `importStructure(format, text)` and `exportStructure(format, opts)` return Promises; `setDisplaySettings` (toolbar presets), `setServices` [MJE, MJW]; `getSmiles`, `clear`, `addButton`, `setLanguage` are in the JSDoc only* | `on('undo')`, `on('redo')` confirmed [MJE]; `molchange`, `selectionchange` in the JSDoc only* | one per iframe | every format except MRV and MOL V2000, clean 2D/3D, CIP, explicit H, automap, aromatise [MJW] |
| new Marvin 25.3.x | React package from Chemaxon's own registry (not npm), Spring Boot back end that also grants the licence [MNI, MNX] | async: `createMarvin(el, settings)` returns `Promise<Marvin>`, or `<ReactMarvin onReady>` [MNX] | `importDocument(text, {clearCanvas, select, scale, placement})`, `exportDocument(format)`, `exportImage` (SVG, PNG, JPG, WEBP, TIFF) [MNX] | not documented publicly* | per element [MNX] | the back end (licence, conversions) [MNI] |
| JSME 2024-04-29 | one script (`jsme.nocache.js`, 10.6 KB loader, about 156 KB gzip per GWT bundle, InChI 323 KB gzip on demand) [JD] | async: the page defines `jsmeOnLoad()`, then `new JSApplet.JSME(id, w, h, {options})` [JD] | about 109 methods: `smiles`, `molFile(v3000)`, `readGenericMolecularInput`, `options(...)`, `setAtomBackgroundColors`, `getMolecularAreaGraphicsString` (SVG) [JA] | `setCallBack(name, fn)`: `AfterStructureModified`, `BeforePaste`, `AfterPaste`, `AtomClicked`, `BondClicked`, `AtomHighlight`, `BondHighlight`, with an event carrying the source, action, atom, bond and molecule [JA] | many (`document.jsapplets`) [JD] | none [JD] |
| OpenChemLib JS 9.26.0 | ES module; `new CanvasEditor(el, {readOnly, initialMode, initialFragment})`, or `<openchemlib-editor>` after `registerCustomElement()` (attributes `idcode`, `fragment`, `mode`, `readonly`) [OT]; `StructureEditor` was removed in 9.0.0 (2025-04-17) [OC] | sync; the custom element applies its initial `idcode` in `requestIdleCallback` [OT] | `setMolecule`, `getMolecule` (the editor edits the passed molecule in place), `setReaction`, `clearAll`, `destroy` [OT] | one listener `setOnChangeListener` with `{type: molecule\|selection\|highlight-atom\|highlight-bond, isUserEvent}`, no detail of what changed; element `change` event; its `idcode-changed` and `fragment-changed` look unreachable in 9.26.0* [OT] | many [OT] | none |
| Kekule.js 1.0.4 | script or ES module plus a theme CSS (presets `fullFunc`, `molOnly`, `compact` [KT]); declarative `data-widget="Kekule.Editor.Composer"` with `data-chem-obj`, or `new Kekule.Editor.Composer(el)`; custom elements `kekule-composer`, `kekule-viewer` [KK] | sync at DOM ready [KK] | `setChemObj`, `getChemObj`, `load`, `newDoc`, `exportObjs`, `undo`, `redo` [KK] | named widget events with `on(name, cb)`: `editObjsUpdated` with change details, `selectionChange`, `hoverOnObjs`, `userModificationDone`, operation-history events `operPush`, `operUndo`, `operRedo` [KK, KT]; copy and paste go through `localStorage`, not the system clipboard [KT]; widget events are not re-sent as DOM events* | many | none (SMILES reading and layout need an Indigo or Open Babel wasm add-on, `mzdraw-competitors.md`) |
| ChemDoodle Web 11.0.0 | `new ChemDoodle.SketcherCanvas(id, w, h, options)` on a canvas (`oneMolecule`, `includeQuery`, `isMobile`, `useServices`, `resizable`) [CD] | sync [CD] | `getMolecule`, `loadMolecule`, styles object [CD] | no event: the host overrides a hook method and calls the original [CD] | many | SMILES, names, CDX/CDXML, InChI through paid cloud services [CD] |
| WebMolKit 2.0.13 | `new WebMolKit.Sketcher()`, host supplies clipboard and menu proxies [WM] | async `setup` / `setupAsync`, then `render(el)` [WM] | `defineMolecule`, `getMolecule`, `undo`, `redo` [WM] | one callback `callbackChangeMolecule(mol)` [WM] | many | none |
| Butlerov 0.1.0 | `new MoleculeEditor({stage, mode, theme, style})` on Konva [BU] | sync | `load`, `save` (MOL, SMILES, own) [BU] | `onchange` without payload [BU] | many | none |
| PubChem Sketcher V2.4 | iframe of `edit3/index.html?smiles=…` (PubChem's own modal, 905×450) [PC] | server session | query parameters passed to the server [PC] | the frame calls `transferSmiles`, `transferInChI`, `transferFormula` on its parent or opener (same origin only) [PC] | one per frame | every gesture: each mouse event is a request and the canvas comes back as an image [PC] |
| MolView 2.4.6 | web app; a "Link" tool gives an URL or HTML snippet [MV]; parameter names not in the docs read* | n/a | URL parameters* | none documented [MV] | n/a | clean-up, 3D, lookups (PubChem, RCSB, COD, NCI resolver) [MV] |
| ChemDraw JS 25/26 | licensed package loaded by a script tag, not bundled; `await RevvitySignals.ChemDrawJS.attach({license, chemistryServiceURL, editor: {element}})` (v25), `perkinelmer.ChemdrawWebManager.attach({...})` before [CJ] | async | `getStructureIDs`, `getStructureProperties`; config of tools and features [CJ]; the API reference is licence-only (not read) | `drawing.onContentChange` [CJ] | per element | conversion, SVG, properties through the ChemDraw Web Service (Docker); an "extended clipboard" needs a Windows helper and a browser extension [CJ] |
| BIOVIA Draw 2024 | Windows desktop; "Draw Bridge" for browsers appears to drive the installed program* [BD] | n/a | not read | not read | n/a | n/a |

What the table teaches:
- **Every rich editor starts asynchronously and makes the host wait** (Ketcher, Marvin JS, new Marvin, JSME, WebMolKit). A host that calls too early gets nothing. mzDraw should expose one `ready` promise and also accept calls made before it (queued), so a host never races.
- **Only Ketcher and Kekule say what changed**; the others say only that something changed. mzDraw's command log is exactly what changed, in a public language (`draw_edit/1`): the change event should carry it.
- **Iframes are the price of foreign code** (Marvin JS, Ketcher's app, PubChem). Same-origin script access across frames (PubChem, Marvin's `contentWindow.marvin`) breaks cross-origin hosts. mzDraw needs no iframe: one custom element, with an optional iframe page and a versioned message protocol for cross-origin hosts.
- **Servers creep in** (Marvin JS formats, ChemDraw JS conversion, PubChem every gesture, ChemDoodle SMILES, Ketcher OCR). mzDraw has none and should keep "no network" as a tested property of the component.
- **Nobody ships real accessibility**: no ARIA in Ketcher's docs, JSME, OCL, Kekule, PubChem; ChemDoodle shows an add-on demo; only the new Marvin documents keyboard navigation of the canvas and claims screen-reader support for its menus and toolbars [MNS, MNM]. mzDraw is already ahead here (focus walk, outline, live line) and should finish the job.

## 2. Touch, pen, keyboard and accessibility per editor
| editor | touch / pen | keyboard | accessibility |
|---|---|---|---|
| Ketcher 3.21.0 | nothing documented [KH]; the molecule canvas binds mouse events, no pointer-type handling seen* [KA] | rich single-key map: atoms by letter, bonds `0`-`4`, `t` cycles ring templates, `Shift+T` library, `Shift+F` functional groups, `Mod+L` layout, `Mod+Shift+L` clean up, `Mod+P` CIP, `Alt+S` check, `Alt+C` calculated values, `Mod+Shift+M/K/F` copy as MOL / KET / image, `Alt+H/V` flip, `Alt+R` rotate, `Mod+=`/`Mod+-` zoom [KH] | not documented; about 214 `aria-` attributes in the sources, quality not assessed* |
| Marvin JS 25.3.0 | two-finger zoom and iPad long-press from 2013-2014 releases, nothing since [MJH]; no pen | digits for bond kinds (two-digit codes for query bonds), element symbols typed on the canvas, Space for abbreviations, `.` for atom query properties [MJK] | colour and icon changes "for accessibility" in 2019 [MJH]; no WCAG statement |
| new Marvin | not documented | full keyboard navigation documented (and screen-reader support with NVDA and VoiceOver for menus, dialogs and toolbars claimed [MNM]): Tab between toolbars and canvas, arrows inside toolbars and menus, on the canvas `Ctrl+Left/Right` cycles objects, Enter enters a structure, arrows walk atoms and bonds, `Shift+Space` context menu, `Alt+arrows` rotate; single keys for bonds (`1 2 3`, `w`/`h` wedges, `y` wavy, `x` cis/trans) and rings (`a` benzene, `4`-`8`, `9`/`0` chair) [MNS] | stated as a goal; no WCAG claim found in the docs read [MNS]* |
| JSME | touch claimed for phones and tablets; long press opens copy and paste; pinch disabled on iOS (2013 note) [JH, JD]; no pen | `C N O P S F L B I H R` atoms, `-` `=` bonds, `3`-`8` rings, `1` phenyl, `0` furyl, two-letter groups (`a` COOH, `y` NO2, `z` SO3H, `t` tBu, `ft` CF3), `Esc` back to carbon, `d` delete [JH] | not documented |
| OpenChemLib 9.26.0 | Pointer Events, one pointer, `touch-action: none`; touch "experimental" since 8.11.0 [OC]; no gestures | key events go to the editor; paste through `Molecule.fromText` [OT] | no ARIA, no theme (white), no i18n found* [OT] |
| Kekule.js 1.0.4 | Pointer Events; pinch and rotate gestures; **stroke recognition**: a free-hand track is simplified, snapped to atoms or closed into a ring, then laid out; hit tolerance per pointer type (mouse, pen, touch) [KK] | configurable hot keys: element letters, `1`-`4` bond order, wedge keys, `Ctrl+A/C/X/V/Z` [KK] | English and Chinese; no ARIA [KK] |
| ChemDoodle Web 11.0.0 | pinch zoom, twist rotate, `isMobile` toolbar [CD] | not documented there [CD] | a demo adds speech for hovered objects and high-contrast colours; compliance is left to the host product [CD] |
| PubChem Sketcher | none: the canvas is an image with mouse handlers [PC] | large single-key map, lower case one-letter elements, upper case common two-letter ones [PC] | `alt` text only [PC] |
| WebMolKit, Butlerov | first finger only [WM]; not documented [BU] | not documented | light and dark themes [BU] |

The bar set by the standards (summarised; full notes in the study's sources): single-key shortcuts must be active only while the editor has focus, or be remappable or switchable off [WCAG 2.1.4]; no keyboard trap [2.1.2]; every drag must have a single-pointer alternative that is not a drag, for example "click two corners" for a selection box; a keyboard alternative alone does not satisfy it [2.5.7, Understanding page]; a canvas counts as one target, but every toolbar button must be at least 24×24 px [2.5.8]; non-text contrast 3:1 for the marks that carry meaning [1.4.11]; colour is never the only signal (valence mistakes, highlights) [1.4.1]; the pointer action commits on the up event and can be cancelled [2.5.2]. `role="application"` fits a drawing surface, with `aria-roledescription` and instructions in `aria-describedby` [ARIA]; a toolbar is one Tab stop with a roving tabindex [APG]. Pointer Events give pressure (0.5 while pressed when the hardware has none), tilt, altitude and azimuth, the eraser as button 5 / buttons bit 32, coalesced and predicted events in secure contexts [PE]; Safari has tilt and pressure since 13, coalesced and predicted since 18.2, no `pointerrawupdate` [BCD]. Clipboard: `text/plain`, `text/html`, `image/png` are mandatory; custom `web <mime>` formats work in Chromium 104 and Firefox 161, not in Safari; `ClipboardItem.supports()` detects them; one item per write; Firefox and Safari need a user gesture [CB, BCD]. Forced colours repaint CSS, not canvas pixels, so the renderer must take system colours itself* [MQ].

## 3. Recurring trouble in Ketcher (what a new editor avoids by design)
From the issues cited by Ketcher's own regression tests [KI]; these are mostly bugs the Ketcher team filed and fixed, not a ranking of user complaints (the tracker was not read). Of 779 distinct issue links, about 550 concern macromolecules; the rest group as follows (a keyword count over 667 descriptions, themes overlap).
| theme | count* | examples [KI] | mzDraw by design |
|---|---|---|---|
| file format fidelity | 64 | asked V3000 or RXN or SDF, got V2000 (#5651, #5652, #5653); an attachment-point name lost from V3000 (#7877); SMILES with a trailing line break taken for a molfile (#3884) | one writer per format in Rust, golden bytes, wasm == native parity; the format is detected and the input trimmed in Rust |
| crashes, console exceptions | 37 | white screen after flipping a wedge (#5536); a minified state-store error on delete (#7386); `process is not defined` on loading KET (#8896) | no JS state; every input refused with a key, never a panic (hostile-input tests exist) |
| undo integrity | 25 | undo left bonds that could not be removed (#6129); a settings change wiped the history (#6164); undo wrong for multi-selections (#5966) | exact inverses and the property test; settings are no undo step |
| keyboard | 18 | an atom key opened a dialog instead of setting the selected atoms (#2490); Enter after undo went to the wrong place (#4338) | keys decided in Rust, tested in e2e |
| clipboard | 15 | copy key did nothing on text objects (#7152) | one clipboard path for every kind of object |
| stereo | 15 | CIP labels left behind after a move (#7899); stereo flag drifting with repeated layout (#3025) | labels computed from geometry in the display list each frame |
| zoom and placement | 14 | different zoom after `setMolecule`, paste and file open (#6370); a second `setMolecule` with a position not honoured (#6608) | one placement rule for every way in; the view never changes on paste unless asked |
| API and events | about 5 | the change event stops after a mode switch (#6071); exception with a subscription while editing a functional group (#5115); API ignores the selection (#8898); `getKet` renumbers ids on every call (#5873) | one document, one event stream, stable ids written as stored |
| view-only mode | 2 | view-only mode could still be edited (#5581, #5582) | read-only decided in Rust (the tool table is empty), tested |
| touch, accessibility, i18n | 0 linked | no regression test mentions them; the default build is English only, a build flag adds Chinese and Spanish* | built and tested from the start |

Other facts that matter for an embedder: Ketcher's React peer range and size (core 21 MB, react 13 MB, standalone 111 MB unpacked [KN]); docs and code drift (a README example calls a method that does not exist at 3.21.0; README button names absent from the type) [KR, KA]; small-molecule exports throw in macromolecule mode [KA]. Marvin JS: version 25 is the last major one, no new licences after 2026-12-31, end of life 2027-12-31, perpetual licences keep working without fixes or support [MJL]; since 2023 its releases are security fixes or "no changes" [MJH]: its users need a replacement.

## 4. Feature inventory
Columns: **who** = editors that have it (source keys of §0, `*` = not read at the source); **how** = how it works, in short; **P** = priority for mzDraw: MUST (a Ketcher user would miss it, or the embedding cannot work without it), SHOULD (competitive), LATER (after the MUST and SHOULD rows), OUT (outside `docs/mzdraw.md` scope, opened and listed, never dropped); **twin** = the scriptable twin: a `draw_edit/1` op (existing ops in plain text, proposed ones marked *new*), a setting `section.key` of `mzdraw-settings/1`, a replay-request field, or a CLI flag of `mzlab-cli mol draw`; **size** = Rust size and complexity guess (S one small PR, M two or three, L more); **state** = built (B), partly built (p), not built (-). Slice = the prompt of §7 that builds it.

### 4.1 Drawing
| feature | who | how | P (reason) | twin | size | state |
|---|---|---|---|---|---|---|
| element by click and by key on the hovered or focused atom | KH, MJK, MNS, JH, PC, KK | letter over an atom sets it; else picks the atom tool | MUST (fastest drawing) | `atom.add`, `atom.set el` | S | B |
| any element (periodic table, typed symbol) | KH, MNC, JH | table dialog or typed symbol | MUST (heteroatoms beyond the 12 keys) | `atom.set el` | S | p (`atom:<symbol>` exists, no table) → slice 11 |
| single, double, triple; click a bond to cycle the order | KH, MJK, MNS, JH | tool or key, click on a bond cycles | MUST | `bond.add o`, `bond.set o` | S | B |
| solid, hashed, wavy wedges; turn a wedge around | KH, MJK, MNS, KK | second click reverses | MUST | `bond.set s rev` | S | B |
| bond kinds: bold, dative / coordinate, hydrogen bond, crossed double (E/Z unknown), aromatic display | KH (14 kinds), MNS (`x` cis/trans), PC (either, dotted) | more values of the bond kind, written to MOL / KET / CDXML where they have a place | SHOULD (inorganic and supramolecular drawings, unknown E/Z) | *new* `bond.set k` | M | - → slice 20 |
| aromatic rings drawn as a circle or as Kekulé bonds | KH (aromatize / dearomatize), JD | a display choice, the record keeps Kekulé | SHOULD (journal habit differs) | setting `display.aromatic=kekule\|circle` | M | - |
| chain tool (drag a zigzag, live count) | KH, MV, PC (alkyl groups) | drag from an atom, 120° zigzag, count shown | MUST (aliphatic chains are slow bond by bond) | *new* `chain` (shorthand that expands to atoms and bonds, as `ring`) | S | - → slice 08 |
| rings 3-8 and benzene, fused on a bond, spiro on an atom | KH, MJK, MNS, JH, PC | click on empty space, an atom or a bond | MUST | `ring` | S | B (keys `R` + digit not bound: UI study Q2) |
| template library: bicyclics, steroids, sugars, amino acids, protecting groups, user templates | KH (`Shift+T`, user templates), MNC, CJ (drag templates), JH (groups) | a palette; click places at the pointer, on an atom or fused on a bond | MUST (Ketcher users rely on it) | *new* `tmpl` (expands to atoms and bonds) + setting section `templates` | M | - → slice 19 |
| functional groups and salts as contracted labels; expand and contract in place | KH (functional groups, salts and solvents), CD, MJK (Space = abbreviations), CJ* | a label stands for atoms; expand draws them laid out, contract folds them back | MUST (protecting groups, metabolites) | *new* `abbr.expand`, `abbr.contract` | M | p (30 labels expand only in the record) → slice 18 |
| user abbreviations | ChemDraw nicknames*, CDK (`mzdraw-competitors.md`) | a list in the settings file: name, SMILES with an attachment | SHOULD (lab-specific groups) | setting section `abbreviations` | S | - → slice 18 |
| charge | KH, MJK, JH | `+`/`-` tool or key | MUST | `atom.set chg` | S | B |
| radical, isotope, explicit H count, valence, alias in one place | KH (atom properties, `/`), MJK, MNC | a small popover next to the atom, keyboard usable (`mzdraw.md` rule 4) | MUST (no other way to set isotopes and radicals on the page) | `atom.set rad iso h lab` | M | p (fields exist in the op, no UI) → slice 11 |
| D and T as hydrogen isotopes | MJK (`D`, `T` keys) | label D / T = H with mass 2 / 3 | SHOULD (deuterated standards in MS) | `atom.set el=H iso` | S | - (read as query atoms now) |
| explicit hydrogens: add all, remove all, show at hetero / chiral only | KH, PC (H add / strip), JD (`removehs`) | real H atoms added at layout positions or taken away | SHOULD (stereo drawings with H) | *new* `h.explicit` (expands to `atom.add` / `remove`) | M | - |
| free-text labels and aliases (Ar, Nu, polymer ends) | KH (alias), MJK | label not expanded; the record gets a pseudo atom | SHOULD | `atom.set lab` | S | - (`docs/mzdraw.md` "still to build") |
| R groups R1-R32 | KH, MJK, PC | label | MUST | `atom.set lab` | S | B |
| R-group definitions and logic, attachment points, Markush | KH, MNC | definition blocks with occurrence rules | OUT (query and Markush are out of scope; listed on import) | - | L | - |
| query atoms, atom lists, query bonds, SMARTS | KH, MJK, JD, PC, OT | query layer | OUT (scope; read and listed already) | - | L | listed |
| S-groups other than abbreviations (SRU, multiple, data, copolymer) | KH, MNC | brackets with properties | OUT (scope) | - | L | listed |
| macromolecules (sequence, HELM, monomer library, antisense) | KH (macromolecule mode, about 550 of the regression issues) | a second editor mode | OUT (scope; a KET or HELM file says so in the import report) | - | L | listed |
| enhanced stereo groups ABS / AND / OR | KH, MNC, BD | groups of centres with a label | SHOULD (V3000 users; now the grouping is dropped and said) | *new* `stereo.group` | M | - |
| CIP labels R / S / E / Z drawn next to the centres | KH (`Mod+P`, Indigo), MJW (server) | computed from the record, placed by geometry | MUST (Ketcher shows them; checking stereo by eye) | setting `display.cip=true` | M | - → slice 16 |
| atom-to-atom maps in reactions, auto map | KH, MJW (server) | a number per atom; auto map needs an engine | SHOULD for manual maps (RXN), LATER for auto map | `atom.set map` *new field* | M / L | - |
| atom and bond highlight colours (kept in the document) | KH (8 colours), JA (background colours), OT (highlight) | a colour index on atoms and bonds | MUST (figures, mzLab substructure and fragment views) | *new* `atom.set hl`, `bond.set hl` | M | - → slice 15 |
| atom numbers shown | KA (debug ids), JD (`number`) | the atom index or id next to the atom | SHOULD (fragmentation discussions, NMR assignment) | setting `display.numbers=none\|index\|id` | S | - |
| terminal carbons, hetero hydrogens shown (label modes) | KA (render options), Indigo (`mzdraw-competitors.md`) | label policy | SHOULD | setting `display.carbons=none\|terminal\|all` | S | - |

### 4.2 Editing and selection
| feature | who | how | P (reason) | twin | size | state |
|---|---|---|---|---|---|---|
| rectangle selection, select all | KH, JH, KK, MV | band drag | MUST | none (view state; API `select`) | S | B |
| lasso selection | KH, MV | free polygon | MUST (crowded drawings) | none | S | - → slice 07 |
| fragment selection (one click picks the connected piece) | KH (Esc cycles selection kinds) | flood over bonds | MUST | none | S | - → slice 07 |
| select without dragging (click two corners; Shift+click adds) | WCAG 2.5.7 | two clicks span the band | MUST (WCAG 2.5.7) | none | S | - → slice 07 |
| move the selection with the keyboard | MNS (Alt+arrows rotate) | arrows nudge the selection by a tenth of a bond, Shift by one bond | MUST (WCAG 2.5.7, keyboard only) | `move` | S | - → slice 07 |
| move, rotate in steps, flip | KH (`Alt+R`, `Alt+H/V`), CJ (flip keeping stereo) | commands on the selection | MUST | `move`, `rotate`, `flip` | S | B |
| free rotation by a handle with 15° snapping | KH (rotation step setting 15°) | drag a handle around the centre | SHOULD | `rotate` | S | - |
| merge atoms on drop; join two pieces | KH, PC (merges overlapping atoms), CJ (join atoms) | an atom dropped on an atom fuses with it | MUST (closing rings, joining fragments) | *new* `merge` | M | - → slice 09 |
| clean up the drawing or the selection (2D layout) | KH (`Mod+L`, `Mod+Shift+L`), MJW (server), PC, MV (server) | positions from `Mol::layout_2d` of `mzlab-mol` | MUST (named in `mzdraw.md`; Ketcher users press it all the time) | *new* `place` (explicit positions; exact inverse = old positions) | M | p (import placement built, clean-up of a drawing not wired) → slice 10 |
| align a bond horizontal or vertical | KH | rotate the piece so a bond is level | LATER | `rotate` | S | - |
| undo and redo | all | command stack | MUST | the log holds the ops still applied | S | B |
| copy, cut, paste, duplicate | all | system clipboard with own and molfile formats | MUST | ops of paste | S | B |
| copy as MOL / KET / SMILES / image, by key | KH (`Mod+Shift+M/K/F`) | one key per format | SHOULD (speed; Ketcher habit) | `save(<format>)` | S | - → slice 21 |
| PNG and HTML on the system clipboard | CB, ChemDraw* | `image/png` with the embedded document, `text/html` with the SVG | SHOULD (paste into Word and slides) | `save("png")` | S | - → slice 21 |
| paste placement without overlap, at the pointer, view unchanged | ChemDraw smart paste (`mzdraw-competitors.md`), KI #6370 | place right of the drawing or at the pointer | SHOULD (avoid Ketcher's rezoom) | ops of paste | S | p |
| drop a file on the canvas | KR (open file), MJE* | bytes to the same door as Open | MUST (desktop habit) | replay `import` | S | - → slice 21 |
| context menu on atom, bond, selection, object, empty space (right click, long press, `Shift+F10`, Menu key) | KH, MNS (`Shift+Space`), JH (long press) | list of applicable commands decided in Rust, an ARIA menu painted by the page | MUST (touch users have no right click; discoverability) | the commands it lists | M | - → slice 12 |
| universal input bar: SMILES, InChI, name, template, element, command | MNC (search bar), PC (text line), `mzdraw-competitors.md` | one field; Rust decides what the text is; a name goes to the host's resolver | SHOULD (fastest start) | replay `import`; host `resolve` hook | M | p (Add field takes SMILES and files) |
| name to structure | PC, MV, PR (PUG-REST, CORS open) | a lookup service | SHOULD as a host hook (mzLab molecule database); never built into the component | host `resolve` | S | - |
| OCR of a structure picture | KR (remote) | image recognition | OUT for now (needs a large model or a server; LATER if asked) | - | L | - |
| measure distance and angle | none of the web editors read | - | LATER | - | S | - |

### 4.3 Reactions, text and graphics
| feature | who | how | P (reason) | twin | size | state |
|---|---|---|---|---|---|---|
| reaction arrow with a text above, plus sign | KH, MNC, JD (reaction option) | objects next to the atoms | MUST | `obj.add`, `obj.set` | S | B |
| arrow kinds: equilibrium, unbalanced equilibrium, retrosynthetic, resonance, failed / crossed, dashed, both ends | KA (17 named kinds), MNC | a `kind` on the arrow object; written to KET, CDXML, MRV where they have one | MUST (retrosynthesis and equilibria are daily) | `obj.add kind` *new field* | S | - → slice 13 |
| curved arrows for electron pushing (two-electron, one-electron) | MNC (curved), CD (electron pushing)* | a curve from a bond or lone pair to an atom or bond | SHOULD (mechanisms; mzLab `mechanism/1`) | `obj.add kind=curve` with anchors | M | - |
| text under the arrow (conditions) | KH, ChemDraw* | second text slot | SHOULD | `obj.set` *new field* `below` | S | - → slice 13 |
| multi-line text with sub- and superscript, bold, italic | KH (rich text), MNC | text runs | MUST (formulas and conditions in figures; `docs/mzdraw.md` "still to build") | `obj.set runs` *new field* | M | - → slice 14 |
| shapes: rectangle, ellipse, line, polyline | KH, MNC | simple objects | SHOULD (figure panels, TLC sketches) | `obj.add t=shape` | M | - |
| images on the canvas | KH (PNG and SVG), MNC (crop) | embedded picture | LATER | `obj.add t=image` | M | - |
| multi-tailed arrows, multi-step schemes | KH (only KET and RDF keep them) | arrow with tails | LATER | `obj.add kind` | M | listed |
| reaction roles from positions; RXN out | KH, JD | by place along the arrow | MUST | replay `out=rxn` | S | B |

### 4.4 Chemistry checks and values
| feature | who | how | P (reason) | twin | size | state |
|---|---|---|---|---|---|---|
| live valence and charge marks, never blocking | KH (structure check, `Alt+S`), MNC* | marks while drawing | MUST | `check/1` (not logged) | S | B |
| check panel: list of problems, go to each | KH (check dialog with categories) | list in the side panel; Enter moves the focus to the atom | SHOULD (a list is what a screen reader can read) | `check/1` | S | - |
| calculated values: formula, average mass, monoisotopic mass, composition, m/z of chosen adducts | KH (`Alt+C`), MNC (on the canvas), CJ (properties) | values of the selection or of each structure, in the panel or pinned under the structure | MUST (the question of a mass spectrometrist; Ketcher has it) | setting `display.values`; CLI `--json` | S | p (Hill formula in "What is drawn") → slice 17 |
| identification annotations (Schymanski level, evidence) | none | per structure, drawn under it | MUST (mzLab's own; no competitor) | `atom.ann` | M | B |
| InChI and InChIKey out | KA, PC (server), JD (on-demand module) | InChI library | SHOULD in the desktop app and CLI; LATER in the page (a C library, `mzdraw-plan.md` decision 5) | replay `out=inchi` | M | - |

### 4.5 Formats (detail: `docs/mzdraw-formats.md`; only what changes the priorities is here)
| feature | who | P (reason) | twin | size | state |
|---|---|---|---|---|---|
| MOL V2000 / V3000, SDF with data, SMILES, KET, CDXML, CML, MRV read, RXN / RDF read, RXN V2000 write, own SVG / PNG | KA, MJW, JA, PC | MUST | replay `import`, `out` | - | B |
| CXSMILES write | KA | SHOULD (keeps coordinates and enhanced stereo) | `out=cxsmiles` | S | - |
| RXN V3000 and RDF write, atom maps | KA | SHOULD | `out=rxn-v3000`, `out=rdf` | S | - |
| CDX binary read | KA, CJ | SHOULD (ChemDraw users send CDX) | `import` | M | - |
| MRV write | MJE | LATER (Marvin JS ends 2027) | `out=mrv` | S | - |
| PDF out with the embedded document | PC (PDF), ChemDraw* | SHOULD (print-ready figures; `mzdraw-plan.md` decision 4) | `out=pdf` | M | - |
| image options: margin, transparent or coloured background, scale | KA (`generateImage` options) | SHOULD | replay `set` of an `export` section | S | - |
| HELM, FASTA, sequences, IDT | KA | OUT (macromolecules) | - | - | listed |

### 4.6 View, settings, look
| feature | who | how | P (reason) | twin | size | state |
|---|---|---|---|---|---|---|
| zoom, pan, fit; two-finger pan and pinch | KH, KK, CD | wheel, keys, gestures | MUST | none (view) | S | B |
| zoom to the selection, 100 % key | KH (`Mod+Shift+0`) | - | SHOULD | none | S | - |
| drawing style with journal presets, physical units | KO (ACS style), ChemDraw* | `mzdraw-settings/1` | MUST | `style.*` | S | B |
| light and dark theme tokens | BU (themes), SD (themes) | CSS custom properties read once and handed to Rust as a palette | MUST (owner feedback) | setting `theme.*` (page only) | S | UI slice PR 3 (not repeated here) |
| forced colours and high contrast on the canvas | MQ; nobody does it | system colours read from CSS and given to the renderer | MUST (WCAG 1.4.11 under forced colours; `docs/mzdraw.md` "still to build") | setting `theme.forced` | S | - → slice 04 |
| reduced motion | MQ | no animated zoom or pan | SHOULD | none | S | - → slice 04 |
| UI languages through keys | KA (en, zh, es at build time*), KK (en, zh) | Rust returns keys, the host resolves them | MUST (mzLab rule: every user text through lang keys) | host `t` hook | S | p (keys in `web/lang/en`) → slice 04 |
| hide or add toolbar buttons | KR (`buttons`, `customButtons`), MJE (`setDisplaySettings`), JD (options) | a token list of tools | SHOULD (a host that wants a small editor) | attribute `tools` | S | - → slice 02 |
| read-only viewer of the same component | JD (`depict`), OT (`readOnly`), KI #5581 | no tool, hover and selection off, still focusable and described | MUST (mzLab tables and reports reuse the renderer) | attribute `readonly` | S | - → slice 05 |
| shortcut remapping and switching single keys off | KK (configurable hot keys), WCAG 2.1.4 | a `shortcuts` section of the settings file; single keys work only while the editor has focus | MUST (WCAG 2.1.4; settings study already reserved the section) | setting section `shortcuts` | S | - → slice 06 |
| key in every tooltip, help list of the keys | KH (tooltips show keys) | from the same table | MUST | - | S | p (help list built; tooltips: UI slice) |
| 3D view | KR (Miew), MV | separate viewer | LATER (desktop app; not a 2D editing need) | - | L | - |
| full-screen toggle | KH | host concern | LATER | host | S | - |

### 4.7 Input: touch, pen, keyboard, accessibility
| feature | who | how | P (reason) | twin | size | state |
|---|---|---|---|---|---|---|
| one finger draws, two fingers pan and zoom, palm rejection, long press selects, pen eraser | KK, CD (partly) | `editor_touch.rs` | MUST | none | M | B |
| "finger pans, pen draws" setting | none read; WCAG 3 draft "pressure not relied on" | when a pen was seen, fingers pan | SHOULD (tablet users with a stylus) | setting `input.finger=draw\|pan` | S | - |
| per-pointer hit radius | KK | radius by pointer type | MUST | none | S | B (8 / 16 / 22 px) |
| pen pressure and tilt | PE | used only for preview line weight, never required | LATER | none | S | - |
| stroke recognition (draw a zigzag or a ring by hand) | KK | a free track simplified and snapped | LATER (nice on tablets, risky to get right) | ops it produces | L | - |
| double tap: edit the label of an atom | none read | opens the atom popover | SHOULD | - | S | - → slice 11 |
| keyboard focus on atoms, walk along bonds, grow, set | MNS | `editor_keys.rs` | MUST | ops | M | B |
| focus on bonds and objects; keyboard placing of arrows and texts | MNS (objects cycle) | the focus ring moves to a bond or an object | MUST (2.1.1 for every object) | ops | S | - → slice 07 |
| no keyboard trap, roving tabindex toolbar | WCAG 2.1.2, APG | `Tab` never taken | MUST | - | S | B (toolbar: UI slice) |
| live announcements and the text outline | ARIA (`aria-live`; `ariaNotify` in 1.3 draft) | keys from Rust | MUST | - | S | B |
| test with a real screen reader | none | NVDA and VoiceOver pass recorded | MUST (claim only what is tested) | - | S | - (needs a person) |

### 4.8 Embedding and hosts
| feature | who | how | P (reason) | twin | size | state |
|---|---|---|---|---|---|---|
| several editors on one page | JD, OT, KK, CD | one editor state per instance | MUST (mzLab pages show several structures) | - | M | - (one global state in `mzdraw-wasm`) → slice 01 |
| custom element `<mzdraw-editor>` | OT, KK | autonomous element, shadow root | MUST | - | M | - → slice 02 |
| one `ready` promise; calls made before ready are queued | KR (init message), MJE (promise) | the element resolves `ready` once the module is in | MUST (every competitor makes hosts race) | - | S | - → slice 02 |
| change event carrying the `draw_edit/1` ops of the step | KR (operation records), KK (operation history) | `change` with `{ops, origin: edit\|undo\|redo\|load\|api}` | MUST (hosts can sync, log, replay) | the ops themselves | S | - → slice 02 |
| apply ops from the host | none | `apply(ops)` returns the result, one undo step | MUST (agents, mzLab panels) | `draw_edit/1` | S | - → slice 02 |
| selection and tool events, selection API | JA (`AtomClicked`), OT, KK | `selectionchange`, `toolchange`; `select(ids)`, `selection` | MUST (mzLab panels act on the selection) | - | S | - → slice 02 |
| host overlays: marks and highlights that are not part of the document | JA (`AtomHighlight`), mzLab `TPDraw` (`mzdraw.md`) | `mark(list)` paints over the drawing, never in the log | MUST (substructure hits, fragments, evidence colours) | - | S | - → slice 15 |
| import report as an event | none | `importreport` `{read, lost}` | MUST (the never-silent rule) | `Imported::notes` | S | p → slice 02 |
| file open and save delegated to the host | CJ (none), KR (download only) | host adapter: browser download / input, or native dialogs | MUST (owner decision: two hosts) | - | S | - → slice 03 |
| storage delegated to the host (autosave, settings), no handle kept open while idle | none | adapter `read` / `write`; default OPFS opened per write | MUST (owner decision) | - | S | p (OPFS built in the page) → slice 03 |
| lazy loading without a bundler | JD (`nocache` loader), OT (ES module) | the element imports its module on first connect, or takes a loader from the host | MUST (mzLab `loadScript` / `loadWasm`) | - | S | - → slice 02 |
| form association (`name`, value in a form, validity) | HTML, TAG | `ElementInternals.setFormValue`, `setValidity` | SHOULD (registration forms, ELN fields) | - | S | - → slice 05 |
| cross-origin embedding: iframe page plus a versioned message protocol | MJE (postMessage example), KR (init message) | `embed.html` relays the element API over `postMessage` with an origin check | SHOULD (hosts on other origins, ELNs) | the same ops | S | - |
| no network, no eval, CSP-friendly | PC, MJW, CJ, CD all need servers | a test that the component makes no request beyond its own files | MUST (files never leave the computer) | - | S | p (page CSP) → slice 02 |
| desktop app for macOS and Windows (native dialogs, file associations, recent files) | BD (Windows), ChemDraw* | Tauri shell around the same element; owner decision | MUST (owner) | CLI parity already | L | - (shell outside this web study; it plugs into slice 03) |
| capability discovery: version, formats, settings schema, ops list | KA (methods only) | `MzdrawEditor.info()` | SHOULD (hosts and agents adapt) | `settings_schema` exists | S | p |

Counts of this inventory (110 rows): **MUST 61** (22 built, 10 partly built, 1 in the UI slice, 28 to build), **SHOULD 34**, **LATER 9**, **OUT 6**. A row marked "SHOULD / LATER" or "SHOULD in the app, LATER in the page" counts as SHOULD; "OUT for now" counts as OUT.

## 5. Embedding API proposal: `<mzdraw-editor>`
Design rules taken from the sources: an autonomous custom element, since customised built-ins do not exist in Safari [BCD]; attributes only for small values, mirrored as properties [TAG]; big data through methods; every asynchronous call returns a Promise and takes an options dictionary with an optional `AbortSignal` [TAG]; events are notifications, named in the present tense, bubbling and composed, with the state readable on the element [TAG]; enumerations are strings [TAG]; bytes are `Uint8Array` [TAG]; errors carry the i18n key of Rust.

### 5.1 Markup
```html
<script type="module" src="mzdraw-editor.js"></script>
<mzdraw-editor name="structure" tools="select erase bond ring atom charge label arrow text"
               style-preset="acs-1996" readonly></mzdraw-editor>
```
| attribute (property) | value | default |
|---|---|---|
| `readonly` (`readOnly`) | boolean: a viewer (no tools, still focusable, outlined, copyable) | off |
| `tools` (`tools`) | space-separated tool tokens to show; empty = no toolbar (host draws its own) | all |
| `style-preset` (`stylePreset`) | a preset name of `mzdraw-settings/1` | `mzlab` |
| `name` (`name`), `required` | form field name; the form value is the document (or `value-format`) | none |
| `value-format` (`valueFormat`) | `mzdraw`, `molfile`, `smiles`, ... (what the form submits) | `mzdraw` |
| `module` (`module`) | URL of `mzdraw_wasm.js` when no loader is configured | `./pkg/mzdraw_wasm.js` beside the element script |

### 5.2 Methods and properties (every method is a thin call into Rust; no chemistry in JS)
| member | returns | what |
|---|---|---|
| `ready` | `Promise<void>` | resolves once the module is loaded and the instance made; calls before it are queued in order |
| `load(input, {format, mode: "replace"\|"add", signal})` | `Promise<ImportReport>` | text or `Uint8Array`; the format is detected in Rust when not given; `{read, lost: [{what, count}]}` |
| `save(format, {signal})` | `Promise<string\|Uint8Array>` | every `Editor::save` kind; PNG through the page rasteriser as today |
| `apply(ops, {signal})` | `Promise<ApplyResult>` | a `draw_edit/1` array, one undo step; refused as a whole with a key |
| `undo()`, `redo()`, `clear()` | `Promise<void>` | the same commands as the buttons |
| `document` | string (get / set) | the `mzdraw/1` text; set = `load(..., {mode: "replace"})` without history |
| `selection` | `{atoms, bonds, objects}` (get) | ids |
| `select(sel)` | `Promise<void>` | sets the selection |
| `mark(marks)` | `Promise<void>` | host overlay: `[{atoms, bonds, colour, label}]`; never in the document or the log; `mark([])` clears |
| `set(setting)`, `settings` | `Promise<void>`, string | `section.key=value` and the settings file, as `Editor::set` / `style()` |
| `command(name)` | `Promise<void>` | any page command (`rotate-l`, `fit`, ...) |
| `focusAtom(id)` | void | puts the keyboard focus on an atom |
| `MzdrawEditor.configure(host)` | void | the host adapter of §5.4, once per page |
| `MzdrawEditor.info()` | `Promise<Info>` | version, formats in and out, ops, settings schema |

### 5.3 Events (all `CustomEvent`, bubbling, composed; `detail` is a copy, the element holds the state)
| event | detail | when |
|---|---|---|
| `ready` | `{version}` | once |
| `change` | `{ops, origin: "edit"\|"undo"\|"redo"\|"load"\|"api", seq}` | after every committed step; `ops` are the `draw_edit/1` ops of the step (for undo: the inverse ops applied), so a host can mirror the document or keep its own log |
| `input` | `{tool}` | while a drag previews (no document change); rate-limited to frames |
| `selectionchange` | `{atoms, bonds, objects}` | selection changed |
| `toolchange` | `{tool}` | tool changed |
| `importreport` | `{read, lost}` | after every load, paste and drop |
| `requestfile` | `{kind: "open"\|"save", format}` | only when no host adapter handles files: the host may `preventDefault()` and do it itself |
| `error` | `{key, params}` | a refused call or input, with the i18n key |
`on<event>` handler properties exist for each event [TAG]. Events fire before the promise of the call that caused them resolves [TAG].

### 5.4 Host adapter (one object, every member optional; the defaults are the browser behaviour of today's page)
```js
MzdrawEditor.configure({
  load: () => import("./pkg/mzdraw_wasm.js"),            // mzLab: () => loadWasm("mzdraw")
  t: (key, params) => string,                            // i18n: mzLab resolves draw2.* keys; default = bundled English table
  files: {
    open: ({accept}) => Promise<{name, bytes} | null>,   // browser: <input type=file>; desktop: native dialog
    save: ({name, mime, bytes}) => Promise<boolean>,     // browser: download link; desktop: native save dialog
  },
  storage: {                                              // autosave and remembered settings
    read: (name) => Promise<Uint8Array | null>,
    write: (name, bytes) => Promise<void>,               // default: OPFS, the file opened, written and closed per call
  },
  clipboard: { read, write },                             // desktop: native formats; default: async clipboard + events
  resolve: (text) => Promise<string | null>,             // name lookup (mzLab molecule database); default: none
  theme: () => ({...tokens}),                             // colours handed to Rust; default: read CSS custom properties
});
```
- Keyboard: single-key shortcuts act only while an editor has focus [WCAG 2.1.4]; `Tab` and `Shift+Tab` always leave [2.1.2]; keys are matched on `KeyboardEvent.key` in Rust as today, ignored during IME composition.
- Pointer: `touch-action: none` on the canvas only, pointer capture, coalesced events forwarded when present [PE]; the canvas has `role="application"`, `aria-roledescription` "molecule editor" and the outline in `aria-describedby` [ARIA].
- Clipboard: the `copy`, `cut`, `paste` events first; custom web formats only after `ClipboardItem.supports()` [CB].
- Cross-origin hosts: `embed.html` holds one element and relays this API as messages `{v: "mzdraw-embed/1", id, call, args}` / `{v, id, result | error}` and the events as `{v, event, detail}`, checking `event.origin` against an allow list given in its URL and the shape of every message [HTML]; never `*` as the target origin for documents [HTML]. The iframe needs no `sandbox` relaxation beyond `allow-scripts`; `allow-scripts` together with `allow-same-origin` on same-origin content can remove the sandbox [HTML].

### 5.5 Two hosts, one component (owner decision relayed by COO-MZDRAW)
mzDraw ships in two ways from one component: (a) a standalone desktop app for macOS and Windows for people who want only the editor; (b) bundled by default inside mzLab, on its site and in the mzLab app (Tauri). Both use `<mzdraw-editor>` and the files `mzdraw-editor.js` + `pkg/` unchanged; what differs is only the host adapter of §5.4.
| need | standalone desktop app | inside mzLab (site and app) | the component |
|---|---|---|---|
| load the module | `import()` from the app bundle | `loadScript("mzdraw-editor.js")` then `configure({load: () => loadWasm("mzdraw")})`, under mzLab's per-module size cap | never assumes a bundler, a base URL or a global; accepts the loader |
| documents in and out | `files` = native open and save dialogs (Tauri commands), recent files, file associations for `.mzdraw`, `.mol`, `.sdf`, `.cdxml`, `.ket` | `load()` / `save()` called by mzLab's panels; on the site `files` = browser input and download | delegates every file action; never writes to disk itself |
| autosave and settings | `storage` = the app data folder | `storage` = mzLab's storage (OPFS on the site, app folder in the app) | one `write` per save, nothing held open while idle; no OPFS handle kept |
| user text | bundled English table | `t` = mzLab lang keys (`draw2.*`), English only | Rust returns keys and parameters only (already the rule) |
| look | its own theme tokens | mzLab's theme tokens | colours come in through `theme`, are passed to Rust as a palette; forced colours override |
| layout | full window | a panel inside mzLab's page | fills its box; a `ResizeObserver` re-fits; no fixed size |
| settings and style | settings file in the app folder | mzLab may inject a settings file (`set`, `settings`) | the same `mzdraw-settings/1` |
| clipboard | native multi-format clipboard through `clipboard` | browser clipboard | default async clipboard, host override |
| parity | the same bytes as `mzlab-cli mol draw` | the same | the component emits only `draw_edit/1` ops and requests |

### 5.6 What the bridge must change first
- `crates/mzdraw-wasm` keeps one editor in a `thread_local` (`PAGE`); every exported function acts on it. Several editors on one page need an instance handle (an exported struct, or a table of editors keyed by a number) with the same functions as methods, and `free` on disconnect. This is slice 01, and every embedding slice depends on it.
- The page `web/mzdraw.js` becomes the first user of the element (the page is `<mzdraw-editor>` plus its side panel), so the e2e test covers the element and no second code path exists.

## 6. Where mzDraw can be better than Ketcher (summary for the owner)
1. One code path for page, desktop app and CLI with the same bytes (built: parity test); no editor read has it.
2. No server and no network for any feature (Ketcher needs a remote service for OCR, Marvin JS for most formats, ChemDraw JS for conversion, PubChem for every gesture).
3. Synchronous-feeling start with one `ready` promise and queued calls; no React peer, no iframe, no 111 MB package.
4. A change event that carries the exact ops of the step, replayable from the CLI.
5. Undo that cannot corrupt (exact inverses, property test), settings outside the history (Ketcher #6164 class).
6. Real accessibility: focus walk on atoms and bonds, outline, live line, drag alternatives, remappable keys, forced colours; tested in e2e.
7. Touch and pen designed in (palm rejection, eraser, two-finger view), where Ketcher binds mouse events.
8. An import report that never drops anything silently.
9. Identification annotations and m/z values for the mass spectrometrist, which no editor has.

## 7. DEV slices (one prompt each in `docs/prompts/`)
Order = dependency order; each is one small PR. Slices 01-05 make the component; 06-21 are the MUST features not yet built (and a few SHOULD that ride along). The UI slice (toolbar, icons, light and dark tokens, PR #32) and the desktop shell are not repeated here.
| NN | slug | content | rows of §4 | size |
|---|---|---|---|---|
| 01 | `instances` | editor instances in `mzdraw-wasm` instead of one global | several editors | M |
| 02 | `element` | `<mzdraw-editor>`: ready, load, save, apply, change / selection / tool / importreport events, lazy module, `tools`, no-network test; the page uses it | custom element, ready, change ops, apply, selection, import report, lazy loading, no network | M |
| 03 | `host-adapter` | `configure({files, storage, load})`: file open / save and autosave delegated; OPFS written per call | files, storage | S |
| 04 | `theme-i18n` | `t` hook, palette into Rust, forced colours, reduced motion | forced colours, reduced motion, languages | S |
| 05 | `viewer-form` | `readonly` viewer and form association | read-only, form | S |
| 06 | `shortcuts` | `shortcuts` section of the settings file, remap and switch off, keys only on focus | shortcut remapping | S |
| 07 | `selection` | lasso, fragment selection, click-two-corners band, keyboard nudge, focus on bonds and objects | lasso, fragment, 2.5.7, nudge, focus on bonds | M |
| 08 | `chain` | chain tool and op `chain` | chain | S |
| 09 | `merge` | merge on drop, op `merge` | merge | M |
| 10 | `clean-up` | clean up of the drawing or selection with `Mol::layout_2d`, op `place` | clean up | M |
| 11 | `atom-popover` | atom properties popover (isotope, radical, H count, charge, label, any element); double tap opens it | any element, radical / isotope, double tap | M |
| 12 | `context-menu` | context menu decided in Rust, right click, long press, Menu key | context menu | M |
| 13 | `arrow-kinds` | arrow kinds and the text below | arrow kinds, conditions | S |
| 14 | `rich-text` | multi-line text with runs (sub, sup, bold, italic) | rich text | M |
| 15 | `highlight-marks` | highlight colours in the document and host marks | highlight, host overlays | M |
| 16 | `cip` | CIP labels drawn (`display.cip`) | CIP | M |
| 17 | `values` | formula, masses, m/z of adducts per structure and selection | calculated values | S |
| 18 | `abbreviations` | expand and contract in place, user abbreviations | functional groups, user abbreviations | M |
| 19 | `templates` | template library and user templates, op `tmpl` | template library | M |
| 20 | `bond-kinds` | bold, dative, hydrogen, crossed double | bond kinds | M |
| 21 | `drop-copy-as` | drop a file, copy as MOL / KET / SMILES / PNG keys, PNG and HTML on the clipboard | drop, copy as, PNG clipboard | S |
Not given a prompt yet (SHOULD or LATER, after these): aromatic circles, D / T, explicit H, free-text labels, enhanced stereo, atom maps, numbers and carbon labels, free rotation handle, check panel, universal input bar, curved arrows, shapes, images, CXSMILES / RXN V3000 / RDF out, CDX read, PDF, image options, zoom to selection, finger-pans setting, iframe protocol, capability discovery.
