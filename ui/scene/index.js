// The scene: base, objects, fireflies, four modes, one 12fps loop that stops
// whenever the window is hidden or unfocused (DESIGN_LAW §3).

import { W, H, paintBase, rng, drawWord, drawBars, CRT, px } from "./stage.js";
import { DIM, drawSprite, sampleWorks } from "./sprites.js";
import { createFireflies } from "./fireflies.js";

export const MODES = ["home", "quiet", "intro", "empty"];

/** Cover-crop metrics: the 320x180 canvas fills the box, centre-cropped. */
export function cover(width, height) {
  const c = Math.max(width / W, height / H);
  return { c, ox: (width - W * c) / 2, oy: (height - H * c) / 2 };
}

export function createScene({
  canvas,
  works = sampleWorks(),
  mode = "home",
  reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches,
  listening = false,
}) {
  canvas.width = W;
  canvas.height = H;
  const c = canvas.getContext("2d");
  const next = rng();
  const base = document.createElement("canvas");
  base.width = W;
  base.height = H;
  const stars = paintBase(base.getContext("2d"), next);
  const flies = createFireflies(next);

  // Sprite scratch for the intro mode's fades.
  const scratch = document.createElement("canvas");
  scratch.width = W;
  scratch.height = H;
  const sc = scratch.getContext("2d");

  let state = { mode, listening, selected: 0, frame: 0, highlight: [] };
  let timer = null;

  function drawObject(w, lift, hl, frame, alpha = 1, rise = 0) {
    if (alpha >= 1 && rise === 0) {
      drawSprite(c, w.kind, w.x, w.y, w.u, lift, hl, frame);
      return;
    }
    sc.clearRect(0, 0, W, H);
    drawSprite(sc, w.kind, w.x, w.y + rise, w.u, lift, hl, frame);
    c.globalAlpha = alpha;
    c.drawImage(scratch, 0, 0);
    c.globalAlpha = 1;
  }

  function draw(frame) {
    c.globalAlpha = 1;
    c.drawImage(base, 0, 0);

    // Stars breathe.
    for (const a of stars) {
      px(c, "#d8eadf", a.x, a.y, 1, 1, 0.35 + 0.65 * Math.max(0, Math.sin(frame * 0.07 + a.p)));
    }

    // The CRT screen.
    const glowing = state.mode === "empty";
    if (glowing) {
      const pulse = 0.18 + 0.14 * Math.sin(frame * 0.18);
      px(c, "#0f3a2e", CRT.x, CRT.y, CRT.w, CRT.h);
      for (let i = 0; i < 3; i++) {
        px(c, "#8fe0a8", CRT.x + 1, CRT.y + 1 + i * 4, CRT.w - 2, 3, pulse * (1 - i * 0.25));
      }
    } else {
      px(c, state.listening ? "#14513f" : "#0f3a2e", CRT.x, CRT.y, CRT.w, CRT.h);
    }
    px(c, "#1f6a52", CRT.x, CRT.y + (frame % 15), CRT.w, 1, 0.45);
    if (glowing) {
      // A slow halo on the ground in front of the screen.
      const halo = 0.03 + 0.03 * Math.sin(frame * 0.18);
      px(c, "#8fe0a8", 226, 116, 40, 3, halo);
    } else if (state.listening) {
      drawBars(c, frame);
    } else {
      drawWord(c, frame);
    }

    // Loading: pale outlines on the path, never a spinner (§S6).
    if (state.loading && state.mode === "home") {
      works.forEach((w, i) => {
        const u = w.u ?? 1;
        const [dw, dh] = DIM[w.kind] ?? DIM.book;
        const x = Math.round(w.x - (dw * u) / 2);
        const y = Math.round(w.y - dh * u);
        const a = 0.26 + 0.08 * Math.sin(frame * 0.4 + i * 0.9);
        px(c, "#a9b8ab", x, y, dw * u, 1, a);
        px(c, "#a9b8ab", x, y + dh * u - 1, dw * u, 1, a);
        px(c, "#a9b8ab", x, y, 1, dh * u, a);
        px(c, "#a9b8ab", x + dw * u - 1, y, 1, dh * u, a);
      });
    }

    // Objects, far to near, so the nearest overlaps the rest.
    if (!state.loading && (state.mode === "home" || state.mode === "quiet")) {
      const a = state.selected;
      if (state.mode === "home") {
        for (let i = works.length - 1; i >= 0; i--) {
          const on = i === a;
          drawObject(works[i], on ? works[i].u : 0, on, frame);
        }
        for (const i of state.highlight) {
          if (typeof works[i] === "undefined") continue;
          drawObject(works[i], 0, true, frame);
        }
      }
    } else if (state.mode === "intro") {
      // The intro's arrival: each object lands in turn, then holds long enough
      // for the film (the cycle is far longer than the intro itself).
      const period = 900;
      const step = 18;
      for (let i = works.length - 1; i >= 0; i--) {
        const local = (frame - i * step + period * 2) % period;
        const inAlpha = Math.min(1, local / 24);
        const outAlpha = local > period - 80 ? Math.max(0, (period - local) / 80) : 1;
        const alpha = Math.min(inAlpha, outAlpha);
        if (alpha <= 0) continue;
        drawObject(works[i], 0, false, frame, alpha, Math.round((1 - inAlpha) * 6));
      }
    }

    // Fireflies drift, except on the emptiest screen.
    if (state.mode !== "empty") {
      flies.step();
      flies.draw(c);
    }
  }

  // Paused while the tab is hidden or the window has lost focus. A window
  // that has not been focused yet still paints, so a fresh tab is never blank.
  function visible() {
    return !document.hidden;
  }

  function start() {
    if (timer || reducedMotion || !visible()) return;
    timer = setInterval(() => {
      state.frame++;
      draw(state.frame);
    }, 80);
  }

  function stop() {
    if (!timer) return;
    clearInterval(timer);
    timer = null;
  }

  const onVisibility = () => (visible() ? start() : stop());
  const onFocus = () => start();
  const onBlur = () => stop();
  document.addEventListener("visibilitychange", onVisibility);
  window.addEventListener("focus", onFocus);
  window.addEventListener("blur", onBlur);

  draw(0);
  start();

  return {
    setMode(m) {
      state.mode = m;
      draw(state.frame);
    },
    setListening(v) {
      state.listening = v;
      draw(state.frame);
    },
    setLoading(v) {
      state.loading = v;
      draw(state.frame);
    },
    select(i) {
      state.selected = i;
      draw(state.frame);
    },
    /** Marks works without moving the selection (the intro's three). */
    setHighlight(indices) {
      state.highlight = indices;
      draw(state.frame);
    },
    isRunning: () => timer !== null,
    destroy() {
      stop();
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
    },
  };
}

/** Cover-crop rects for the objects and the CRT, in CSS pixels. */
export function hitRects(works, width, height) {
  const { c, ox, oy } = cover(width, height);
  const rects = works.map((w, i) => {
    const [dw, dh] = DIM[w.kind];
    const pw = dw * w.u;
    const ph = dh * w.u;
    return {
      i,
      label: w.title,
      left: ox + (w.x - pw / 2 - 2) * c,
      top: oy + (w.y - ph - 4) * c,
      width: (pw + 4) * c,
      height: (ph + 6) * c,
    };
  });
  rects.crt = {
    left: ox + 236 * c,
    top: oy + 92 * c,
    width: 28 * c,
    height: 24 * c,
  };
  return rects;
}
