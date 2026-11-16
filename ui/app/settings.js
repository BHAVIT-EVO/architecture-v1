// Settings: one panel, 720px, six tabs, rows with toggles, selects and a
// hotkey recorder. The scene sits behind it in "quiet".

const STORE = "evo.settings";
const TABS = ["General", "Voice", "Privacy and Trust", "Appearance", "Shortcuts", "About"];

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

const readSettings = () => {
  try { return { keys: {}, ...JSON.parse(localStorage.getItem(STORE) || "{}") }; }
  catch { return { keys: {} }; }
};
const saveSettings = (s) => localStorage.setItem(STORE, JSON.stringify(s));

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

export function openSettings({ onPause, onActivity, onReplayIntro, onReplayOnboarding, onTestVoice, onClose }) {
  const settings = readSettings();
  const set = (key, value) => { settings[key] = value; saveSettings(settings); };

  const body = el("div", { class: "body" });
  const shut = () => { scrim.remove(); onClose?.(); };
  const close = el("button", { class: "icon-button", "aria-label": "Close", onclick: shut },
    el("span", { html: '<svg width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M5 5l8 8M13 5l-8 8"/></svg>' }));
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

  function privacy() {
    body.append(row("Stays on this Mac", "Audio and text are processed here. Nothing is uploaded.", el("span", { class: "tag" }, "Verified")));
    body.append(row("History", "How long Evo keeps what it saw.", select(settings.retention ?? "Forever", ["Forever", "1 year", "90 days"], (v) => set("retention", v))));
    body.append(row("What Evo can see", "Folders, apps and sites, with three modes.", 
      el("button", { class: "btn-ghost", style: "height:34px", onclick: () => { shut(); onActivity?.(); } }, "Open")));
    body.append(row("No account", "There is nothing to sign in to and nothing to sync.", el("span", { class: "faint", style: "font-size:12.5px" }, "—")));
    body.append(row("Where your data lives", "~/Library/Application Support/Evo/storage",
      el("a", { class: "btn-ghost", style: "height:34px;display:inline-flex;align-items:center", href: "../../PRIVACY.md" }, "Read PRIVACY.md")));
  }

  function appearance() {
    body.append(row("Theme", "Dusk is the only palette in this build.", select(settings.theme ?? "Dusk", ["Dusk", "Match macOS"], (v) => set("theme", v))));
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
      saveSettings(settings);
      cleanup();
      button.textContent = value;
    };
    const cleanup = () => {
      button.classList.remove("is-recording");
      document.removeEventListener("keydown", onKey, true);
    };
    document.addEventListener("keydown", onKey, true);
  }

  function about() {
    body.append(row("Evo", "Work memory for this Mac.", el("span", { class: "faint", style: "font-size:12.5px" }, "0.1.0")));
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

  show("General");
  document.body.append(scrim);
  const onKey = (e) => { if (e.key === "Escape") { document.removeEventListener("keydown", onKey, true); shut(); } };
  document.addEventListener("keydown", onKey, true);
  return { close: () => scrim.remove() };
}
