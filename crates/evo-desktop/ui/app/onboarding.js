// Onboarding: four resumable steps in the Home card, over the scene.
// 1 welcome · 2 permissions one at a time · 3 the live "Hey Evo" test ·
// 4 a first win. Progress is stored, so quitting mid-way resumes the step.

import { api, isApp } from "./bridge.js";

const STORE = "evo.onboard";

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

export async function createOnboarding({ works, onDone, onOpen }) {
  const root = document.querySelector("#onboard");
  const saved = (() => {
    try { return { step: 0, mic: null, windows: null, ...JSON.parse(localStorage.getItem(STORE) || "{}") }; }
    catch { return { step: 0, mic: null, windows: null }; }
  })();
  const state = saved;
  // The step itself lives in the app's settings store (docs/BRIDGE.md), so a
  // quit mid-way resumes at the same step. The local copy is the browser
  // preview's fallback only.
  try {
    const settings = await api.settings();
    if (typeof settings?.onboarding_step === "number") state.step = settings.onboarding_step;
  } catch {
    /* browser preview */
  }
  const persist = () => {
    localStorage.setItem(STORE, JSON.stringify(state));
    if (isApp) api.setSettings({ onboarding_step: state.step }).catch(() => {});
  };

  let stream = null;
  let raf = 0;

  root.hidden = false;
  document.body.dataset.onboard = "";

  function stopMic() {
    cancelAnimationFrame(raf);
    stream?.getTracks().forEach((t) => t.stop());
    stream = null;
  }

  function finish(open) {
    stopMic();
    state.done = true;
    state.step = 3;
    persist();
    root.hidden = true;
    root.replaceChildren();
    delete document.body.dataset.onboard;
    onDone?.(open);
  }

  const dots = (step) =>
    el("div", { class: "steps", "aria-hidden": "true" },
      [0, 1, 2, 3].map((i) => el("i", { class: i <= step ? "on" : "" })));

  function card(step, title, body, actions, extra) {
    root.replaceChildren(
      el("div", { class: "panel-card" },
        dots(step),
        el("h2", {}, title),
        body,
        extra ?? null,
        el("div", { class: "actions" }, actions))
    );
  }

  // ── 1. welcome ────────────────────────────────────────────────────────
  function welcome() {
    card(0, "Hi. I'm Evo.",
      el("div", {},
        el("img", { src: "evo-mark.svg", alt: "Evo", style: "width:96px;height:96px;margin:0 auto 16px;display:block" }),
        el("p", {}, "I remember what you were working on, so you can pick it back up. This takes about a minute.")),
      [
        el("button", { class: "btn-primary", onclick: () => go(1) }, "Get started"),
        el("button", { class: "btn-ghost", onclick: () => finish() }, "Not now"),
      ]);
  }

  // ── 2. permissions, one at a time ─────────────────────────────────────
  function microphone() {
    const actions = [
      el("button", {
        class: "btn-primary",
        onclick: async () => {
          try {
            stream = await navigator.mediaDevices.getUserMedia({ audio: true });
            stopMic();
            state.mic = "allowed";
            persist();
            windows();
          } catch {
            state.mic = "denied";
            persist();
            deniedMic();
          }
        },
      }, "Allow"),
      el("button", { class: "btn-ghost", onclick: () => { state.mic = "later"; persist(); windows(); } }, "Not now"),
    ];
    card(1, "The microphone",
      el("p", {}, "Evo hears “Hey Evo” and nothing else. Audio is matched on this Mac and never sent anywhere."),
      actions,
      el("p", { class: "why" }, "If you say no, voice stays off and you can type instead. Everything else still works."));
  }

  function deniedMic() {
    card(1, "No microphone, no problem",
      el("p", {}, "Voice is off. You can turn it on later in Settings, or type commands instead."),
      [el("button", { class: "btn-primary", onclick: () => windows() }, "Continue")],
      el("p", { class: "why" }, "Evo never asks twice in a row."));
  }

  function windows() {
    card(1, "Which window you were in",
      el("p", {}, "Evo remembers the window in front, so it can put the right work back."),
      [
        el("button", {
          class: "btn-primary",
          onclick: () => {
            // A web page cannot grant this. The installed app asks macOS on
            // first launch; saying so is the honest version of "Allow".
            state.windows = "asked";
            persist();
            card(1, "macOS will ask",
              el("p", {}, "Grant Accessibility in System Settings → Privacy & Security when the app asks. Until then, Evo shows less and says so."),
              [el("button", { class: "btn-primary", onclick: () => go(2) }, "Continue")],
              el("p", { class: "why" }, "Nothing is captured without it."));
          },
        }, "Allow"),
        el("button", { class: "btn-ghost", onclick: () => { state.windows = "later"; persist(); go(2); } }, "Not now"),
      ],
      el("p", { class: "why" }, "If you say no, Home shows fewer works and never pretends otherwise."));
  }

  // ── 3. the live test ──────────────────────────────────────────────────
  function heyEvo() {
    const bars = el("div", { class: "wave", "aria-hidden": "true" }, Array.from({ length: 7 }, () => el("i")));
    const status = el("p", {}, state.mic === "allowed" ? "Say “Hey Evo”." : "Say “Hey Evo”, or type it.");
    const typed = el("input", { class: "field", style: "width:100%;height:var(--h-chip);padding:0 13px", placeholder: "hey evo", "aria-label": "Type the wake phrase" });

    const typeInstead = el("button", {
      class: "btn-ghost",
      onclick: () => {
        stopMic();
        card(2, "Type it instead",
          el("p", {}, "The installed app matches the phrase on device. This page can only check that you typed it."),
          [el("button", { class: "btn-primary", onclick: () => succeed("typed") }, "That's it")],
          typed);
        typed.focus();
        typed.addEventListener("keydown", (e) => {
          if (e.key === "Enter" && typed.value.toLowerCase().includes("hey evo")) succeed("typed");
        });
      },
    }, "Type it instead");

    const succeed = (how) => {
      stopMic();
      state.wake = how;
      persist();
      card(2, "That was it.",
        el("p", {}, "Evo is listening on this Mac. Closing the window keeps the menu bar and microphone agent running; Pause everything stops microphone capture."),
        el("div", { class: "helper-note" },
          el("p", {}, "Evo keeps a small helper running so it can hear “Hey Evo”. macOS will tell you about this. You can turn it off in Settings.")),
        [el("button", { class: "btn-primary", onclick: () => go(3) }, "Continue")]);
    };

    card(2, "Say “Hey Evo”", status, [typeInstead], bars);

    if (state.mic !== "allowed") {
      bars.style.opacity = ".35";
      return;
    }

    navigator.mediaDevices.getUserMedia({ audio: true }).then((s) => {
      stream = s;
      const ctx = new (window.AudioContext || window.webkitAudioContext)();
      const src = ctx.createMediaStreamSource(s);
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 512;
      src.connect(analyser);
      const data = new Uint8Array(analyser.fftSize);
      const kids = [...bars.children];
      let loudSince = 0;
      const loop = (t) => {
        analyser.getByteTimeDomainData(data);
        let peak = 0;
        for (const v of data) peak = Math.max(peak, Math.abs(v - 128) / 128);
        kids.forEach((bar, i) => {
          const h = 6 + peak * 20 * (0.6 + 0.4 * Math.sin(t / 120 + i));
          bar.style.height = `${Math.max(6, h)}px`;
        });
        if (peak > 0.08) {
          loudSince ||= Date.now();
          if (Date.now() - loudSince > 500) return succeed("voice");
        } else {
          loudSince = 0;
        }
        raf = requestAnimationFrame(loop);
      };
      raf = requestAnimationFrame(loop);
    }).catch(() => {
      bars.style.opacity = ".35";
      status.textContent = "The microphone is unavailable. Type it instead.";
    });
  }

  // ── 4. first win ──────────────────────────────────────────────────────
  function firstWin() {
    const picks = works.slice(0, 3);
    const body = picks.length
      ? el("div", { style: "display:flex;flex-direction:column;gap:6px" },
          picks.map((w) =>
            el("button", {
              class: "work",
              onclick: () => { finish(w); onOpen?.(w); },
            }, el("span", {}, el("strong", {}, w.title), el("small", {}, w.source)))))
      : el("p", {}, "Nothing here yet. Start something and Evo will remember it.");

    card(3, "Open one thing", el("p", {}, "Pick up the work you left. That's the whole idea."), [], body);
    if (!picks.length) {
      root.querySelector(".actions").append(
        el("button", { class: "btn-primary", onclick: () => finish() }, "Go to Home"));
    }
  }

  const STEPS = [welcome, () => (state.mic === "denied" ? deniedMic() : microphone()), heyEvo, firstWin];

  function go(step) {
    stopMic();
    state.step = step;
    persist();
    STEPS[step]();
  }

  go(Math.min(state.step ?? 0, 3));
  return { go, finish, stopMic };
}
