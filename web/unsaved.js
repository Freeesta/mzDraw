// The "Save changes?" question of the unsaved-changes guard (the dialog of index.html, #d2-unsaved). It only shows the question and
// returns the answer ("save", "discard" or "cancel"); whether to ask and what each answer does are decided in Rust (`unsaved_guard`,
// `unsaved_choice`). An alert dialog: Save has the focus, Tab stays inside, Escape is Cancel, Enter presses the focused button, and the
// focus goes back to where it was.

const $ = (/** @type {string} */ id) => /** @type {HTMLElement} */ (document.getElementById(id));
/** @type {Promise<string> | null} */ let open = null;

/** shows the question; resolves with the answer @param {{ t: (k: string, p?: any) => string, fallback: () => void }} ctx @returns {Promise<string>} */
export function unsavedDialog(ctx) {
  if (open) return open;
  const root = $("d2-unsaved"), save = $("d2-unsaved-save"), discard = $("d2-unsaved-discard"), cancel = $("d2-unsaved-cancel");
  const buttons = [save, discard, cancel];
  let prior = /** @type {HTMLElement | null} */ (document.activeElement);
  open = new Promise(resolve => {
    /** @param {string} answer */
    const done = answer => {
      root.removeEventListener("keydown", key);
      document.removeEventListener("focusin", keep, true);
      root.hidden = true; open = null;
      // back to where it was; when that is gone (a flyout that closed with the click) the canvas takes it
      if (prior && prior.isConnected && typeof prior.focus === "function") prior.focus({ preventScroll: true });
      if (!prior || document.activeElement !== prior) ctx.fallback();
      resolve(answer);
    };
    /** @param {KeyboardEvent} e */
    const key = e => {
      if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); done("cancel"); }
      else if (e.key === "Tab") {
        const i = buttons.indexOf(/** @type {HTMLElement} */ (document.activeElement)), n = buttons.length;
        e.preventDefault();
        buttons[(i < 0 ? 0 : i + (e.shiftKey ? n - 1 : 1)) % n].focus();
      }
    };
    // a focus that goes elsewhere (the page gives it to the canvas after a click on a button) is brought back, and is where it returns to
    const keep = (/** @type {FocusEvent} */ e) => { if (!root.contains(/** @type {Node} */ (e.target))) { prior = /** @type {HTMLElement} */ (e.target); save.focus(); } };
    save.onclick = () => done("save"); discard.onclick = () => done("discard"); cancel.onclick = () => done("cancel");
    root.addEventListener("keydown", key);
    document.addEventListener("focusin", keep, true);
    root.hidden = false; save.focus();
  });
  return open;
}
