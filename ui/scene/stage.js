// The clearing — static layers. Moved from mockup-home-v3.html, unchanged.
// 320x180, seed 11, one pixel per unit. Nothing here is redesigned.

export const W = 320;
export const H = 180;
export const SEED = 11;

/** The mockup's generator: LCG, seed 11, consumed in one fixed order. */
export function rng(seed = SEED) {
  let s = seed;
  return () => (s = (s * 16807) % 2147483647) / 2147483647;
}

export const SKY = [
  "#04090e", "#06141b", "#09202a", "#0d2c31", "#133b37", "#1b4b3e",
  "#2a5d41", "#476b3d", "#78793f", "#b18541", "#dc9b4b",
];
export const GRASS = ["#1d4a34", "#1a4431", "#163c2a", "#123424"];
export const STONE = ["#6b7a70", "#7d8b80", "#5a685e"];

/** One lit pixel run. `a` is globalAlpha, restored to 1 afterwards. */
export function px(c, col, x, y, w = 1, h = 1, a = 1) {
  c.fillStyle = col;
  c.globalAlpha = a;
  c.fillRect(Math.round(x), Math.round(y), w, h);
  c.globalAlpha = 1;
}

function pine(c, x, b, h, col) {
  for (let i = 0; i < h; i++) {
    const hw = Math.floor(i * 0.28) + (i % 4 === 3 ? 1 : 0);
    px(c, col, x - hw, b - h + i, hw * 2 + 1, 1);
  }
  px(c, col, x, b, 1, 2);
}

/** Paints every layer that never changes, and returns the live star field. */
export function paintBase(c, next) {
  c.globalAlpha = 1;
  c.clearRect(0, 0, W, H);

  // Sky: banded dusk gradient, dithered on one-pixel parity.
  for (let y = 0; y < 120; y++) {
    for (let x = 0; x < W; x++) {
      const v = (y / 119) * (SKY.length - 1);
      const i = Math.floor(v);
      const f = v - i;
      px(c, SKY[Math.min(10, i + (f > (((x + y) & 1) ? 0.35 : 0.7) ? 1 : 0))], x, y);
    }
  }

  // Moon: a crescent, bitten from the lower left.
  for (let y = -9; y <= 9; y++) {
    for (let x = -9; x <= 9; x++) {
      if (x * x + y * y <= 64 && (x + 4) * (x + 4) + (y - 3) * (y - 3) > 56) {
        px(c, "#f3eed2", 262 + x, 34 + y);
      }
    }
  }

  // Stars are generated here, after the moon: the seed order is the art.
  const stars = Array.from({ length: 60 }, () => ({
    x: Math.floor(next() * W),
    y: Math.floor(next() * 72),
    p: next() * 9,
  }));

  // Hill line behind the trees.
  for (let x = 0; x < W; x++) {
    px(c, "#10302b", x, Math.floor(108 + 6 * Math.sin(x * 0.03) + 4 * Math.sin(x * 0.07 + 1)), 1, 14);
  }

  // Distant pines on the hill.
  for (let x = -4; x < W + 8; x += 8 + Math.floor(next() * 6)) {
    pine(c, x, 124, 12 + Math.floor(next() * 10), "#0b2420");
  }

  // Grass.
  for (let y = 120; y < H; y++) {
    for (let x = 0; x < W; x++) {
      px(c, GRASS[Math.min(3, Math.floor((y - 120) / 16 + next() * 0.8))], x, y);
    }
  }
  for (let i = 0; i < 320; i++) px(c, "#33704a", Math.floor(next() * W), 121 + Math.floor(next() * 58));
  for (let i = 0; i < 40; i++) px(c, "#e8e0a0", Math.floor(next() * W), 125 + Math.floor(next() * 52));

  // The stone path: widens as it comes toward the viewer.
  for (let y = 114; y < H; y++) {
    const t = (y - 114) / 66;
    const cx = 250 - 100 * t + 8 * Math.sin(t * 5);
    const w = 6 + 30 * t;
    for (let x = Math.floor(cx - w / 2); x <= cx + w / 2; x++) {
      const edge = x <= cx - w / 2 + 1 || x >= cx + w / 2 - 1;
      const n = Math.abs((x >> 1) * 31 + (y >> 1) * 17) % 3;
      const fleck = ((y >> 2) & 1) === 0 && Math.abs(x + (y >> 2) * 3) % 7 === 0;
      px(c, edge ? "#3d4a42" : fleck ? "#46544b" : STONE[n], x, y);
    }
  }

  // Foreground pines frame the clearing.
  [[8, 150, 56], [36, 146, 42], [302, 152, 58], [318, 142, 44], [286, 138, 30]]
    .forEach(([x, b, h]) => pine(c, x, b, h, "#04120f"));

  // Light spilled on the ground, in four fading bands.
  for (let i = 0; i < 4; i++) {
    px(c, "#8fe0a8", 226 - i * 6, 116 + i * 3, 14 + i * 8, 3 - (i > 1 ? 1 : 0), 0.05);
  }

  // The CRT: housing, bezel, screen well.
  px(c, "#cfc8aa", 236, 92, 28, 22);
  px(c, "#e8e1c4", 236, 92, 28, 2);
  px(c, "#a39d84", 262, 92, 2, 22);
  px(c, "#9a947d", 244, 114, 12, 2);
  px(c, "#071512", 239, 95, 22, 15);

  return stars;
}

/** The CRT screen's inner rect, in scene units. */
export const CRT = { x: 239, y: 95, w: 22, h: 15 };

/** Five-by-three pixel glyphs; the only pixel text anywhere in Evo. */
export const GLYPH = {
  e: ["111", "101", "111", "100", "011"],
  v: ["101", "101", "101", "101", "010"],
  o: ["010", "101", "101", "101", "010"],
};

/** "evo" on the CRT screen, blinking per frame. */
export function drawWord(c, frame) {
  let x = 243;
  for (const ch of ["e", "v", "o"]) {
    GLYPH[ch].forEach((row, y) => {
      for (let k = 0; k < 3; k++) if (row[k] === "1") px(c, "#8fe0a8", x + k, 99 + y);
    });
    x += 6;
  }
}

/** The listening state: five bars rising from the screen's lower edge. */
export function drawBars(c, frame) {
  for (let i = 0; i < 5; i++) {
    const h = 2 + Math.abs(Math.sin(frame * 0.6 + i * 1.7)) * 9;
    px(c, "#8fe0a8", 241 + i * 4, 110 - Math.floor(h), 2, Math.floor(h));
  }
}
