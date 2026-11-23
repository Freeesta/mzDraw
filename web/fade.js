// The soft change of theme. The colours of the page (panels, palettes, flyouts, the area behind the canvas) cross-fade in CSS while the root
// carries `d2-fading` (mzdraw.css); the canvas is painted by script, so the old picture, kept as a copy over the new one, fades out. Nothing
// here decides anything about the drawing: it runs when the theme attribute or the system setting changed, never on the first paint (the
// theme is on the root before it, by theme.js), and not at all for a person who asked for less motion.

/** how long the fade lasts, in milliseconds (the same as the transitions of mzdraw.css) */
const MS = 300;

/** @param {HTMLCanvasElement} cv the canvas @returns {() => void} the function to call when the theme changed, before the canvas is painted again */
export function themeFade(cv) {
  const root = document.documentElement, still = matchMedia("(prefers-reduced-motion: reduce)");
  let paper = getComputedStyle(cv).backgroundColor, timer = 0;
  return () => {
    const old = paper;
    if (still.matches) { paper = getComputedStyle(cv).backgroundColor; return; }
    // the class goes on before the new colours are computed, so that the change starts a transition (read after, it would be a jump)
    root.classList.add("d2-fading");
    paper = getComputedStyle(cv).backgroundColor;
    if (old === paper || !cv.width || !cv.height) { root.classList.remove("d2-fading"); return; }
    // the canvas as it was, with the paper it was on, over the canvas painted next; it fades out and goes
    const ghost = document.createElement("canvas");
    ghost.className = "d2-ghost"; ghost.setAttribute("aria-hidden", "true");
    ghost.width = cv.width; ghost.height = cv.height; ghost.style.backgroundColor = old;
    ghost.getContext("2d")?.drawImage(cv, 0, 0);
    cv.after(ghost);
    clearTimeout(timer);
    timer = window.setTimeout(() => root.classList.remove("d2-fading"), MS + 100);
    ghost.getBoundingClientRect();
    ghost.style.opacity = "0";
    const done = () => ghost.remove();
    ghost.addEventListener("transitionend", done, { once: true });
    window.setTimeout(done, MS + 200);
  };
}
