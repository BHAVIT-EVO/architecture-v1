// ⌘K: search as a centred dialog over the dimmed scene.

import { whenText } from "./works.js";

const el = (tag, props = {}, ...kids) => {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") n.className = v;
    else if (k === "html") n.innerHTML = v;
    else if (k.startsWith("on")) n.addEventListener(k.slice(2), v);
    else if (v !== false && v != null) n.setAttribute(k, v === true ? "" : v);
  }
  for (const kid of kids.flat()) if (kid != null) n.append(kid.nodeType ? kid : document.createTextNode(kid));
  return n;
};

export function openSearch({ works, onPick }) {
  const input = el("input", {
    placeholder: "Search your work", "aria-label": "Search your work",
    autocomplete: "off", spellcheck: "false",
  });
  const results = el("div", { class: "results" });
  const scrim = el("div", { class: "scrim" },
    el("div", { class: "search-panel", role: "dialog", "aria-label": "Search your work" },
      el("label", { class: "field" },
        el("span", { html: '<svg width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><use href="#search"/></svg>' }),
        input,
        el("kbd", {}, "esc")),
      results));

  let matches = [];
  let active = 0;

  function close() {
    scrim.remove();
    document.removeEventListener("keydown", onKey, true);
  }

  function render(query) {
    const needle = query.trim().toLowerCase();
    matches = works.filter((w) =>
      !needle || w.title.toLowerCase().includes(needle) || w.source.toLowerCase().includes(needle));
    active = 0;
    results.replaceChildren();
    if (!matches.length) {
      results.append(el("p", { class: "none" }, "Nothing matches that. Try fewer words."));
      return;
    }
    matches.forEach((w, i) => {
      results.append(el("button", {
        class: "row",
        "aria-current": String(i === active),
        onclick: () => pick(i),
        onmouseenter: () => { active = i; highlight(); },
      },
        el("span", { class: "tile" }, el("span", { html: '<svg width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><use href="#chat"/></svg>' })),
        el("span", {}, el("strong", {}, w.title), el("small", { class: "why" }, w.source)),
        el("time", {}, whenText(w.opened).replace("Last open ", ""))));
    });
  }

  function highlight() {
    [...results.querySelectorAll(".row")].forEach((row, i) =>
      row.setAttribute("aria-current", String(i === active)));
  }

  function pick(i) {
    const work = matches[i];
    close();
    if (work) onPick?.(work);
  }

  function onKey(e) {
    if (e.key === "Escape") { e.preventDefault(); close(); return; }
    if (e.key === "ArrowDown") { e.preventDefault(); active = Math.min(active + 1, matches.length - 1); highlight(); }
    if (e.key === "ArrowUp") { e.preventDefault(); active = Math.max(active - 1, 0); highlight(); }
    if (e.key === "Enter") { e.preventDefault(); pick(active); }
  }

  input.addEventListener("input", (e) => render(e.target.value));
  document.addEventListener("keydown", onKey, true);
  scrim.addEventListener("click", (e) => { if (e.target === scrim) close(); });
  document.body.append(scrim);
  render("");
  input.focus();
  return { close };
}
