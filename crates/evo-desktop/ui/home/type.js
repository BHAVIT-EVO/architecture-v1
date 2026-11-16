// Work type → sprite (DESIGN_LAW §3). Chat and web pages are a book, documents
// and files are paper, folders and projects are a folder, everything else a
// notebook.

const RULES = [
  ["folder", ["finder", "terminal", "xcode", "vs code", "cursor", "project", "workspace", "repository"]],
  ["paper", ["preview", "pages", "word", "docs", "sheets", "numbers", "keynote", "pdf", "document", "youtube", "netflix", "video"]],
  ["book", ["claude", "chatgpt", "gemini", "chat", "chrome", "safari", "firefox", "arc", "browser", "web"]],
];

/** The work's kind from its source and title, unless the data names one. */
export function workType({ kind, source = "", title = "" } = {}) {
  if (kind === "book" || kind === "paper" || kind === "folder" || kind === "notebook") return kind;
  const haystack = `${source} ${title}`.toLowerCase();
  for (const [type, needles] of RULES) {
    if (needles.some((n) => haystack.includes(n))) return type;
  }
  return "notebook";
}
