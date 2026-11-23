// The soft change of theme. The colours of the page (panels, palettes, flyouts, the area behind the canvas) cross-fade in CSS while the root
// carries `d2-fading` (mzdraw.css); the canvas is painted by script, so the old picture, kept as a copy over the new one, fades out. Nothing
// here decides anything about the drawing: it runs when the theme attribute or the system setting changed, never on the first paint (the
// theme is on the root before it, by theme.js), and not at all for a person who asked for less motion.

/** how long the fade lasts, in milliseconds (the same as the transitions of mzdraw.css) */
const MS = 300;

/** @param {HTMLCanvasElement} cv the canvas @returns {{before: () => void, change: () => void}} `before` is called just before the page puts a new theme on
 * the root (the canvas is copied while it still shows the old theme), `change` when the theme attribute or the system setting changed, before the canvas is
 * painted again (a theme a host set is only seen here, and then the copy is made here) */
export function themeFade(cv) {
  const root = document.documentElement, still = matchMedia("(prefers-reduced-motion: reduce)");
  let paper = getComputedStyle(cv).backgroundColor, timer = 0;
  /** the copy of the canvas as it is, with the paper it is on, laid over the canvas @type {HTMLCanvasElement | null} */
  let held = null;
  const drop = () => { held?.remove(); held = null; };
  const copy = () => {
    if (held || still.matches || !cv.width || !cv.height) return;
    held = document.createElement("canvas");
    held.className = "d2-ghost"; held.setAttribute("aria-hidden", "true");
    held.width = cv.width; held.height = cv.height; held.style.backgroundColor = paper;
    held.getContext("2d")?.drawImage(cv, 0, 0);
    cv.after(held);
    const mine = held;
    window.setTimeout(() => { if (held === mine) drop(); }, 1000);     // no change of theme followed: the copy is not kept
  };
  return {
    before: copy,
    change() {
      const old = paper;
      if (still.matches) { paper = getComputedStyle(cv).backgroundColor; drop(); return; }
      // the class goes on before the new colours are computed, so that the change starts a transition (read after, it would be a jump)
      root.classList.add("d2-fading");
      paper = getComputedStyle(cv).backgroundColor;
      if (old === paper) { root.classList.remove("d2-fading"); drop(); return; }
      // a copy made before the change shows the old picture; one made now (a host set the theme) is the best there is
      if (!held) { const keep = paper; paper = old; copy(); paper = keep; }
      const ghost = held;
      held = null;
      if (!ghost) { root.classList.remove("d2-fading"); return; }
      clearTimeout(timer);
      timer = window.setTimeout(() => root.classList.remove("d2-fading"), MS + 100);
      ghost.getBoundingClientRect();
      ghost.style.opacity = "0";
      const done = () => ghost.remove();
      ghost.addEventListener("transitionend", done, { once: true });
      window.setTimeout(done, MS + 200);
    },
  };
}
