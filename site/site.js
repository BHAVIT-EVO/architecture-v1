// The site's only script: the hero's scene and notch demo, the 404 scene, and
// the OS-aware download link. Everything else works without it.

import { createScene } from "../ui/scene/index.js";
import { at, unit, TS } from "../ui/scene/sprites.js";

const KINDS = ["book", "paper", "folder", "notebook"];
const onPath = (count) => TS.slice(0, count).map((t, i) => {
  const p = at(t);
  return { x: p.x, y: p.y, u: unit(t), kind: KINDS[i % KINDS.length] };
});

const $ = (s) => document.querySelector(s);

const OS = (() => {
  const ua = navigator.userAgent;
  if (/Mac/.test(ua)) return { name: "macOS", file: "downloads/Evo-0.1.0.dmg" };
  if (/Win/.test(ua)) return { name: "Windows", file: "downloads/Evo-0.1.0.exe" };
  return { name: "Linux", file: "downloads/evo-0.1.0.AppImage" };
})();

// One label everywhere, as §11 asks: "Download Evo, it's free".
for (const button of document.querySelectorAll("[data-download]")) {
  button.href = "download.html";
  const note = document.querySelector("[data-os-note]");
  if (note) note.textContent = OS.file.split("/").pop();
}

// The hero: the same scene, with the notch demo running inside it.
const heroScene = $("[data-scene]");
if (heroScene) {
  const canvas = heroScene.querySelector("canvas");
  const scene = createScene({
    canvas,
    works: onPath(5),
    mode: heroScene.dataset.scene || "home",
  });
  scene.select(0);

  // The notch demo: listening, opening a real-looking work, then done.
  const notch = $("[data-notch]");
  const text = notch?.querySelector(".tx");
  const steps = [
    ["listening", "Listening for “Hey Evo”"],
    ["opening", "Opening Q3 roadmap"],
    ["done", "Done"],
  ];
  if (notch && text) {
    let i = 0;
    const tick = () => {
      const [state, line] = steps[i % steps.length];
      notch.dataset.state = state;
      text.textContent = line;
      i += 1;
    };
    tick();
    setInterval(tick, 2600);
  }
}

// 404: one lost object on the path into the trees.
const lost = $("[data-lost]");
if (lost) {
  const scene = createScene({
    canvas: lost.querySelector("canvas"),
    works: onPath(1).map((w) => ({ ...w, x: 150, y: 158, u: 2 })),
    mode: "home",
  });
  scene.select(0);
}
