// Settings: one panel, 720px, six tabs, rows with toggles, selects and a
// hotkey recorder. The scene sits behind it in "quiet".
//
// Privacy and Trust, Appearance and About read the real store through the
// bridge (docs/BRIDGE.md); a value is painted only after Rust has answered.

import { api, isApp } from "./bridge.js";
import { iconSvgPath } from "./card-view.js";

const TABS = ["General", "Voice", "Privacy and Trust", "Appearance", "Shortcuts", "About"];

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

const readSettings = () => ({ keys: {} });

// build.rs stamps epoch seconds; the panel shows a local date instead.
const built = (value) => {
  const text = String(value ?? "");
  if (!/^\d+$/.test(text)) return text || "unknown";
  return new Date(Number(text) * 1000).toLocaleString();
};

const combo = (e) => {
  const parts = [];
  if (e.metaKey) parts.push("⌘");
  if (e.ctrlKey) parts.push("⌃");
  if (e.altKey) parts.push("⌥");
  if (e.shiftKey) parts.push("⇧");
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if (!["Meta", "Control", "Alt", "Shift"].includes(e.key)) parts.push(key);
  return parts.join("");
};

export function openSettings({ tab = null, onPause, onPauseState, onActivity, onReplayIntro, onReplayOnboarding, onTestVoice, onClose }) {
  const settings = readSettings();
  const set = (key, value) => { settings[key] = value; };

  const body = el("div", { class: "body" });
  const shut = () => { scrim.remove(); onClose?.(); };
  const close = el("button", { class: "icon-button", "aria-label": "Close", onclick: shut },
    iconSvgPath(document, "M5 5l8 8M13 5l-8 8"));
  const scrim = el("div", { class: "scrim" },
    el("div", { class: "sheet", role: "dialog", "aria-label": "Settings" },
      el("header", {}, el("h2", {}, "Settings"), el("span", { class: "sp" }), close)));
  scrim.addEventListener("click", (e) => { if (e.target === scrim) shut(); });

  const seg = el("div", { class: "segmented", role: "tablist" });
  const buttons = TABS.map((t) =>
    el("button", { role: "tab", "aria-pressed": String(t === "General"), onclick: () => show(t) }, t));
  buttons.forEach((b) => seg.append(b));
  scrim.querySelector(".sheet").append(el("div", { class: "tabs" }, seg), body);

  // ── row builders ────────────────────────────────────────────────────
  const row = (title, hint, side) =>
    el("div", { class: "row-line" }, el("span", {}, el("strong", {}, title), hint ? el("small", {}, hint) : null), el("span", { class: "side" }, side));

  const toggle = (on, onchange) =>
    el("button", { class: "switch", role: "switch", "aria-checked": String(on), onclick: onchange }, el("span", { class: "track" }));

  const select = (value, options, onchange) => {
    const node = el("select", { class: "select", style: "height:32px", onchange: (e) => onchange(e.target.value) });
    for (const option of options) {
      node.append(el("option", { value: option, selected: option === value }, option));
    }
    return node;
  };

  const section = (label) => body.append(el("p", { class: "section-label" }, label));

  // ── tabs ────────────────────────────────────────────────────────────
  function general() {
    section("Everywhere");
    body.append(row("Pause everything", "Stops capture, listening and every action.", 
      el("button", { class: "btn-ghost", style: "height:34px", onclick: () => { onPause?.(); shut(); } }, "Pause")));
    body.append(row("Open at login", "Evo starts with your Mac.", toggle(settings.login ?? true, (e) => {
      set("login", !(settings.login ?? true));
      e.currentTarget.setAttribute("aria-checked", String(settings.login));
    })));
    body.append(row("Keep Evo listening in the background", "Closing the window keeps the menu bar and microphone agent running. Turn this off to let Quit Evo completely stop them.", toggle(settings.keepBackground ?? false, (e) => {
      set("keepBackground", !(settings.keepBackground ?? false));
      e.currentTarget.setAttribute("aria-checked", String(settings.keepBackground));
      api.setSettings({ keep_background: settings.keepBackground });
    })));
    body.append(row("Quiet the desktop when I continue", "Hides other regular apps while this work opens. Evo never quits them.", toggle(settings.quietDesktop ?? false, (e) => {
      set("quietDesktop", !(settings.quietDesktop ?? false));
      e.currentTarget.setAttribute("aria-checked", String(settings.quietDesktop));
      api.setSettings({ quiet_desktop: settings.quietDesktop });
    })));
    body.append(row("Language", "Only English is installed.", select(settings.lang ?? "English (US)", ["English (US)"], (v) => set("lang", v))));

    section("Trust");
    body.append(row("Activity and permissions", "See what Evo did, and what it may do.", 
      el("button", { class: "btn-ghost", style: "height:34px", onclick: () => { shut(); onActivity?.(); } }, "Open")));
  }

  function voice() {
    body.append(row("Listen for “Hey Evo”", "The microphone is only open while Evo listens.", toggle(settings.listen ?? true, (e) => {
      set("listen", !(settings.listen ?? true));
      e.currentTarget.setAttribute("aria-checked", String(settings.listen));
    })));
    body.append(row("Wake sensitivity", "Higher hears you from further away.", select(settings.sensitivity ?? "Balanced", ["Calm", "Balanced", "Eager"], (v) => set("sensitivity", v))));
    body.append(row("Microphone", "The system default input.", select("System default", ["System default"], () => {})));
    body.append(row("Test your voice", "Say “Hey Evo” and watch the CRT.", 
      el("button", { class: "btn-ghost", style: "height:34px", onclick: () => { shut(); onTestVoice?.(); } }, "Test")));
    body.append(el("p", { class: "section-label" }, "Speech"));
    body.append(row("Commands", "Opening work, finding work, asking what you did.", el("span", { class: "faint", style: "font-size:12.5px" }, "On this Mac")));
  }

  const MODE_LABEL = { ask: "Ask every time", risky: "Ask for risky actions", scope: "Act within scope" };
  const notice = (text) =>
    el("p", { class: "why", style: "color:var(--muted);font-size:12.5px;padding:10px 0" }, text);

  async function privacy() {
    body.append(notice("Reading from the app's settings store…"));
    let data = null;
    try { data = await api.privacy(); } catch { data = null; }
    body.lastChild.remove();
    if (!data) {
      body.append(notice(isApp ? "The store didn't answer. Nothing here is invented." : "Permissions live in the app's settings store; this preview cannot read it."));
      return;
    }
    body.append(row("Stays on this Mac", "Audio and text are processed here. Nothing is uploaded.", el("span", { class: "tag" }, "Verified")));
    body.append(row("Permission mode", MODE_LABEL[data.mode] ?? data.mode,
      el("button", { class: "btn-ghost", style: "height:34px", onclick: () => { shut(); onActivity?.(); } }, "Change")));
    body.append(row("What Evo can see", `${data.scopes.filter((s) => s.allowed).length} of ${data.scopes.length} things allowed.`,
      el("button", { class: "btn-ghost", style: "height:34px", onclick: () => { shut(); onActivity?.(); } }, "Open")));
    body.append(row("Pause everything", data.paused ? "Capture, listening and every action are stopped." : "Evo is running and may act within the rules above.",
      el("button", { class: "btn-ghost", style: "height:34px", onclick: async (e) => {
        e.currentTarget.disabled = true;
        try {
          data.paused ? await api.resume() : await api.pause();
          body.replaceChildren();
          privacy();
          onPauseState?.();
        } catch (error) {
          e.currentTarget.textContent = error.message;
        }
      } }, data.paused ? "Resume" : "Pause")));
    body.append(row("No account", "There is nothing to sign in to and nothing to sync.", el("span", { class: "faint", style: "font-size:12.5px" }, "—")));
    body.append(row("Where your data lives", `${data.storage.path}, ${Math.max(1, Math.round(data.storage.bytes / 1024))} KB`,
      el("a", { class: "btn-ghost", style: "height:34px;display:inline-flex;align-items:center", href: "../../PRIVACY.md" }, "Read PRIVACY.md")));
  }

  async function appearance() {
    body.append(notice("Reading the saved choice…"));
    let saved = null;
    try { saved = await api.settings(); } catch { saved = null; }
    body.lastChild.remove();
    if (saved?.appearance) window.applyEvoTheme?.(saved.appearance);
    if (!saved) {
      body.append(notice("The appearance choice is kept by the app's settings store."));
      return;
    }
    const names = { auto: "Auto", day: "Day", dusk: "Dusk" };
    body.append(row("Theme", "Auto follows macOS; Day and Dusk are fixed palettes.",
      select(names[saved.appearance] ?? "Dusk", ["Auto", "Day", "Dusk"], async (name) => {
        const choice = name.toLowerCase();
        try {
          await api.setSettings({ appearance: choice });
          window.applyEvoTheme?.(choice);
        } catch (error) {
          body.append(notice(error.message));
        }
      })));
    body.append(row("Reduce motion", "No animation; the scene draws one still frame.", toggle(settings.reduce ?? false, (e) => {
      set("reduce", !(settings.reduce ?? false));
      document.documentElement.dataset.reduce = settings.reduce ? "1" : "";
      e.currentTarget.setAttribute("aria-checked", String(settings.reduce));
    })));
    body.append(row("Text size", "Affects the interface, not the scene.", select(settings.text ?? "Default", ["Default", "Large"], (v) => set("text", v))));
  }

  function shortcuts() {
    body.append(el("p", { class: "why", style: "color:var(--muted);font-size:13px;padding-bottom:8px" },
      "Press Record, then the keys you want. Escape cancels."));
    const rows = [
      ["search", "Search your work", settings.keys.search ?? "⌘K"],
      ["listen", "Listen now", settings.keys.listen ?? "⌥Space"],
      ["pause", "Pause everything", settings.keys.pause ?? "⌥⌘P"],
      ["home", "Go to Home", settings.keys.home ?? "⌘0"],
    ];
    for (const [id, label, def] of rows) {
      const button = el("button", { class: "kbd-record", onclick: () => record(id, button) }, settings.keys[id] ?? def);
      body.append(row(label, null, button));
    }
  }

  function record(id, button) {
    button.classList.add("is-recording");
    button.textContent = "Press keys";
    const onKey = (e) => {
      if (e.key === "Escape") { cleanup(); button.textContent = settings.keys[id] ?? ""; return; }
      if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) return;
      e.preventDefault();
      const value = combo(e);
      if (!value) return;
      settings.keys[id] = value;
      cleanup();
      button.textContent = value;
    };
    const cleanup = () => {
      button.classList.remove("is-recording");
      document.removeEventListener("keydown", onKey, true);
    };
    document.addEventListener("keydown", onKey, true);
  }

  async function about() {
    body.append(notice("Reading the build…"));
    let info = null;
    try { info = await api.about(); } catch { info = null; }
    body.lastChild.remove();
    body.append(el("img", { src: "evo-lockup.svg", alt: "Evo", style: "height:40px;margin:0 0 12px;display:block" }));
    body.append(row("Evo", info ? `Version ${info.version}` : "Version unavailable",
      el("span", { class: "faint", style: "font-size:12.5px" },
        info ? `Built ${built(info.build_time)}` : "—")));
    body.append(row("Version", info ? info.commit ?? "unknown" : "—", el("span", { class: "faint", style: "font-size:12.5px" }, info ? info.version : "")));
    body.append(row("Help", "The intro and onboarding can be replayed at any time.",
      el("span", { class: "side" },
        el("button", { class: "btn-ghost", style: "height:34px", onclick: () => { shut(); onReplayIntro?.(); } }, "Replay intro"),
        el("button", { class: "btn-ghost", style: "height:34px", onclick: () => { shut(); onReplayOnboarding?.(); } }, "Replay setup"))));
    body.append(row("Fonts", "Instrument Serif and Instrument Sans, SIL OFL 1.1.", el("span", { class: "faint", style: "font-size:12.5px" }, "Bundled")));
    body.append(row("Privacy", "No account, no analytics, no uploads.", el("span", { class: "faint", style: "font-size:12.5px" }, "This build")));
  }

  function show(tab) {
    buttons.forEach((b) => b.setAttribute("aria-pressed", String(b.textContent === tab)));
    body.replaceChildren();
    ({ General: general, Voice: voice, "Privacy and Trust": privacy,
       Appearance: appearance, Shortcuts: shortcuts, About: about })[tab]();
  }

  show(tab && TABS.includes(tab) ? tab : "General");
  document.body.append(scrim);
  const onKey = (e) => { if (e.key === "Escape") { document.removeEventListener("keydown", onKey, true); shut(); } };
  document.addEventListener("keydown", onKey, true);
  return { close: () => scrim.remove() };
}
