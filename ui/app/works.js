// Work projection for Home: clean titles, order by recency, one selected work,
// and no work in two places at once.

import { cleanTitle } from "../home/title.js";
import { workType } from "../home/type.js";
import { at, unit, TS } from "../scene/sprites.js";

const SCENE_SLOTS = TS.length;

export function normalize(row, index) {
  const { title, source } = cleanTitle(row.raw ?? row.title ?? "", {
    app: row.app,
    audio: row.audio,
  });
  return {
    id: row.id ?? `work-${index}`,
    title,
    source,
    opened: row.opened
      ? Date.parse(row.opened)
      : Date.now() - (row.openedAgo ?? index * 60) * 60_000,
    kind: workType({ kind: row.kind, source, title }),
    merged: Boolean(row.merged),
  };
}

/** Newest first, one entry per work, nothing twice. */
export function projection(rows) {
  const seen = new Set();
  const keyed = new Set();
  const works = rows
    .map(normalize)
    .filter((w) => {
      const key = `${w.title}|${w.source}`;
      if (seen.has(w.id) || keyed.has(key) || w.merged) return false;
      seen.add(w.id);
      keyed.add(key);
      return true;
    })
    .sort((a, b) => b.opened - a.opened);

  const selected = works[0] ?? null;
  const rest = works.slice(1);
  const sceneWorks = works.slice(0, SCENE_SLOTS).map((w, i) => {
    const p = at(TS[i]);
    return { ...w, ...p, u: unit(TS[i]) };
  });
  return { works, selected, rest, sceneWorks };
}

/** "Last open an hour ago" — the card's line, from a real timestamp. */
export function whenText(opened, now = Date.now()) {
  const minutes = Math.max(0, Math.round((now - opened) / 60_000));
  if (minutes < 2) return "Last open just now";
  if (minutes < 60) return `Last open ${minutes} minutes ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 2) return "Last open an hour ago";
  if (hours < 24) return `Last open ${hours} hours ago`;
  const days = Math.round(hours / 24);
  if (days < 2) return "Last open yesterday";
  if (days < 7) return `Last open ${days} days ago`;
  const weeks = Math.round(days / 7);
  return weeks < 2 ? "Last open a week ago" : `Last open ${weeks} weeks ago`;
}
