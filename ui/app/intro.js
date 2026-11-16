// The intro. About 25 seconds, four captions in the headline voice, over the
// scene in its "intro" mode, ending on the Home card without a cut.

const BEATS = [
  { text: "Every day, you open dozens of things.", hold: 3000 },
  { text: "Then you lose your place.", hold: 3200, dim: true },
  { text: "Just say it.", hold: 3400, wake: true },
  { text: "Evo remembers where you were.", hold: 3600, remember: true },
  { text: "Pick up where you left off.", hold: 2200, handover: true },
];

export function createIntro({ scene, works, onDone, force = false }) {
  const root = document.querySelector("#intro");
  const label = root.querySelector("#introText");
  const caret = root.querySelector(".caret");
  const notch = document.querySelector("#notch");
  const ntx = document.querySelector("#ntx");
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

  root.hidden = false;
  document.body.dataset.intro = "";
  let done = false;
  const timers = [];
  // Reduced motion removes the typing and the transitions, never the words:
  // each caption still holds long enough to be read.
  const wait = (ms) => new Promise((resolve) => timers.push(setTimeout(resolve, ms)));

  async function type(text) {
    label.textContent = "";
    if (reduced) {
      label.textContent = text;
      return;
    }
    for (const ch of text) {
      if (done) return;
      label.textContent += ch;
      await wait(42);
    }
  }

  function wake(on, text = "") {
    if (on) {
      notch.dataset.on = "";
      ntx.innerHTML = text;
    } else {
      delete notch.dataset.on;
      ntx.textContent = "";
    }
  }

  function finish() {
    if (done) return;
    done = true;
    for (const t of timers) clearTimeout(t);
    wake(false);
    root.hidden = true;
    delete document.body.dataset.intro;
    document.body.classList.add("intro-handover");
    scene.setHighlight([]);
    scene.select(0);
    scene.setMode("home");
    scene.setListening(false);
    window.removeEventListener("keydown", onKey, true);
    root.removeEventListener("click", onKey, true);
    onDone?.();
  }

  const onKey = () => finish();
  window.addEventListener("keydown", onKey, true);
  root.addEventListener("click", onKey, true);

  (async () => {
    scene.setMode("intro");
    scene.setHighlight([]);
    await wait(300);
    for (const beat of BEATS) {
      if (done) return;
      if (beat.wake) {
        scene.setListening(true);
        wake(true, "<b>Listening</b>");
        await wait(700);
        ntx.innerHTML = "<b>Hey Evo</b>";
      }
      if (beat.remember) {
        // The reveal starts as the line types: the works light up under it.
        root.removeAttribute("data-dim");
        wake(false);
        scene.setListening(false);
        scene.setHighlight([0, 1, 2].filter((i) => works[i]));
      }
      await type(beat.text);
      if (beat.dim) root.dataset.dim = "";
      if (beat.handover) {
        document.body.classList.add("intro-handover");
      }
      await wait(beat.hold);
    }
    caret?.classList.add("done");
    finish();
  })();

  return { finish, force };
}
