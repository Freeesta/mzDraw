// The icons of the page: an original set drawn for this page (no icon pack, nothing traced). Stroke icons on a 24x24 grid, 1.5 px stroke,
// round caps and joins, in currentColor so that they follow the theme; filled parts (the solid wedge) use currentColor too.
// Bonds are drawn by chemical convention along 30 degrees, rings as regular polygons with a flat bottom.

const B = "M5.1 16L18.9 8";                     // the bond axis of every bond icon: 16 px long, 30 degrees up to the right
/** @type {Record<string, string>} the inner markup of each icon */
export const ICONS = {
  select: '<rect x="3.5" y="3.5" width="12" height="12" rx="1" stroke-dasharray="2 2.6"/><path d="M13 13l7.5 2.7-3.2 1.4-1.4 3.2z" fill="currentColor"/>',
  "move-here": '<rect x="3.5" y="3.5" width="9" height="9" rx="1" stroke-dasharray="2 2.6"/><path d="M13 11l7.5 2.7-3.2 1.4-1.4 3.2z" fill="currentColor"/><path d="M13.5 5.5h6M17.5 3.5l2 2-2 2"/>',
  erase: '<path d="M14 4.5l5.5 5.5-9.5 9.5H7L3.5 16z"/><path d="M8.5 10l5.5 5.5M12 19.5h8.5"/>',
  "bond:1": `<path d="${B}"/>`,
  "bond:2": '<path d="M4.2 14.5L18.1 6.5M5.9 17.5L19.8 9.5"/>',
  "bond:3": '<path d="M3.8 13.8L17.7 5.8M5.1 16L18.9 8M6.3 18.2L20.2 10.2"/>',
  "bond:up": '<path d="M5.1 16L17.7 5.8L20.2 10.2z" fill="currentColor"/>',
  "bond:down": '<path d="M7.6 13.9l.6 1M10.1 11.9l1.1 1.9M12.6 9.8l1.6 2.8M15.1 7.8l2.1 3.6M17.6 5.8l2.6 4.4"/>',
  "bond:wavy": '<path d="M5.1 16Q5.9 12.1 9.7 13.3T14.3 10.7T18.9 8"/>',
  "bond:dash": '<path d="M5.1 16L7.4 14.7M9.4 13.5L11.7 12.2M13.7 11L16 9.6M18 8.5L19.4 7.7"/>',
  "bond:bdash": '<path d="M5.1 16L7.4 14.7M9.4 13.5L11.7 12.2M13.7 11L16 9.6M18 8.5L19.4 7.7" stroke-width="3.4"/>',
  "bond:hash": '<path d="M5.1 16L19.4 7.7M7.4 16.1L8.6 13.9M10.5 14.3L11.7 12.1M13.6 12.5L14.8 10.3M16.7 10.7L17.9 8.5"/>',
  "ring:3": '<path d="M4.2 18.8L12 5.3l7.8 13.4z"/>',
  "ring:4": '<path d="M6 18V6h12v12z"/>',
  "ring:5": '<path d="M7 19.7L3.9 10.2 12 4.3l8.1 5.9-3.1 9.5z"/>',
  "ring:6": '<path d="M7.8 19.4L3.5 12l4.2-7.4h8.6l4.2 7.4-4.3 7.4z"/>',
  "ring:6a": '<path d="M7.8 19.4L3.5 12l4.2-7.4h8.6l4.2 7.4-4.3 7.4z"/><path d="M8.4 16.6L6.3 12.8M9.8 6.6h4.4M17.7 12.8l-2.1 3.8"/>',
  "ring:7": '<path d="M8.3 20.1l-4.6-5.8 1.7-7.2L12 3.9l6.6 3.2 1.7 7.2-4.6 5.8z"/>',
  "ring:8": '<path d="M8.7 19.9l-4.6-4.6V8.7l4.6-4.6h6.6l4.6 4.6v6.6l-4.6 4.6z"/>',
  elements: '<rect x="3.5" y="4.5" width="4" height="4" rx=".8"/><rect x="16.5" y="4.5" width="4" height="4" rx=".8"/><rect x="3.5" y="10.5" width="17" height="4" rx=".8"/><path d="M7.75 10.5v4M12 10.5v4M16.25 10.5v4"/><rect x="7.5" y="16.5" width="9" height="3" rx=".8"/>',
  "charge:+": '<circle cx="12" cy="12" r="8"/><path d="M12 8.5v7M8.5 12h7"/>',
  "charge:-": '<circle cx="12" cy="12" r="8"/><path d="M8.5 12h7"/>',
  label: '<path d="M5.5 19V5h5a4 4 0 0 1 0 8h-5M10 13l4.5 6"/><path d="M17.5 14.5l1.5-1v6"/>',
  arrow: '<path d="M3.5 12h16.5M15 7l5 5-5 5"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  "scheme-row": '<path d="M2.5 12h4M4.5 10v4M9 12h6M12.5 9.5L15 12l-2.5 2.5M19 12h2.5"/>',
  "pathway-label": '<path d="M3 15h18M17 11.5l4 3.5-4 3.5"/><path d="M6 9.5h7M9.5 6.5v6"/>',
  "pathway-tree": '<rect x="2.5" y="9.5" width="4" height="5" rx="1"/><rect x="17.5" y="3.5" width="4" height="4" rx="1"/><rect x="17.5" y="16.5" width="4" height="4" rx="1"/><path d="M6.5 12h4v-6h7M10.5 12v6h7"/>',
  text: '<path d="M5 7V4.5h14V7M12 4.5v15M9 19.5h6"/>',
  undo: '<path d="M9 14L4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11"/>',
  redo: '<path d="M15 14l5-5-5-5"/><path d="M20 9H9.5a5.5 5.5 0 0 0 0 11H13"/>',
  cut: '<circle cx="7" cy="17.5" r="2.5"/><circle cx="17" cy="17.5" r="2.5"/><path d="M8.6 15.6L17.5 4M15.4 15.6L6.5 4"/>',
  copy: '<rect x="8.5" y="8.5" width="11.5" height="11.5" rx="1.5"/><path d="M15.5 8.5V5.5A1.5 1.5 0 0 0 14 4H5.5A1.5 1.5 0 0 0 4 5.5V14a1.5 1.5 0 0 0 1.5 1.5h3"/>',
  paste: '<path d="M9 5H6.5A1.5 1.5 0 0 0 5 6.5v13A1.5 1.5 0 0 0 6.5 21h11a1.5 1.5 0 0 0 1.5-1.5v-13A1.5 1.5 0 0 0 17.5 5H15"/><rect x="9" y="3" width="6" height="4" rx="1"/>',
  duplicate: '<rect x="8.5" y="8.5" width="11.5" height="11.5" rx="1.5"/><path d="M15.5 8.5V5.5A1.5 1.5 0 0 0 14 4H5.5A1.5 1.5 0 0 0 4 5.5V14a1.5 1.5 0 0 0 1.5 1.5h3M14.25 11.5v5.5M11.5 14.25H17"/>',
  "rotate-l": '<path d="M6.7 6.7a7.5 7.5 0 1 0 10.6 0"/><path d="M20.8 7l-3.5-.3-.3 3.5"/>',
  "rotate-r": '<path d="M17.3 6.7a7.5 7.5 0 1 1-10.6 0"/><path d="M3.2 7l3.5-.3.3 3.5"/>',
  "flip-h": '<path d="M12 3v18" stroke-dasharray="2 2.5"/><path d="M9 7L3.5 17H9zM15 7l5.5 10H15z"/>',
  "flip-v": '<path d="M3 12h18" stroke-dasharray="2 2.5"/><path d="M7 9l10-5.5V9zM7 15l10 5.5V15z"/>',
  "align:left": '<path d="M4 3.5v17"/><rect x="7" y="6" width="12" height="4" rx=".8"/><rect x="7" y="14" width="7" height="4" rx=".8"/>',
  "align:centre": '<path d="M12 3.5v17"/><rect x="5" y="6" width="14" height="4" rx=".8"/><rect x="8.5" y="14" width="7" height="4" rx=".8"/>',
  "align:right": '<path d="M20 3.5v17"/><rect x="5" y="6" width="12" height="4" rx=".8"/><rect x="10" y="14" width="7" height="4" rx=".8"/>',
  "align:top": '<path d="M3.5 4h17"/><rect x="6" y="7" width="4" height="12" rx=".8"/><rect x="14" y="7" width="4" height="7" rx=".8"/>',
  "align:middle": '<path d="M3.5 12h17"/><rect x="6" y="5" width="4" height="14" rx=".8"/><rect x="14" y="8.5" width="4" height="7" rx=".8"/>',
  "align:bottom": '<path d="M3.5 20h17"/><rect x="6" y="5" width="4" height="12" rx=".8"/><rect x="14" y="10" width="4" height="7" rx=".8"/>',
  "distribute:h": '<path d="M4 4v16M20 4v16"/><rect x="9.5" y="7" width="5" height="10" rx=".8"/>',
  "distribute:v": '<path d="M4 4h16M4 20h16"/><rect x="7" y="9.5" width="10" height="5" rx=".8"/>',
  "scheme-orient": '<path d="M3 17l4.5-9 4.5 9z"/><path d="M15 8.5a5 5 0 0 1 5 5" /><path d="M20.8 10.6l-.8 3.3-3.2-.9"/>',
  "zoom-in": '<circle cx="10.5" cy="10.5" r="6.5"/><path d="M15.3 15.3l5.2 5.2M7.5 10.5h6M10.5 7.5v6"/>',
  "zoom-out": '<circle cx="10.5" cy="10.5" r="6.5"/><path d="M15.3 15.3l5.2 5.2M7.5 10.5h6"/>',
  fit: '<path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/><rect x="8.5" y="8.5" width="7" height="7" rx="1"/>',
  open: '<path d="M3.5 18.5v-13h6l2 2.5h9v10.5z"/>',
  save: '<path d="M12 3.5v11M7.5 10l4.5 4.5 4.5-4.5M4 15.5v4h16v-4"/>',
  clear: '<path d="M6 3.5h8l4 4v13H6z"/><path d="M14 3.5v4h4"/>',
  panel: '<rect x="3.5" y="4.5" width="17" height="15" rx="1.5"/><path d="M14.5 4.5v15M16.75 8.5h1.5M16.75 11h1.5"/>',
  more: '<circle cx="6" cy="12" r="1.3" fill="currentColor"/><circle cx="12" cy="12" r="1.3" fill="currentColor"/><circle cx="18" cy="12" r="1.3" fill="currentColor"/>',
  colour: '<path d="M12 3.5c4.7 5 6.5 7.7 6.5 10.2a6.5 6.5 0 0 1-13 0c0-2.5 1.8-5.2 6.5-10.2z"/><path d="M9 14.5a3 3 0 0 0 3 3"/>',
  formats: '<path d="M6 3.5h8l4 4v13H6z"/><path d="M14 3.5v4h4M9 11.5h6M9 14.5h6M9 17.5h3.5"/>',
  palettes: '<rect x="3.5" y="3.5" width="7" height="7" rx="1.2"/><rect x="13.5" y="3.5" width="7" height="7" rx="1.2"/><rect x="3.5" y="13.5" width="7" height="7" rx="1.2"/><path d="M13.5 17h7M17 13.5v7"/>',
  gear: '<circle cx="12" cy="12" r="3"/><path d="M12 3.2l1.6 2.3 2.7-.6.9 2.6 2.6.9-.6 2.7 2.3 1.6-2.3 1.6.6 2.7-2.6.9-.9 2.6-2.7-.6L12 20.8l-1.6-2.3-2.7.6-.9-2.6-2.6-.9.6-2.7L2.5 12l2.3-1.6-.6-2.7 2.6-.9.9-2.6 2.7.6z"/>',
  all: '<rect x="3.5" y="3.5" width="17" height="17" rx="1" stroke-dasharray="2 2.6"/><path d="M8 12l3 3 5-6"/>',
};

const NS = "http://www.w3.org/2000/svg";
/** an icon as an `svg` element (hidden from assistive technology: the button carries the name) @param {string} name */
export function icon(name) {
  const svg = document.createElementNS(NS, "svg");
  for (const [k, v] of [["viewBox", "0 0 24 24"], ["width", "24"], ["height", "24"], ["fill", "none"], ["stroke", "currentColor"], ["stroke-width", "1.5"],
    ["stroke-linecap", "round"], ["stroke-linejoin", "round"], ["aria-hidden", "true"], ["focusable", "false"], ["class", "ico"]]) svg.setAttribute(k, v);
  svg.innerHTML = ICONS[name] || "";
  svg.dataset.icon = name;
  return svg;
}
