// The About box and the report box. The words of the issue, its form, its URL and its notes are made by the module (`feedback`, `about`:
// crates/mzdraw/src/feedback.rs); this file paints them and opens the link in a new tab. The page sends nothing by itself: the issue is
// a prefilled GitHub form that the person submits there. Two small popovers, no modal dialog: Escape closes one and gives the focus back.

const $ = (/** @type {string} */ id) => /** @type {HTMLElement} */ (document.getElementById(id));

/** @param {{ed: any, t: (key: string, params?: any) => string, focusCanvas: () => void}} h */
export function feedbackBoxes(h) {
  const { ed, t } = h;
  const report = $("d2-report"), about = $("d2-about"), say = (/** @type {string} */ s) => { $("d2-live").textContent = s; };
  const box = { report, about };
  /** the opener that has the focus back when a box closes @type {HTMLElement | null} */
  let from = null;
  const shut = (/** @type {HTMLElement} */ b, back = true) => {
    b.hidden = true;
    for (const o of [$("d2-about-open"), $("d2-report-open")]) o.setAttribute("aria-expanded", "false");
    if (back && from) from.focus({ preventScroll: true });
  };
  /** @param {"report" | "about"} which @param {HTMLElement} opener */
  function show(which, opener) {
    const other = which === "report" ? about : report;
    shut(other, false);
    from = opener;
    const b = box[which];
    b.hidden = false;
    $(which === "report" ? "d2-report-open" : "d2-about-open").setAttribute("aria-expanded", "true");
    if (which === "about") {
      const a = JSON.parse(ed.about());
      $("d2-about-name").textContent = t("draw2.name");
      $("d2-about-version").textContent = t("draw2.about.version", { version: a.version });
      /** @type {HTMLAnchorElement} */ ($("d2-about-repo")).href = a.repository;
      /** @type {HTMLAnchorElement} */ ($("d2-about-licence")).href = a.licence;
    } else $("d2-report-note").textContent = "";
    /** @type {HTMLElement} */ (b.querySelector("input, button, a")).focus({ preventScroll: true });
  }
  const toggle = (/** @type {"report" | "about"} */ which, /** @type {HTMLElement} */ opener) => (box[which].hidden ? show(which, opener) : shut(box[which]));
  $("d2-about-open").addEventListener("click", e => toggle("about", /** @type {HTMLElement} */ (e.currentTarget)));
  $("d2-report-open").addEventListener("click", e => toggle("report", /** @type {HTMLElement} */ (e.currentTarget)));
  $("d2-about-report").addEventListener("click", () => { show("report", $("d2-report-open")); });
  $("d2-about-close").addEventListener("click", () => shut(about));
  $("d2-report-close").addEventListener("click", () => shut(report));
  for (const b of [report, about]) b.addEventListener("keydown", e => { if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); shut(b); } });

  const kind = () => /** @type {HTMLInputElement} */ (report.querySelector("input[name='d2-report-kind']:checked")).value;
  const withLog = () => /** @type {HTMLInputElement} */ ($("d2-report-log")).checked;
  /** the report as the module made it: what the person gave, the log (off unless ticked) and the user agent @returns {any} */
  const make = () => JSON.parse(ed.feedback(kind(), withLog(), navigator.userAgent, navigator.language || ""));

  $("d2-report-go").addEventListener("click", () => {
    const f = make();
    // the notes the module keyed (log too long, no log) are said in words, and the log is offered to the clipboard instead
    $("d2-report-note").textContent = f.notes.map((/** @type {string} */ k) => t(k)).join(" ");
    $("d2-report-copy").hidden = !f.notes.includes("feedback.logTooLong");
    // a link opened by a click, in a new tab, without the opener: the only thing that leaves the page is the person's own submission on GitHub
    const a = document.createElement("a");
    a.href = f.url; a.target = "_blank"; a.rel = "noopener noreferrer";
    document.body.append(a); a.click(); a.remove();
    say(t("draw2.report.opened"));
  });
  $("d2-report-copy").addEventListener("click", async () => {
    try { await navigator.clipboard.writeText(ed.save("log")); $("d2-report-note").textContent = t("draw2.report.copied"); }
    catch { $("d2-report-note").textContent = t("draw2.report.copyFailed"); }
  });
  // changing a choice makes the last answer stale
  report.addEventListener("change", () => { $("d2-report-note").textContent = ""; $("d2-report-copy").hidden = true; });
}
