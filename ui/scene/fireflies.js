// Fireflies over the grass. Moved from mockup-home-v3.html, unchanged.

import { W, px } from "./stage.js";

export const COUNT = 18;

export function createFireflies(next) {
  const flies = Array.from({ length: COUNT }, () => ({
    x: next() * W,
    y: 112 + next() * 60,
    vx: (next() - 0.5) * 0.3,
    vy: (next() - 0.5) * 0.2,
    p: next() * 9,
  }));

  return {
    step() {
      for (const o of flies) {
        o.x += o.vx;
        o.y += o.vy;
        o.p += 0.1;
        if (o.x < 0 || o.x > W) o.vx *= -1;
        if (o.y < 108 || o.y > 176) o.vy *= -1;
      }
    },
    draw(c) {
      for (const o of flies) {
        const al = 0.3 + 0.7 * Math.max(0, Math.sin(o.p));
        px(c, "#f4d98f", o.x - 1, o.y - 1, 3, 3, 0.12 * al);
        px(c, "#f6e6a8", o.x, o.y, 1, 1, al);
      }
    },
  };
}
