// Dev-only gallery: tokens, every component and every state, the scene in its
// four modes. Nothing here ships; it exists to prove the foundation.

import { createScene, hitRects, MODES } from "./scene/index.js";
import { sampleWorks } from "./scene/sprites.js";

const root = document.querySelector("#gal");

const h = (tag, props = {}, ...kids) => {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") node.className = v;
    else if (k === "html") node.innerHTML = v;
    else if (k.startsWith("on")) node.addEventListener(k.slice(2), v);
    else if (v !== false && v != null) node.setAttribute(k, v === true ? "" : v);
  }
  for (const kid of kids.flat()) if (kid != null) node.append(kid.nodeType ? kid : document.createTextNode(kid));
  return node;
};

const section = (title, note, ...kids) =>
  h("section", { id: title.toLowerCase() }, h("h2", {}, title), note ? h("p", { class: "note" }, note) : null, ...kids);

const icon = (id) => h("span", { html: `<svg class="i" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><use href="#${id}"/></svg>` });

const state = (label, ...nodes) => h("div", { class: "state" }, h("span", {}, label), ...nodes);

// ── tokens ──────────────────────────────────────────────────────────────
const COLORS = [
  ["--ink", "#06120e"], ["--panel", "rgba(9,21,17,.9)"], ["--line", "rgba(240,234,216,.13)"],
  ["--text", "#f0ead8"], ["--muted", "#a9b8ab"], ["--faint", "#7f9283"],
  ["--accent", "#8fe0a8"], ["--on-accent", "#06180f"],
];

root.append(
  section("Tokens", "DESIGN_LAW §2. Eight colours, two faces, four radii, three control heights.",
    h("div", { class: "swatches" },
      COLORS.map(([name, value]) =>
        h("div", { class: "swatch" },
          h("div", { class: "chip-color", style: `background:var(${name})` }),
          h("code", {}, `${name} ${value}`)))),
    h("h3", {}, "Type"),
    h("div", { class: "specimen" },
      h("div", { class: "row-line" }, h("span", { class: "t-display" }, "Pick up where you left off."), h("span", {}, "Instrument Serif 56/1.02 · −.015em")),
      h("div", { class: "row-line" }, h("span", { class: "t-card-title" }, "Frontend prompt for product redesign"), h("span", {}, "Instrument Serif 31/1.08")),
      h("div", { class: "row-line" }, h("span", { class: "t-body" }, "Everything you were working on, in the order you left it."), h("span", {}, "Instrument Sans 15/1.45 400")),
      h("div", { class: "row-line" }, h("span", { class: "t-small" }, "Listening for “Hey Evo”"), h("span", {}, "Instrument Sans 500")),
      h("div", { class: "row-line" }, h("strong", { class: "t-body" }, "Continue"), h("span", {}, "Instrument Sans 600"))),
    h("h3", {}, "Radius and controls"),
    h("div", { class: "matrix" },
      state("card 24", h("div", { style: "width:88px;height:56px;border-radius:24px;border:1px solid var(--line);background:var(--panel)" })),
      state("menu 16", h("div", { style: "width:88px;height:56px;border-radius:16px;border:1px solid var(--line);background:#10241c" })),
      state("row 14", h("div", { style: "width:88px;height:56px;border-radius:14px;border:1px solid var(--line);background:rgba(240,234,216,.07)" })),
      state("pill 99", h("div", { style: "width:88px;height:56px;border-radius:99px;border:1px solid var(--line);background:var(--accent)" })),
      state("primary 46", h("button", { class: "btn-primary" }, "Continue")),
      state("chip 34", h("span", { class: "chip" }, icon("i-lock"), "Stays on this Mac")),
      state("icon 36", h("button", { class: "icon-button", "aria-label": "Settings" }, icon("i-gear")))),
  ),
);

// ── scene ───────────────────────────────────────────────────────────────
const works = sampleWorks();
const sceneEl = h("div", { class: "scene", "data-mode": "home", tabindex: "0", "aria-label": "Home scene" });
const canvas = h("canvas", { "aria-hidden": "true" });
const veil = h("div", { class: "veil" });
sceneEl.append(canvas, veil);

const status = document.querySelector("#scene-status");
const scene = createScene({ canvas, works, mode: "home" });

let selected = 0;
const hits = [];

function placeHits() {
  const rects = hitRects(works, sceneEl.clientWidth, sceneEl.clientHeight);
  hits.forEach((b, i) => {
    const r = rects[i];
    Object.assign(b.style, { left: `${r.left}px`, top: `${r.top}px`, width: `${r.width}px`, height: `${r.height}px` });
  });
  const crt = rects.crt;
  Object.assign(crtHit.style, { left: `${crt.left}px`, top: `${crt.top}px`, width: `${crt.width}px`, height: `${crt.height}px`, borderRadius: "10px" });
  hits.forEach((b) => b.hidden = sceneEl.dataset.mode !== "home");
}

hits.push(...works.map((w, i) => {
  const b = h("button", {
    class: "hit",
    "aria-label": w.title,
    onmouseenter: () => { selected = i; scene.select(i); },
    onfocus: () => { selected = i; scene.select(i); },
    onclick: () => { selected = i; scene.select(i); },
  }, h("span", { class: "tag" }, w.title));
  sceneEl.append(b);
  return b;
}));

const crtHit = h("button", { class: "hit", "aria-label": "Talk to Evo" });
crtHit.addEventListener("click", async () => {
  scene.setListening(true);
  await new Promise((r) => setTimeout(r, 1800));
  scene.setListening(false);
});
sceneEl.append(crtHit);

sceneEl.addEventListener("keydown", (e) => {
  if (e.key === "ArrowRight" || e.key === "ArrowDown") { e.preventDefault(); selected = (selected + 1) % works.length; scene.select(selected); hits[selected].focus(); }
  if (e.key === "ArrowLeft" || e.key === "ArrowUp") { e.preventDefault(); selected = (selected - 1 + works.length) % works.length; scene.select(selected); hits[selected].focus(); }
});

new ResizeObserver(placeHits).observe(sceneEl);
window.addEventListener("resize", placeHits);
requestAnimationFrame(placeHits);

const modeButtons = MODES.map((m) =>
  h("button", {
    class: "chip",
    "aria-pressed": m === "home" ? "true" : "false",
    onclick: (e) => {
      scene.setMode(m);
      sceneEl.dataset.mode = m;
      for (const b of modeButtons) b.setAttribute("aria-pressed", b.dataset.mode === m ? "true" : "false");
      placeHits();
    },
  }, m));

modeButtons.forEach((b, i) => (b.dataset.mode = MODES[i]));

root.append(
  section("Scene", "Papered from the mockup's canvas: 320×180, seed 11, cover-cropped, ~12fps. The loop stops whenever this tab is hidden or the window loses focus.",
    h("div", { class: "scene-controls" }, modeButtons,
      h("button", { class: "chip", onclick: () => { const el = document.querySelector(".scene"); el.style.height = el.style.height === "240px" ? "" : "240px"; placeHits(); } }, "Taller")),
    h("div", { class: "scene-demo" }, sceneEl),
    h("p", { class: "note" }, "Click an object or the CRT. Arrow keys move the selection. Hovering an object shows its title.")),
);

setInterval(() => {
  status.textContent = `scene — ${sceneEl.dataset.mode} · ${scene.isRunning() ? "running" : "paused"}`;
}, 400);

// ── components ──────────────────────────────────────────────────────────
const componentsSection = section("Components", "Every control, with default, hover, focus, active, disabled, loading and error. Hover and press move; Tab shows the 2px accent ring at 3px offset.",
    h("h3", {}, "Panel card"),
    h("div", { class: "matrix" },
      state("default", h("div", { class: "panel-card", style: "width:280px" }, h("p", { class: "faint t-caption" }, "Last open an hour ago"), h("h3", { class: "t-card-title" }, "Frontend prompt for product redesign"))),
      state("error", h("div", { class: "panel-card is-error", style: "width:280px" }, h("p", { class: "t-body" }, "Evo couldn't read this work."))),
      state("disabled", h("div", { class: "panel-card", "aria-disabled": "true", style: "width:280px" }, h("p", { class: "t-body" }, "Unavailable")))),
    h("h3", {}, "Primary and ghost buttons"),
    h("div", { class: "matrix" },
      state("default", h("button", { class: "btn-primary" }, "Continue")),
      state("disabled", h("button", { class: "btn-primary", disabled: true }, "Continue")),
      state("loading", h("button", { class: "btn-primary is-loading" }, "Continue")),
      state("ghost", h("button", { class: "btn-ghost" }, "Not now")),
      state("ghost disabled", h("button", { class: "btn-ghost", disabled: true }, "Not now")),
      state("ghost loading", h("button", { class: "btn-ghost is-loading" }, "Not now")),
      state("ghost error", h("button", { class: "btn-ghost is-error" }, "Try again"))),
    h("h3", {}, "Icon button"),
    h("div", { class: "matrix" },
      state("default", h("button", { class: "icon-button", "aria-label": "Settings" }, icon("i-gear"))),
      state("pressed", h("button", { class: "icon-button", "aria-pressed": "true", "aria-label": "Settings" }, icon("i-gear"))),
      state("loading", h("button", { class: "icon-button is-loading", "aria-label": "Loading" }, icon("i-gear"))),
      state("disabled", h("button", { class: "icon-button", disabled: true, "aria-label": "Settings" }, icon("i-gear")))),
    h("h3", {}, "Chips and live status"),
    h("div", { class: "matrix" },
      state("chip", h("span", { class: "chip" }, icon("i-lock"), "Stays on this Mac")),
      state("pressed", h("button", { class: "chip", "aria-pressed": "true" }, "Memory")),
      state("disabled", h("span", { class: "chip", disabled: true }, "Unavailable")),
      state("error", h("span", { class: "chip is-error" }, "Microphone off")),
      state("live", h("span", { class: "chip chip-live" }, h("span", { class: "live", "aria-hidden": "true" }, h("i"), h("i"), h("i"), h("i")), "Listening for “Hey Evo”")),
      state("paused", h("span", { class: "chip chip-live", "data-state": "paused" }, h("span", { class: "live", "aria-hidden": "true" }, h("i"), h("i"), h("i"), h("i")), "Not listening"))),
    h("h3", {}, "Segmented control"),
    h("div", { class: "matrix" },
      state("scene", h("div", { class: "segmented", role: "group", "aria-label": "View" }, h("button", { "aria-pressed": "true" }, "Scene"), h("button", { "aria-pressed": "false" }, "List"))),
      state("disabled", h("div", { class: "segmented", role: "group", "aria-label": "View" }, h("button", { "aria-pressed": "true", disabled: true }, "Scene"), h("button", { "aria-pressed": "false", disabled: true }, "List")))),
    h("h3", {}, "Text field"),
    h("div", { class: "matrix" },
      state("default", h("label", { class: "field", style: "width:260px" }, icon("i-search"), h("input", { placeholder: "Search your work", "aria-label": "Search your work" }), h("kbd", {}, "⌘K"))),
      state("filled", h("label", { class: "field", style: "width:260px" }, icon("i-search"), h("input", { value: "redesign", "aria-label": "Search your work" }), h("kbd", {}, "⌘K"))),
      state("error", h("label", { class: "field", "data-state": "error", style: "width:260px" }, icon("i-search"), h("input", { placeholder: "Search your work", "aria-label": "Search your work" }))),
      state("disabled", h("label", { class: "field", "data-state": "disabled", style: "width:260px" }, icon("i-search"), h("input", { placeholder: "Search your work", disabled: true, "aria-label": "Search your work" })))),
    h("h3", {}, "Toggle and select"),
    h("div", { class: "matrix" },
      state("off", h("button", { class: "switch", role: "switch", "aria-checked": "false" }, h("span", { class: "track" }), "Listen for “Hey Evo”")),
      state("on", h("button", { class: "switch", role: "switch", "aria-checked": "true" }, h("span", { class: "track" }), "Listen for “Hey Evo”")),
      state("disabled", h("button", { class: "switch", role: "switch", "aria-checked": "false", disabled: true }, h("span", { class: "track" }), "Listen for “Hey Evo”")),
      state("select", h("button", { class: "select" }, "Dusk", h("span", { class: "faint" }, "▾"))),
      state("open", h("button", { class: "select", "aria-expanded": "true" }, "Dusk", h("span", { class: "faint" }, "▾")))),
    h("h3", {}, "Menu"),
    h("div", { class: "matrix" },
      state("open", h("div", { class: "menu" },
        h("button", { class: "menu-item", role: "menuitem" }, "Rename"),
        h("button", { class: "menu-item", role: "menuitem" }, "Restore earlier version"),
        h("button", { class: "menu-item", role: "menuitem" }, "This isn’t work"),
        h("div", { class: "menu-sep" }),
        h("button", { class: "menu-item", role: "menuitem" }, "Hide from Home"),
        h("button", { class: "menu-item", role: "menuitem", disabled: true }, "Merge with…"))),
      state("error", h("div", { class: "menu" }, h("button", { class: "menu-item is-error" }, "Couldn’t rename. Try again.")))),
    h("h3", {}, "Row"),
    h("div", { class: "grid2" },
      h("div", {},
        state("default", h("button", { class: "row" }, h("span", { class: "tile" }, icon("i-chat")), h("span", {}, h("strong", {}, "Frontend prompt for product redesign"), h("small", {}, "A Claude chat in Chrome")), h("time", {}, "1h"))),
        state("current", h("button", { class: "row", "aria-current": "true" }, h("span", { class: "tile" }, icon("i-play")), h("span", {}, h("strong", {}, "Watch The Family Man"), h("small", {}, "Chrome")), h("time", {}, "7d"))),
        state("loading", h("button", { class: "row is-loading" }, h("span", { class: "tile" }, icon("i-dir")), h("span", {}, h("strong", {}, "Reading work…"), h("small", {}, "One moment")), h("time", {}, "")))),
      h("div", {},
        state("error", h("button", { class: "row is-error" }, h("span", { class: "tile" }, icon("i-dir")), h("span", {}, h("strong", {}, "Couldn’t open this work"), h("small", {}, "Try again")), h("time", {}, ""))),
        state("disabled", h("button", { class: "row", disabled: true }, h("span", { class: "tile" }, icon("i-chat")), h("span", {}, h("strong", {}, "Not available"), h("small", {}, "Nothing can be reopened")), h("time", {}, ""))),
        state("tag", h("div", { style: "display:flex;gap:8px;align-items:center" }, h("span", { class: "tag" }, "Example"), h("span", { class: "tag is-error" }, "Needs permission"))))),
    h("h3", {}, "Toast"),
    h("div", { class: "matrix" },
      state("default", h("div", { class: "toast" }, h("span", { class: "dot" }), "Renamed.")),
      state("loading", h("div", { class: "toast is-loading" }, h("span", { class: "dot" }), "Opening your windows…")),
      state("error", h("div", { class: "toast is-error" }, h("span", { class: "dot" }), "Evo couldn’t open that work."))),
    h("h3", {}, "Dialog"),
    h("div", { class: "matrix" },
      state("default", h("div", { class: "dialog" }, h("h2", {}, "Leave this work?"), h("p", {}, "Your windows go back exactly where they were."), h("div", { class: "actions" }, h("button", { class: "btn-ghost" }, "Stay"), h("button", { class: "btn-primary" }, "Leave")))),
      state("error", h("div", { class: "dialog is-error" }, h("h2", {}, "Evo couldn’t reopen this"), h("p", {}, "The page it needs isn’t open any more."), h("div", { class: "actions" }, h("button", { class: "btn-ghost" }, "Dismiss"), h("button", { class: "btn-primary" }, "Try again"))))),
    h("h3", {}, "Notch"),
    h("div", { class: "matrix" },
      state("resting", h("div", { style: "width:200px;background:var(--ink);border-radius:0 0 18px 18px" }, h("div", { class: "notch" }))),
      state("listening", h("div", { style: "width:520px;background:var(--ink);border-radius:0 0 28px 28px" }, h("div", { class: "notch", "data-on": true }, h("div", { class: "w", "aria-hidden": "true" }, h("i"), h("i"), h("i"), h("i"), h("i"), h("i"), h("i")), h("div", { class: "tx" }, h("b", {}, "Listening"))))),
      state("done", h("div", { style: "width:520px;background:var(--ink);border-radius:0 0 28px 28px" }, h("div", { class: "notch", "data-on": true }, h("div", { class: "tx" }, h("b", {}, "Done"))))),
      state("needs you", h("div", { style: "width:520px;background:var(--ink);border-radius:0 0 28px 28px" }, h("div", { class: "notch", "data-on": true }, h("div", { class: "tx" }, h("b", {}, "Two buttons")), h("div", { class: "actions" }, h("span", { class: "chip" }, "Continue"), h("span", { class: "chip" }, "Not now")))))),
    h("h3", {}, "Empty state"),
    h("div", { class: "empty" }, h("strong", {}, "Nothing here yet."), h("span", {}, "Start something and Evo will keep track of it."), h("button", {}, "See activity and permissions")),
);

root.append(componentsSection);

// Deep links for the gallery itself: gallery.html#components
if (location.hash) document.querySelector(location.hash)?.scrollIntoView({ block: "start" });
