// The Continue card's text. Every value here comes from another application
// — a window title, a document name, a URL — so it is written with
// textContent and never parsed as HTML. A title that reads
// `<img src=x onerror=alert(1)>` must appear as that literal text.
//
// Kept out of home.js so it can be tested without a browser.

export function paintCard(nodes, work, whenText) {
  const { title, when, source, icon } = nodes;
  if (!work) {
    title.textContent = "Nothing here yet.";
    when.textContent = "";
    source.textContent = "Start something and Evo will keep track of it.";
    return;
  }
  when.textContent = whenText(work.opened);
  title.textContent = work.title;
  source.textContent = work.source;
  if (icon?.setAttribute) icon.setAttribute("href", `#${work.kind ?? "chat"}`);
}

// An icon is built as SVG elements, never parsed from a string: `kind` can be
// derived from a title, and a title is text from another application.
export function iconSvg(doc, kind) {
  const NS = "http://www.w3.org/2000/svg";
  const svg = doc.createElementNS(NS, "svg");
  svg.setAttribute("class", "i");
  svg.setAttribute("width", "17");
  svg.setAttribute("height", "17");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.7");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  const use = doc.createElementNS(NS, "use");
  const known = ["chat", "play", "dir", "web", "lock", "search", "gear"];
  use.setAttribute("href", `#${known.includes(kind) ? kind : "chat"}`);
  svg.append(use);
  return svg;
}

// A path icon (an X, a speaker) built the same way.
export function iconSvgPath(doc, d, size = 16) {
  const NS = "http://www.w3.org/2000/svg";
  const svg = doc.createElementNS(NS, "svg");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.8");
  svg.setAttribute("stroke-linecap", "round");
  const path = doc.createElementNS(NS, "path");
  path.setAttribute("d", d);
  svg.append(path);
  return svg;
}
