// Help and Diagnostics, both in the world: the sheet sits over the quiet scene.

import { HELP, SHORTCUTS, DIAG } from "./copy.js";
import { api } from "./bridge.js";
import { iconSvg, iconSvgPath } from "./card-view.js";

const el = (tag, props = {}, ...kids) => {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") n.className = v;
    else if (k.startsWith("on")) n.addEventListener(k.slice(2), v);
    else if (v !== false && v != null) n.setAttribute(k, v === true ? "" : v);
  }
  for (const kid of kids.flat()) if (kid != null) n.append(kid.nodeType ? kid : document.createTextNode(kid));
  return n;
};

const closeButton = (shut) =>
  el("button", { class: "icon-button", "aria-label": "Close", onclick: shut },
    iconSvgPath(document, "M5 5l8 8M13 5l-8 8"));

function sheet(title, onClose) {
  const body = el("div", { class: "body" });
  const scrim = el("div", { class: "scrim" });
  const shut = () => { scrim.remove(); document.removeEventListener("keydown", onKey, true); onClose?.(); };
  const onKey = (e) => { if (e.key === "Escape") shut(); };
  scrim.append(el("div", { class: "sheet", role: "dialog", "aria-label": title },
    el("header", {}, el("h2", {}, title), el("span", { class: "sp" }), closeButton(shut)), body));
  scrim.addEventListener("click", (e) => { if (e.target === scrim) shut(); });
  document.addEventListener("keydown", onKey, true);
  document.body.append(scrim);
  return { body, shut, scrim };
}

const row = (title, hint, side) =>
  el("div", { class: "row-line" }, el("span", {}, el("strong", {}, title), hint ? el("small", {}, hint) : null),
    el("span", { class: "side" }, side));

export function openHelp({ onReplayTour, onDiagnostics, onClose }) {
  const { body } = sheet(HELP.title, onClose);

  const search = el("input", { placeholder: HELP.search, "aria-label": HELP.search, class: "help-search" });
  const list = el("div", { class: "help-list" });
  body.append(el("label", { class: "field help-field" },
    iconSvg(document, "search"), search), list);

  function render(q = "") {
    const needle = q.trim().toLowerCase();
    const entries = HELP.guide.filter(([q1, a]) => !needle || q1.toLowerCase().includes(needle) || a.toLowerCase().includes(needle));
    list.replaceChildren();
    if (!entries.length) {
      list.append(el("p", { class: "none" }, "Nothing matches that. Try fewer words."));
    }
    for (const [question, answer] of entries) {
      const detail = el("p", { class: "answer" }, answer);
      detail.hidden = true;
      const toggle = el("button", { class: "help-q", "aria-expanded": "false", onclick: () => {
        const open = toggle.getAttribute("aria-expanded") === "true";
        toggle.setAttribute("aria-expanded", String(!open));
        detail.hidden = open;
      } }, question);
      list.append(toggle, detail);
    }
  }
  search.addEventListener("input", (e) => render(e.target.value));
  render();

  body.append(row(HELP.tour.label, HELP.tour.hint,
    el("button", { class: "btn-ghost", style: "height:34px", onclick: onReplayTour }, "Replay")));
  body.append(row(HELP.shortcuts.label, HELP.shortcuts.hint,
    el("button", { class: "btn-ghost", style: "height:34px", onclick: openShortcuts }, "Open")));
  for (const [version, line] of HELP.whatsNew.items) {
    body.append(row(version, line, el("span", { class: "faint", style: "font-size:12.5px" }, "")));
  }
  body.append(row(HELP.report.label, HELP.report.hint,
    el("button", { class: "btn-ghost", style: "height:34px", onclick: openReport }, "Open")));
  body.append(row("Diagnostics", "Microphone, permissions, voice and storage.",
    el("button", { class: "btn-ghost", style: "height:34px", onclick: onDiagnostics }, "Open")));
}

export function openShortcuts() {
  const { body } = sheet("Keyboard shortcuts");
  for (const [keys, what] of SHORTCUTS) {
    body.append(row(what, null, el("kbd", { class: "kbd-chip" }, keys)));
  }
}

export function openReport() {
  const { body, scrim } = sheet(HELP.reportTitle);
  const events = JSON.parse(localStorage.getItem("evo.activity") || "[]").slice(0, 5);
  const onboard = JSON.parse(localStorage.getItem("evo.onboard") || "null");
  const payload = {
    app: "Evo 0.1.0",
    system: navigator.platform,
    microphone: onboard?.mic ?? "not asked",
    voice: { engine: "on this Mac", wake: onboard?.wake ?? "untested" },
    activity: events.map((e) => `${e.what} (${e.why})`),
    note: "",
  };
  body.append(el("p", { class: "report-lead" }, HELP.reportIntro));
  body.append(el("pre", { class: "report-preview" }, JSON.stringify(payload, null, 2)));

  let approved = false;
  const send = el("button", { class: "btn-primary", disabled: true, onclick: () => {
    localStorage.setItem("evo.report", JSON.stringify({ ...payload, at: Date.now() }));
    scrim.querySelector(".sheet").replaceChildren(
      el("header", {}, el("h2", {}, HELP.reportTitle), el("span", { class: "sp" }), closeButton(() => scrim.remove())),
      el("div", { class: "body" }, el("p", { class: "report-lead" }, HELP.saved)));
  } }, HELP.send);

  body.append(el("div", { class: "row-line" },
    el("span", {}, el("strong", {}, HELP.approve), el("small", {}, "The preview above is the whole report.")),
    el("span", { class: "side" }, el("button", {
      class: "switch", role: "switch", "aria-checked": "false",
      onclick: (e) => {
        approved = !approved;
        e.currentTarget.setAttribute("aria-checked", String(approved));
        send.disabled = !approved;
      },
    }, el("span", { class: "track" })))));
  body.append(el("div", { class: "actions-row" },
    el("button", { class: "btn-ghost", onclick: () => navigator.clipboard?.writeText(JSON.stringify(payload, null, 2)) }, HELP.copy),
    send));
}

const built = (value) => {
  const text = String(value ?? "");
  if (!/^\d+$/.test(text)) return text || "unknown";
  return new Date(Number(text) * 1000).toLocaleString();
};

export async function openDiagnostics({ works, onClose }) {
  const { body } = sheet(DIAG.title, onClose);
  const bytes = (n) => {
    if (!n) return "0 KB";
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`;
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  };

  // Every row below is a measurement or a stored fact from Rust. There is no
  // browser-only startup, heap or frame-rate row: those measured the preview,
  // not the app (docs/BRIDGE.md, get_diagnostics).
  body.append(el("p", { class: "why", style: "color:var(--muted);font-size:12.5px;padding:8px 0" }, "Reading…"));
  let facts = null;
  try {
    facts = await api.diagnostics();
  } catch {
    facts = null;
  }
  body.lastChild.remove();
  if (!facts) {
    body.append(el("p", { class: "why", style: "color:var(--muted);font-size:12.5px;padding:8px 0" },
      "The app didn't answer. Diagnostics need the Evo app, not a preview."));
    return;
  }

  body.append(row("Capture", facts.daemon.running
      ? "The daemon is running and watching what you allow."
      : "The daemon is not running; nothing is being captured.",
    el("span", { class: "side" }, el("span", { class: "faint", style: "font-size:12.5px" }, facts.daemon.label))));
  body.append(row("Ledger", "The searchable history.",
    el("span", { class: "faint", style: "font-size:12.5px" },
      facts.ledger.present ? bytes(facts.ledger.bytes) : "not created yet")));
  body.append(row("Log", `${facts.log.files} files, ${facts.log.crashes} crash records.`,
    el("span", { class: "faint", style: "font-size:12.5px" }, `${facts.log.path}, ${bytes(facts.log.bytes)}`)));
  body.append(row("Storage", "Everything Evo keeps on this Mac.",
    el("span", { class: "faint", style: "font-size:12.5px" }, `${facts.storage.path}, ${bytes(facts.storage.bytes)}`)));
  body.append(row("Build", `Version ${facts.version}.`,
    el("span", { class: "faint", style: "font-size:12.5px" }, `Built ${built(facts.build_time)}`)));

  const micRow = row(DIAG.mic, "Only open while Evo listens.",
    el("span", { class: "side" }, el("span", { class: "diag-state", "data-state": "idle" }, "Not tested"),
      el("button", { class: "btn-ghost", style: "height:34px", onclick: test }, DIAG.test)));
  body.append(micRow);

  async function test(e) {
    const state = body.querySelector(".diag-state");
    const button = e.currentTarget;
    button.disabled = true;
    state.dataset.state = "testing";
    state.textContent = DIAG.testing;
    let stream;
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const ctx = new AudioContext();
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 1024;
      ctx.createMediaStreamSource(stream).connect(analyser);
      const data = new Float32Array(analyser.fftSize);
      let peak = 0;
      for (let i = 0; i < 20; i++) {
        await new Promise((r) => setTimeout(r, 100));
        analyser.getFloatTimeDomainData(data);
        peak = Math.max(peak, ...data.map(Math.abs));
      }
      ctx.close();
      state.dataset.state = peak > 0.01 ? "ok" : "quiet";
      state.textContent = peak > 0.01 ? DIAG.heard : DIAG.quiet;
    } catch {
      state.dataset.state = "error";
      state.textContent = DIAG.refused;
    } finally {
      stream?.getTracks().forEach((t) => t.stop());
      button.disabled = false;
    }
  }
}
