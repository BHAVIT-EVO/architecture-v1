// Title cleaner. Browsers and apps append their own names to window titles;
// the person should see the work, not the container. The source is returned
// separately so the card can still say where it came from.

const BROWSERS = [
  ["google chrome", "Chrome"], ["chrome", "Chrome"], ["safari", "Safari"],
  ["mozilla firefox", "Firefox"], ["firefox", "Firefox"], ["arc", "Arc"],
  ["brave browser", "Brave"], ["brave", "Brave"], ["microsoft edge", "Edge"],
  ["edge", "Edge"], ["vivaldi", "Vivaldi"], ["opera", "Opera"],
];

const APPS = [
  ["youtube", "YouTube"], ["netflix", "Netflix"], ["spotify", "Spotify"],
  ["claude", "Claude"], ["chatgpt", "ChatGPT"], ["gemini", "Gemini"],
  ["notion", "Notion"], ["figma", "Figma"], ["slack", "Slack"],
  ["zoom", "Zoom"], ["linear", "Linear"], ["jira", "Jira"],
  ["github", "GitHub"], ["gmail", "Gmail"], ["google docs", "Google Docs"],
  ["google sheets", "Google Sheets"], ["visual studio code", "VS Code"],
  ["vs code", "VS Code"], ["cursor", "Cursor"], ["xcode", "Xcode"],
  ["terminal", "Terminal"], ["finder", "Finder"], ["preview", "Preview"],
  ["pages", "Pages"], ["numbers", "Numbers"], ["keynote", "Keynote"],
  ["microsoft word", "Word"], ["word", "Word"], ["freebuff", "Freebuff"],
  ["notes", "Notes"], ["mail", "Mail"], ["calendar", "Calendar"],
];

// Fragments that are never part of a title.
const NOISE = [
  /^audio playing$/i, /^playing$/i, /^and \d+ more pages?$/i,
  /^\d+ more pages?$/i, /^profile \d+$/i, /^person \d+$/i,
  /^\(\d+\)$/, /^\d+$/, /^private browsing$/i, /^incognito$/i,
  /^google search$/i, /^bing search$/i,
];

const SEPARATORS = /\s+(?:[-–—|·•]|::)\s+/;

function match(list, value) {
  const lower = String(value ?? "").toLowerCase().trim();
  if (!lower) return null;
  for (const [needle, label] of list) {
    if (lower === needle || lower.includes(needle)) return label;
  }
  return null;
}

/**
 * Splits a raw window title into the work's title and where it came from.
 * @param {string} raw
 * @param {{app?: string, audio?: boolean, browserProfile?: string}} [hint]
 * @returns {{title: string, source: string}}
 */
export function cleanTitle(raw, hint = {}) {
  const original = String(raw ?? "").replace(/\s+/g, " ").trim();
  if (!original) return { title: "Untitled work", source: source(hint, {}, false) };

  const parts = original.split(SEPARATORS).map((p) => p.trim()).filter(Boolean);
  const kept = [];
  let inner = null;
  let browser = null;
  let audio = Boolean(hint.audio);
  const browserProfile = String(hint.browserProfile ?? "").trim();

  for (let i = 0; i < parts.length; i++) {
    const part = parts[i];
    if (NOISE.some((re) => re.test(part))) {
      if (/audio|playing/i.test(part)) audio = true;
      continue;
    }
    if (browserProfile && part === browserProfile) continue;
    if (i > 0) {
      const foundBrowser = match(BROWSERS, part);
      if (foundBrowser) { browser = foundBrowser; continue; }
      const foundApp = match(APPS, part);
      if (foundApp) { if (!inner) inner = foundApp; continue; }
    }
    kept.push(part);
  }

  // A window whose whole title is just a browser name is that browser's own
  // window; the title stays, the source is still known.
  if (!browser && kept.length === 1) browser = match(BROWSERS, kept[0]);

  const title = kept.join(" — ").replace(/\s+/g, " ").trim() || "Untitled work";
  return { title, source: source(hint, { inner, browser }, audio) };
}

function source(hint, found, audio) {
  const inner = found.inner ?? match(APPS, hint.app);
  const browser = found.browser ?? match(BROWSERS, hint.app);
  const layers = [inner, browser].filter(Boolean);
  const unique = layers.filter((name, i) => layers.indexOf(name) === i);

  let value;
  if (unique.length === 0) value = hint.app ? String(hint.app) : "Unknown source";
  else if (unique.length === 1) value = unique[0];
  else value = `${unique[0]} in ${unique[1]}`;
  return audio ? `${value}, audio playing` : value;
}
