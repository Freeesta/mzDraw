// The context menu of the canvas: paints the menu the module builds (`context_menu`: what is under the pointer decides the items, the
// selection follows the click), forwards the choice (`context_run`, one undo step) and does what the module asks the page to do
// after it (`context_ask`: a field to fill, a file to write, the clipboard of the system). No decision and no chemistry here: the items,
// their order, which are enabled, which is checked, the keys shown, when a menu is out of date (`context_valid`) and where a menu
// opened from the keyboard goes (`context_anchor`) all come from Rust (crates/mzdraw/src/context.rs). This file is the interface:
// the DOM of the menu, its keyboard walk (arrows, Home, End, Enter, Escape, letters), its place on the screen and its closing.

const ASK = 256;
const $ = (/** @type {string} */ id) => /** @type {HTMLElement} */ (document.getElementById(id));

/** @param {{ed: any, t: (key: string, params?: any) => string, cv: HTMLCanvasElement, after: (flags: number) => number,
 *   say: (text: string) => void, pick: (palette: string, field: string) => void, saveDefault: () => any, copyPng: () => Promise<void>}} h */
export function contextMenu(h) {
  const { ed, t, cv } = h, host = $("d2-menu");
  let shown = false;
  /** where the menu was opened, in the window, and how far from the point it stands @type {{x: number, y: number, off: number} | null} */
  let at = null;

  /** the text of an item: the catalog by its id, else the module's own label @param {{id: string, label: string}} item */
  const label = item => { const k = `draw2.ctx.${item.id}`, v = t(k); return v && v !== k ? v : item.label; };

  /** the buttons that can be reached by the keyboard: enabled, and not inside a group that is folded */
  const reach = () => /** @type {HTMLElement[]} */ ([...host.querySelectorAll("button:not([aria-disabled='true'])")]).filter(b => !b.closest("[hidden]"));

  /** @param {HTMLElement} b @param {boolean} on */
  function fold(b, on) {
    const sub = $(b.getAttribute("aria-controls") || "");
    sub.hidden = !on; b.setAttribute("aria-expanded", String(on)); place();
  }

  /** one item (and its group) @param {any} item @param {HTMLElement} into @param {number} n */
  function make(item, into, n) {
    const b = document.createElement("button"), name = document.createElement("span");
    b.type = "button"; b.tabIndex = -1; b.dataset.id = item.id; b.dataset.testid = "d2-ctx-" + item.id;
    b.setAttribute("role", item.checked === undefined ? "menuitem" : "menuitemradio");
    if (item.checked !== undefined) b.setAttribute("aria-checked", String(item.checked));
    if (!item.enabled) b.setAttribute("aria-disabled", "true");
    name.className = "n"; name.textContent = label(item); b.append(name);
    if (item.key) {
      const k = document.createElement("span");
      k.className = "k"; k.textContent = item.key; b.append(k); b.setAttribute("aria-keyshortcuts", item.key.replace("Ctrl", "Control"));
    }
    into.append(b);
    if (!item.children) return;
    const sub = document.createElement("div");
    sub.className = "sub"; sub.id = `d2-ctx-sub-${n}`; sub.hidden = true; sub.setAttribute("role", "menu"); sub.setAttribute("aria-orientation", "vertical"); sub.setAttribute("aria-label", name.textContent);
    b.setAttribute("aria-haspopup", "menu"); b.setAttribute("aria-expanded", "false"); b.setAttribute("aria-controls", sub.id); b.dataset.group = "1";
    item.children.forEach((/** @type {any} */ c, /** @type {number} */ i) => make(c, sub, n * 100 + i + 1));
    into.append(sub);
  }

  /** the menu the module gave, as DOM @param {any} menu @param {string} kind */
  function build(menu, kind) {
    const root = document.createElement("div");
    root.className = "ctx"; root.tabIndex = -1; root.setAttribute("role", "menu"); root.setAttribute("aria-orientation", "vertical");
    root.setAttribute("aria-label", t("draw2.ctx.menu", { target: t(`draw2.ctx.target.${menu.target}`) }));
    // a canvas with nothing drawn has nothing to export: the module says so, in words, instead of an item
    if (menu.notice) {
      const n = document.createElement("button");
      n.type = "button"; n.tabIndex = -1; n.className = "note"; n.setAttribute("role", "menuitem"); n.setAttribute("aria-disabled", "true"); n.textContent = t(menu.notice);
      root.append(n);
    }
    menu.items.forEach((/** @type {any} */ item, /** @type {number} */ i) => make(item, root, i + 1));
    host.dataset.kind = kind; host.dataset.target = menu.target;
    host.replaceChildren(root);
    return root;
  }

  /** opens the menu for what is at (x, y) of the canvas, in pixels; `kind` is the pointer kind (0 mouse, 1 pen, 2 finger); `byKey`
   * puts the focus on the first item (a menu from the keyboard) instead of on the menu @param {number} x @param {number} y @param {number} kind @param {boolean} [byKey] */
  function open(x, y, kind, byKey = false) {
    close(false);
    const menu = JSON.parse(ed.context_menu(x, y, kind));
    h.after(1 | 4);                                    // the selection follows what was clicked
    const root = build(menu, byKey ? "key" : ["mouse", "pen", "touch"][kind] || "mouse");
    host.hidden = false; shown = true;
    // beside the point, not under it (a finger would hide it); inside the window, above the point when it does not fit below
    const r = cv.getBoundingClientRect(), off = kind ? 12 : 6;
    at = { x: r.left + x, y: r.top + y, off };
    place();
    (byKey ? reach()[0] || root : root).focus({ preventScroll: true });
  }

  /** puts the menu beside the point it was opened at, inside the window: left of the point when it does not fit on the right, above it
   * (or as high as the window allows) when it does not fit below; called again when a group opens or folds and the menu changes size */
  function place() {
    if (!at) return;
    const box = host.firstElementChild?.getBoundingClientRect();
    if (!box) return;
    let left = at.x + at.off, top = at.y + at.off;
    if (left + box.width > innerWidth - 4) left = Math.max(4, at.x - at.off - box.width);
    if (top + box.height > innerHeight - 4) top = Math.max(4, Math.min(at.y - at.off - box.height, innerHeight - 4 - box.height));
    Object.assign(host.style, { left: left + "px", top: top + "px" });
  }

  /** a menu from the keyboard (the Menu key, Shift+F10): the module says where it goes */
  function openAtAnchor() {
    const [x, y] = ed.context_anchor(cv.clientWidth, cv.clientHeight);
    open(x, y, 0, true);
  }

  /** closes the menu; the focus goes back to the canvas when it was in the menu @param {boolean} [back] */
  function close(back = true) {
    if (!shown) return;
    const had = host.contains(document.activeElement);
    shown = false; host.hidden = true; host.replaceChildren(); ed.context_close();
    if (had && back) cv.focus({ preventScroll: true });
  }

  /** closes the menu when the module says it no longer describes the state (any change of the drawing, the selection, the clipboard
   * or the view): a stale menu never keeps its old target */
  function sync() { if (shown && !ed.context_valid()) close(); }

  /** runs an item @param {string} id */
  function run(id) {
    const f = ed.context_run(id), asked = f & ASK ? ed.context_ask() : "";
    close(false);
    cv.focus({ preventScroll: true });
    h.after((f & ~ASK) | 2);
    if (asked) answer(asked);
  }

  /** what the module asked the page to do after an item @param {string} asked */
  function answer(asked) {
    const i = asked.indexOf(":"), what = i < 0 ? asked : asked.slice(0, i), arg = i < 0 ? "" : asked.slice(i + 1);
    if (what === "clipboard") navigator.clipboard?.writeText(ed.clipboard("molfile")).catch(() => { });      // best effort: the editor keeps its own copy
    else if (what === "element") { h.pick("atoms", "d2-element"); h.say(t("draw2.ctx.say.element")); }
    else if (what === "label") { h.pick("charges", "d2-label"); h.say(t("draw2.ctx.say.label")); }
    else if (what === "text") { h.pick("objects", "d2-text-tool"); h.say(t("draw2.ctx.say.text")); }
    else if (what === "import") side("d2-text");
    else if (what === "data") side("d2-data", arg);
    else if (what === "annotation") side("d2-ann", arg);
    else if (what === "fit") h.after(ed.fit(cv.clientWidth, cv.clientHeight));
    else if (what === "save-image") h.saveDefault();
    else if (what === "copy-as") copyAs(arg);
  }

  /** opens the side palette that holds `box` and puts the keyboard on its field, for the structure `anchor` when it has one @param {string} box @param {string} [anchor] */
  function side(box, anchor) {
    const b = $(box), d = b.closest("details");
    if (d) d.open = true;
    const field = (anchor ? b.querySelector(`fieldset[data-anchor="${anchor}"]`) : b)?.querySelector("input, select, textarea, button") || b;
    if (field instanceof HTMLElement) field.focus();
    if (b instanceof HTMLTextAreaElement) b.select();
  }

  /** copies the selection to the clipboard of the system as a text or a picture @param {string} kind */
  async function copyAs(kind) {
    try {
      if (kind === "png") await h.copyPng(); else await navigator.clipboard.writeText(ed.context_copy_as(kind));
      h.say(t("draw2.ctx.say.copied", { what: t(`draw2.ctx.sel.copyas.${kind}`) }));
    } catch { h.say(t("draw2.ctx.say.copyFailed")); }
  }

  host.addEventListener("contextmenu", e => e.preventDefault());       // no menu of the browser on ours either
  host.addEventListener("click", e => {
    const b = /** @type {HTMLElement} */ (e.target).closest("button");
    if (!b || b.getAttribute("aria-disabled") === "true") return;
    if (b.dataset.group) {
      // Enter or Space (a click with no pointer) on a group opens it and moves the focus in, as the Right arrow does
      const on = b.getAttribute("aria-expanded") !== "true";
      fold(b, on);
      if (on && e.detail === 0) /** @type {HTMLElement | null} */ ($(b.getAttribute("aria-controls") || "").querySelector("button:not([aria-disabled='true'])"))?.focus({ preventScroll: true });
    } else if (b.dataset.id) run(b.dataset.id);
  });
  host.addEventListener("keydown", e => {
    const items = reach(), at = items.indexOf(/** @type {HTMLElement} */ (document.activeElement)), key = e.key;
    const go = (/** @type {HTMLElement | undefined} */ b) => { if (b) b.focus({ preventScroll: true }); e.preventDefault(); };
    const group = /** @type {HTMLElement | null} */ (document.activeElement)?.closest(".sub");
    if (key === "ArrowDown") go(items[(at + 1) % items.length]);
    else if (key === "ArrowUp") go(items[at <= 0 ? items.length - 1 : at - 1]);
    else if (key === "Home") go(items[0]);
    else if (key === "End") go(items[items.length - 1]);
    else if (key === "ArrowRight" && at >= 0 && items[at].dataset.group) {
      fold(items[at], true); go(/** @type {HTMLElement} */ ($(items[at].getAttribute("aria-controls") || "").querySelector("button:not([aria-disabled='true'])")));
    } else if (key === "ArrowLeft" && group) {
      const owner = /** @type {HTMLElement} */ (host.querySelector(`[aria-controls="${group.id}"]`));
      fold(owner, false); go(owner);
    } else if (key === "Escape" || key === "Tab") { e.preventDefault(); close(); }
    else if (key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey && key !== " ") {
      // a letter goes to the next item that starts with it
      const next = [...items.slice(at + 1), ...items.slice(0, at + 1)].find(b => (b.querySelector(".n")?.textContent || "").toLowerCase().startsWith(key.toLowerCase()));
      go(next);
    }
  });
  // a press anywhere outside the menu, the window losing the focus and the Escape key from elsewhere close it
  document.addEventListener("pointerdown", e => { if (shown && !host.contains(/** @type {Node} */ (e.target))) close(false); }, true);
  addEventListener("blur", () => close(false));
  document.addEventListener("keydown", e => { if (shown && e.key === "Escape" && !host.contains(/** @type {Node} */ (e.target))) close(); }, true);

  return { open, openAtAnchor, close, sync, isOpen: () => shown };
}
