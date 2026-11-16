// Work objects on the path. Moved from mockup-home-v3.html, unchanged.
// Four shapes, four sizes, one selection caret.

import { px } from "./stage.js";

export const DIM = {
  book: [7, 4],
  paper: [5, 6],
  folder: [7, 5],
  notebook: [5, 6],
};

/** Sprite for a work type (DESIGN_LAW §3). */
export function spriteFor(type) {
  switch (type) {
    case "web":
    case "chat":
      return "book";
    case "document":
    case "file":
    case "video":
      return "paper";
    case "folder":
    case "project":
      return "folder";
    default:
      return "notebook";
  }
}

/** Distance along the path, 0 (at the CRT) → 1 (nearest). */
export const TS = [0.80, 0.62, 0.48, 0.36, 0.26, 0.17];

/** Where a work sits. The near end is larger and closer to the viewer. */
export function at(t) {
  return { x: 250 - 100 * t + 8 * Math.sin(t * 5), y: 114 + 66 * t };
}

/** Nearest works are largest: three sizes only. */
export function unit(t) {
  return t > 0.7 ? 3 : t > 0.4 ? 2 : 1;
}

/**
 * One work object.
 * `lift` raises it while selected, `hl` draws the selection outline and caret.
 */
export function drawSprite(c, kind, x, y, u, lift, hl, frame, theme = { day: false, accent: "#8fe0a8" }) {
  const [dw, dh] = DIM[kind];
  const w = dw * u;
  const h = dh * u;
  const l = Math.round(x - w / 2);
  const t = Math.round(y - h - lift);
  const w2 = Math.floor(w / 2);

  px(c, "#000", l + 1, y - 1, w, 2, 0.4);
  if (hl) px(c, theme.day ? "#fff" : theme.accent, l - 1, t - 1, w + 2, h + 2);

  if (kind === "book") {
    px(c, "#e6dcbc", l, t, w2, h);
    px(c, "#f4eed8", l + w2, t, w - w2, h);
    px(c, "#7a4b2a", l + w2, t, u, h);
    for (let i = u; i < h - u; i += 2 * u) {
      px(c, "#b8ad8a", l + u, t + i, Math.max(1, w2 - 2 * u), u);
    }
  }
  if (kind === "paper") {
    px(c, "#f2efe3", l, t, w, h);
    for (let i = 2 * u; i < h - u; i += 2 * u) px(c, "#b3ad96", l + u, t + i, w - 2 * u, u);
    px(c, "#cfc9b2", l + w - u, t, u, u);
  }
  if (kind === "folder") {
    px(c, "#c2933f", l, t - u, 3 * u, u);
    px(c, "#dcae52", l, t, w, h);
    px(c, "#c2933f", l, t + h - u, w, u);
  }
  if (kind === "notebook") {
    px(c, "#3f7f9a", l, t, w, h);
    px(c, "#2c5b70", l, t, u, h);
    px(c, "#e6dcbc", l + 2 * u, t + u, 2 * u, 2 * u);
  }
  if (hl) {
    const bob = Math.round(Math.sin(frame * 0.5) * 1.2);
    const ty = t - 6 + bob;
    px(c, theme.day ? "#187a4a" : theme.accent, x - 2, ty, 5, 1);
    px(c, theme.day ? "#187a4a" : theme.accent, x - 1, ty + 1, 3, 1);
    px(c, theme.day ? "#187a4a" : theme.accent, x, ty + 2, 1, 1);
  }
}

/** The mockup's six sample works, only for the dev gallery. */
export function sampleWorks() {
  const rows = [
    { title: "Frontend prompt for product redesign", source: "A Claude chat in Chrome", when: "Last open an hour ago", icon: "chat", kind: "book" },
    { title: "Eating at every Tollywood celebrity restaurant for 24 hours", source: "YouTube in Chrome, audio playing", when: "Last open 2 hours ago", icon: "play", kind: "paper" },
    { title: "Untitled chat", source: "Claude in Chrome", when: "Last open 2 days ago", icon: "chat", kind: "folder" },
    { title: "YC application script request", source: "Claude in Chrome", when: "Last open 6 days ago", icon: "chat", kind: "notebook" },
    { title: "Watch The Family Man", source: "Chrome", when: "Last open 7 days ago", icon: "play", kind: "paper" },
    { title: "Muse", source: "Freebuff", when: "Last open 7 days ago", icon: "dir", kind: "folder" },
  ];
  return rows.map((r, i) => {
    const p = at(TS[i]);
    return { ...r, ...p, u: unit(TS[i]), t: TS[i] };
  });
}
