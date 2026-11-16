// Home. The newest work is selected, shown once, and everything else is
// behind it — in the scene and in the list, never both.

import { createScene, hitRects } from "../scene/index.js";
import { projection, whenText } from "./works.js";
import { createIntro } from "./intro.js";
import { createOnboarding } from "./onboarding.js";
import { openSearch } from "./search.js";
import { openSettings } from "./settings.js";
import { openActivity, logEvent, isPaused, refreshStatus, togglePause as flipPause } from "./activity.js";
import { openHelp, openDiagnostics } from "./help.js";
import { openTray, showState, clearState } from "./states.js";
import { api, isApp } from "./bridge.js";
import { paintCard, iconSvg } from "./card-view.js";

const $ = (s) => document.querySelector(s);
const STORE = "evo.home.overrides";

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

const overrides = (() => {
  try {
    return { hidden: [], renamed: {}, merged: [], ...JSON.parse(localStorage.getItem(STORE) || "{}") };
  } catch {
    return { hidden: [], renamed: {}, merged: [] };
  }
})();
const save = () => localStorage.setItem(STORE, JSON.stringify(overrides));

// The works come from Rust (docs/BRIDGE.md). A failure is a real error state,
// not a silent fallback to something invented. Mock mode exists only in a
// normal browser, inside bridge.js.
let feedFailed = false;
const inApp = isApp;
const loaded = await api.getWorks().catch(() => {
  feedFailed = true;
  return { works: [] };
});
try {
  const savedTheme = await api.settings();
  window.applyEvoTheme?.(savedTheme.appearance);
} catch (error) {
  console.error("Could not read the saved appearance.", error);
  window.applyEvoTheme?.("dusk");
}
if (inApp) document.body.classList.add("evo-shell");
await refreshStatus();
const rows = (loaded.works ?? []).map((work) => ({
  id: work.id,
  raw: work.title ?? work.raw ?? "",
  openedAgo: work.last_active_ms
    ? Math.max(1, Math.round((Date.now() - work.last_active_ms) / 60_000))
    : 0,
  source: work.source,
}));

// A fresh install has no history. Three labelled objects keep the world
// whole without pretending to be anything someone did.
const EXAMPLES = ["book", "paper", "folder"].map((kind, i) => ({
  id: `example-${i + 1}`,
  raw: "Example",
  source: "Example",
  openedAgo: 30 + i * 40,
  kind,
}));
const feed = rows.length ? rows : EXAMPLES;

let model = projection(
  feed
    .filter((r) => !overrides.hidden.includes(r.id) && !overrides.merged.includes(r.id))
    .map((r) => ({ ...r, raw: overrides.renamed[r.id] ?? r.raw }))
);

const ICON = { book: "chat", paper: "play", folder: "dir", notebook: "web" };

const sceneEl = $(".scene");
const scene = createScene({ canvas: $(".scene canvas"), works: model.sceneWorks, mode: "home" });
scene.setLoading(true);
const hits = [];

let selected = 0;
let preview = -1;

// ── card ────────────────────────────────────────────────────────────────
function paint(index) {
  const w = model.sceneWorks[index] ?? model.selected;
  paintCard(
    { title: $("#pt"), when: $("#pw"), source: $("#ps"), icon: $("#pi use") },
    w,
    whenText
  );
  $("#cont").disabled = !w;
  $("#continue-outcome").hidden = true;
  $("#continue-action").hidden = true;
}

function show(i) {
  preview = i;
  paint(i);
  const rows = document.querySelectorAll(".list .row");
  rows.forEach((r, j) => r.setAttribute("aria-current", String(j + 1 === i)));
}

function select(i) {
  selected = i;
  scene.select(i);
  show(i);
}

// ── scene hit targets ───────────────────────────────────────────────────
function placeHits() {
  if (!sceneEl.clientWidth) return;
  const rects = hitRects(model.sceneWorks, sceneEl.clientWidth, sceneEl.clientHeight);
  hits.forEach((b, i) => {
    const r = rects[i];
    Object.assign(b.style, {
      left: `${r.left}px`, top: `${r.top}px`, width: `${r.width}px`, height: `${r.height}px`,
    });
  });
  Object.assign(crtHit.style, {
    left: `${rects.crt.left}px`, top: `${rects.crt.top}px`,
    width: `${rects.crt.width}px`, height: `${rects.crt.height}px`, borderRadius: "10px",
  });
}

const crtHit = el("button", { class: "hit", "aria-label": "Talk to Evo", onclick: () => listen() });
sceneEl.append(crtHit);

function rebuildHits() {
  for (const b of hits) b.remove();
  hits.length = 0;
  model.sceneWorks.forEach((w, i) => {
    const b = el("button", {
      class: "hit",
      "aria-label": w.title,
      onmouseenter: () => show(i),
      onmouseleave: () => show(selected),
      onfocus: () => show(i),
      onblur: () => show(selected),
      onclick: () => select(i),
    }, el("span", { class: "tag" }, w.title));
    sceneEl.insertBefore(b, crtHit);
    hits.push(b);
  });
  placeHits();
}

new ResizeObserver(placeHits).observe(sceneEl);

// ── list ────────────────────────────────────────────────────────────────
function renderList(query = "") {
  const needle = query.trim().toLowerCase();
  const list = $("#list");
  list.replaceChildren();
  const rest = model.rest.filter(
    (w) => !needle || w.title.toLowerCase().includes(needle) || w.source.toLowerCase().includes(needle)
  );
  $("#moreN").hidden = rest.length === 0;
  $("#moreN").textContent = `Show ${rest.length} more in the list`;

  for (const w of rest) {
    list.append(
      el("button", { class: "row", role: "listitem", onclick: () => openInScene(w.id) },
        el("span", { class: "tile" }, iconSvg(document, w.kind)),
        el("span", {}, el("strong", {}, w.title), el("small", {}, w.source)),
        el("time", {}, whenText(w.opened).replace("Last open ", "")))
    );
  }
}

function openInScene(id) {
  if (isPaused()) return pausedRefusal();
  const i = model.sceneWorks.findIndex((w) => w.id === id);
  if (i >= 0) select(i);
  view("scene");
}

// The kill switch is not a mute: while paused, opening is refused, not queued.
function pausedRefusal() {
  toast("Evo is paused. Resume from the tray to open work.");
  return false;
}

// ── view toggle ─────────────────────────────────────────────────────────
function view(v) {
  document.body.dataset.view = v;
  $("#vs").setAttribute("aria-pressed", String(v === "scene"));
  $("#vl").setAttribute("aria-pressed", String(v === "list"));
}
$("#vs").onclick = () => view("scene");
$("#vl").onclick = () => view("list");
$("#moreN").onclick = () => view("list");

// ── menu ────────────────────────────────────────────────────────────────
const menu = $("#mn");
const menuItems = [
  ["Rename", rename],
  ["Restore earlier version", null, "No earlier version is recorded yet."],
  ["This isn’t work", () => removeSelected("Not work. It won’t appear on Home again.")],
  ["Hide from Home", () => removeSelected("Hidden from Home.")],
  ["Merge with…", merge],
];
for (const [label, fn, reason] of menuItems) {
  menu.append(
    el("button", {
      class: "menu-item",
      role: "menuitem",
      disabled: !fn,
      title: reason ?? null,
      onclick: (e) => { e.stopPropagation(); closeMenu(); fn?.(); },
    }, label)
  );
}

function closeMenu() {
  menu.removeAttribute("data-open");
  $("#mb").setAttribute("aria-expanded", "false");
}
$("#mb").onclick = (e) => {
  e.stopPropagation();
  const open = menu.hasAttribute("data-open");
  menu.toggleAttribute("data-open", !open);
  $("#mb").setAttribute("aria-expanded", String(!open));
};
document.addEventListener("click", closeMenu);

async function removeSelected(message) {
  const w = model.sceneWorks[selected] ?? model.selected;
  if (!w) return;
  if (inApp) {
    try {
      await api.hideWork(w.id);
    } catch (error) {
      toast(error.message);
      return;
    }
  }
  overrides.hidden.push(w.id);
  save();
  logEvent({ what: `Hid “${w.title}”`, why: "You asked Home to hide it", undo: { kind: "hide", id: w.id } });
  refresh();
  toast(message);
}

// ── dialogs ─────────────────────────────────────────────────────────────
function dialog(title, body, actions) {
  const scrim = el("div", { class: "dialog-scrim" });
  const box = el("div", { class: "dialog" }, el("h2", {}, title), body,
    el("div", { class: "actions" }, ...actions));
  scrim.append(box);
  scrim.addEventListener("click", (e) => { if (e.target === scrim) scrim.remove(); });
  document.body.append(scrim);
  return scrim;
}

function rename() {
  const w = model.sceneWorks[selected] ?? model.selected;
  if (!w) return;
  const input = el("input", {
    class: "field", style: "width:100%;padding:0 13px",
    value: w.title, "aria-label": "Work title",
  });
  input.style.height = "var(--h-primary)";
  const scrim = dialog("Rename this work", input, [
    el("button", { class: "btn-ghost", onclick: () => scrim.remove() }, "Cancel"),
    el("button", {
      class: "btn-primary",
      onclick: async () => {
        const value = input.value.trim();
        if (!value) return;
        if (inApp) {
          try {
            await api.renameWork(w.id, value);
          } catch (error) {
            toast(error.message);
            return;
          }
        }
        overrides.renamed[w.id] = value;
        save();
        logEvent({ what: `Renamed a work to “${value}”`, why: "You renamed it on Home", undo: { kind: "rename", id: w.id } });
        scrim.remove();
        refresh();
        toast("Renamed.");
      },
    }, "Save"),
  ]);
  input.focus();
  input.select();
}

function merge() {
  const w = model.sceneWorks[selected] ?? model.selected;
  const others = model.rest;
  if (!w) return;
  if (others.length === 0) {
    toast("There is no other work to merge with.");
    return;
  }
  const pick = el("div", { class: "menu", style: "box-shadow:none;border:0;padding:0" });
  const scrim = dialog(`Merge “${w.title}” with…`, pick, [
    el("button", { class: "btn-ghost", onclick: () => scrim.remove() }, "Cancel"),
  ]);
  for (const other of others) {
    pick.append(el("button", {
      class: "menu-item",
      onclick: async () => {
        if (inApp) {
          try {
            await api.mergeWorks(w.id, other.id);
          } catch (error) {
            toast(error.message);
            return;
          }
        }
        overrides.merged.push(other.id);
        save();
        logEvent({ what: `Merged “${other.title}”`, why: "You said it was the same work", undo: { kind: "merge", id: other.id } });
        scrim.remove();
        refresh();
        toast(`Merged “${other.title}”.`);
      },
    }, other.title));
  }
}

// ── toast ───────────────────────────────────────────────────────────────
function toast(text) {
  const node = el("div", { class: "toast", style: "position:fixed;left:32px;bottom:28px;z-index:50" },
    el("span", { class: "dot" }), text);
  document.body.append(node);
  setTimeout(() => node.remove(), 2800);
  return node;
}

function showVoiceEvent(event) {
  const type = event?.type;
  const chip = $(".chip-live");
  if (type === "listening") {
    chip.textContent = "Listening";
    chip.classList.add("is-active");
    toast("Listening");
  } else if (type === "live_transcript") {
    chip.textContent = event.partial ? `Heard: ${event.partial}` : "Listening";
  } else if (type === "thinking") {
    chip.textContent = "Thinking";
  } else if (type === "choices") {
    const choices = Array.isArray(event.choices) ? event.choices.slice(0, 3) : [];
    toast(choices.length ? `Which one: ${choices.join(", ")}` : "I couldn't find that. Try another name, or type it.");
  } else if (type === "opened") {
    chip.textContent = "Listening";
    toast(event.title ? `Opened ${event.title}.` : (event.summary ?? "Opened."));
  } else if (type === "not_found") {
    toast(event.message ?? "I couldn't find that. Try another name, or type it.");
  } else if (type === "no_speech") {
    toast("I didn't catch that. Try again, or type it.");
  } else if (type === "mic_denied") {
    const node = toast(event.message ?? "Microphone access is off. Allow access in System Settings.");
    const action = el("button", { class: "btn-ghost" }, event.action ?? "Allow microphone access");
    action.onclick = () => { window.location.href = "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"; };
    node.append(action);
  } else if (type === "agent_error") {
    toast(event.message ?? "Voice could not start. Try again, or type it.");
  }
}

window.addEventListener("evo-voice-event", (event) => showVoiceEvent(event.detail));

// ── continue ────────────────────────────────────────────────────────────
// The real notch is native; this page only asks Rust to bring a work back.
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
let busy = false;

async function continueSelected() {
  if (busy) return;
  const w = model.selected;
  if (!w) return;
  if (isPaused()) return pausedRefusal();
  busy = true;
  scene.setListening(true);
  try {
    const result = await api.continueWork(w.id);
    const outcome = result?.hint ?? "Evo could not open this work.";
    const action = result?.action ?? "Try again";
    const outcomeNode = $("#continue-outcome");
    const actionNode = $("#continue-action");
    outcomeNode.textContent = outcome;
    outcomeNode.hidden = false;
    actionNode.textContent = action;
    actionNode.hidden = false;
    actionNode.onclick = () => {
      if (action === "Try again") continueSelected();
      if (action === "Put things back") api.putThingsBack().then(() => toast("Put things back")).catch((error) => toast(error.message));
      if (action === "Allow access") window.location.href = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
    };
    $("#cont").disabled = true;
    toast(outcome);
  } catch (error) {
    toast(error.message);
  } finally {
    scene.setListening(false);
    busy = false;
  }
}
$("#cont").onclick = continueSelected;

// ── headline ────────────────────────────────────────────────────────────
async function typeHeadline() {
  const text = "Pick up where you left off.";
  const target = $("#ht");
  if (reduced) {
    target.textContent = text;
    $("#caret").classList.add("done");
    return;
  }
  for (const ch of text) {
    target.textContent += ch;
    await wait(42);
  }
  $("#caret").classList.add("done");
}

// ── keyboard ────────────────────────────────────────────────────────────
addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    closeMenu();
    document.querySelector(".dialog-scrim")?.remove();
  }
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
    e.preventDefault();
    openSearch({
      works: model.works,
      searchFn: inApp
        ? async (query) => {
            const found = await api.search(query);
            return (found.results ?? []).map((hit) => ({
              id: hit.id,
              title: hit.title,
              source: "Evo",
              opened: Date.now(),
            }));
          }
        : null,
      onPick: (work) => {
        if (isPaused()) return pausedRefusal();
        const i = model.sceneWorks.findIndex((w) => w.id === work.id);
        if (i >= 0) { select(i); view("scene"); }
        logEvent({ what: `Opened “${work.title}”`, why: "You searched for it" });
      },
    });
  }
  if (e.target instanceof Element && e.target.matches("input, textarea")) return;
  const count = model.sceneWorks.length;
  if (count === 0) return;
  if (e.key === "ArrowRight" || e.key === "ArrowDown") {
    e.preventDefault();
    select((selected + 1) % count);
  }
  if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
    e.preventDefault();
    select((selected - 1 + count) % count);
  }
  const onControl = document.activeElement instanceof Element &&
    document.activeElement.matches("input, button, textarea, [role=menuitem]");
  if (e.key === "Enter" && !onControl) {
    e.preventDefault();
    listen();
  }
});

$("#q").addEventListener("input", (e) => {
  renderList(e.target.value);
  if (e.target.value) view("list");
});

// ── boot ────────────────────────────────────────────────────────────────
function refresh() {
  const currentId = (model.sceneWorks[selected] ?? model.selected)?.id;
  const hidden = (r) => !overrides.hidden.includes(r.id) && !overrides.merged.includes(r.id);
  model = projection(feed.filter(hidden).map((r) => ({ ...r, raw: overrides.renamed[r.id] ?? r.raw })));
  // createScene captured this array; keep its identity and swap the contents.
  sceneWorks.length = 0;
  sceneWorks.push(...model.sceneWorks);
  const keep = currentId ? model.sceneWorks.findIndex((w) => w.id === currentId) : 0;
  selected = keep >= 0 ? keep : 0;
  preview = -1;
  scene.select(selected);
  rebuildHits();
  renderList($("#q").value);
  paint(selected);
}

const sceneWorks = model.sceneWorks;
rebuildHits();
renderList();
paint(0);
scene.setLoading(false);
if (location.hash === "#list") view("list");

// ── first launch: the intro, then onboarding ──────────────────────────
function startOnboarding() {
  createOnboarding({
    works: model.sceneWorks,
    onDone: () => localStorage.setItem("evo.onboard.done", "1"),
    onOpen: (w) => {
      const i = model.sceneWorks.findIndex((x) => x.id === w.id);
      if (i >= 0) select(i);
      view("scene");
      listen();
    },
  });
}

const introDone = localStorage.getItem("evo.intro.done");
const onboardDone = localStorage.getItem("evo.onboard.done");
const params = new URLSearchParams(location.search);
const replay = params.get("replay");
const entry = params.get("entry");

if (entry === "off") {
  // A returning user: no intro, no onboarding (used by screenshots/tests).
  typeHeadline();
} else if (entry === "onboarding") {
  typeHeadline();
  startOnboarding();
} else if (replay === "intro" || (!introDone && replay !== "onboarding")) {
  typeHeadline();
  createIntro({
    scene,
    works: model.sceneWorks,
    onDone: () => {
      localStorage.setItem("evo.intro.done", "1");
      if (!onboardDone) startOnboarding();
    },
  });
} else {
  typeHeadline();
  if (!onboardDone || replay === "onboarding") startOnboarding();
}

// ── search, settings, activity ────────────────────────────────────────
const replayIntro = () => {
  localStorage.removeItem("evo.intro.done");
  localStorage.removeItem("evo.onboard.done");
  location.search = "?replay=intro";
};
const replayOnboarding = () => {
  localStorage.setItem("evo.intro.done", "1");
  localStorage.removeItem("evo.onboard.done");
  localStorage.removeItem("evo.onboard");
  location.search = "?replay=onboarding";
};

function reflectPause() {
  const paused = isPaused();
  document.body.toggleAttribute("data-paused", paused);
  if (paused) scene.setListening(false);
  const chip = document.querySelector(".chip-live");
  chip?.setAttribute("data-state", paused ? "paused" : "live");
  const label = chip?.lastChild;
  if (label && label.nodeType === 3) label.textContent = paused ? "Not listening" : "Listening for “Hey Evo”";
}

function openActivityScreen() {
  return openActivity({
    onChange: () => {
      reflectPause();
      refresh();
    },
  });
}

function openSettingsScreen(tab = null) {
  scene.setMode("quiet");
  document.querySelector(".scene").dataset.mode = "quiet";
  openSettings({
    tab,
    onClose: () => {
      scene.setMode("home");
      document.querySelector(".scene").dataset.mode = "home";
    },
    onPause: async () => { await flipPause(); reflectPause(); },
    onPauseState: () => { reflectPause(); refresh(); },
    onActivity: openActivityScreen,
    onReplayIntro: replayIntro,
    onReplayOnboarding: replayOnboarding,
    onTestVoice: () => { location.search = "?replay=onboarding"; },
  });
}

$("#settings").onclick = openSettingsScreen;

document.querySelector(".trust button")?.addEventListener("click", openActivityScreen);
reflectPause();

// ── help, diagnostics and the tray ────────────────────────────────────
function withQuietScene(open) {
  const sceneEl = $(".scene");
  scene.setMode("quiet");
  sceneEl.dataset.mode = "quiet";
  open(() => {
    scene.setMode("home");
    sceneEl.dataset.mode = "home";
  });
}

function openHelpScreen() {
  withQuietScene((done) => openHelp({
    onClose: done,
    onReplayTour: () => { done(); replayIntro(); },
    onDiagnostics: () => { done(); openDiagnosticsScreen(); },
  }));
}

function openDiagnosticsScreen() {
  withQuietScene((done) => openDiagnostics({ works: model.works, onClose: done }));
}

async function togglePause() {
  try {
    await flipPause();
  } catch (error) {
    toast(error.message);
  }
  reflectPause();
}

$("#trayAnchor").onclick = (e) => {
  e.stopPropagation();
  openTray({
    anchor: $("#trayAnchor"),
    listening: !isPaused(),
    paused: isPaused(),
    onPause: togglePause,
    onOpen: () => window.focus(),
    onHelp: openHelpScreen,
    onDiagnostics: openDiagnosticsScreen,
    onQuit: () => api.quit?.(),
  });
};

addEventListener("keydown", (e) => {
  if (e.key === "?" && !e.metaKey && !e.ctrlKey) {
    const tag = document.activeElement?.tagName;
    if (tag !== "INPUT" && tag !== "TEXTAREA") { e.preventDefault(); openHelpScreen(); }
  }
});

// ── the four states ───────────────────────────────────────────────────
const wanted = params.get("state");
if (wanted === "loading") {
  scene.setLoading(true);
} else if (wanted === "empty") {
  scene.setMode("empty");
  sceneEl.dataset.mode = "empty";
  showState("empty", { onAction: openHelpScreen });
} else if (wanted) {
  showState(wanted, { onAction: () => location.reload() });
} else if (feedFailed) {
  showState(navigator.onLine ? "error" : "offline", { onAction: () => location.reload() });
} else if (!rows.length) {
  scene.setMode("empty");
  sceneEl.dataset.mode = "empty";
  showState("empty", { onAction: openHelpScreen });
} else {
  clearState();
}

// Deep links for the proof shots: ?panel=search|settings|activity
if (entry === "off") {
  const panel = params.get("panel");
  if (panel === "search") openSearch({ works: model.works });
  if (panel === "settings") openSettingsScreen();
  if (panel === "activity") openActivityScreen();
  if (panel === "permissions") openActivityScreen()?.show("Permissions");
  if (panel === "data") openActivityScreen()?.show("Data");
  if (panel === "appearance") openSettingsScreen("Appearance");
  if (panel === "about") openSettingsScreen("About");
  if (panel === "help") openHelpScreen();
  if (panel === "diagnostics") openDiagnosticsScreen();
  if (panel === "tray") $("#trayAnchor").click();
}
