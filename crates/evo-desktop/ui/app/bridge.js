// The only file that talks to Rust (docs/BRIDGE.md).
//
// In the app the message handler `evo` is injected by webshell.rs; every
// command goes through it, each reply is matched by request id, and nothing
// else in the UI knows how the bridge works.
//
// In a normal browser there is no handler. Then — and only then — this file
// serves the local sample corpus so the page still shows something. The app
// build fails if that corpus is bundled (scripts/build-app.sh).

const native = window.webkit?.messageHandlers?.evo;
export const isApp = Boolean(native);
const appScheme = window.location?.protocol === "evo-app:";

if (appScheme && !native) {
  throw new Error("Evo could not connect to its native bridge. Quit and reopen Evo.");
}

let nextId = 1;
const pending = new Map();
// Reading the engine replays its log, and on a long history that is slow;
// refusing to wait would turn a slow read into a false "store didn't answer".
const TIMEOUT_MS = 300000;

function send(cmd, args = {}) {
  if (!native) {
    if (appScheme) {
      return Promise.reject(
        new Error("Evo could not connect to its native bridge. Quit and reopen Evo."),
      );
    }
    return mock(cmd, args);
  }
  const id = nextId++;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      if (!pending.has(id)) return;
      pending.delete(id);
      reject(Object.assign(new Error("Evo did not answer."), { code: "timeout" }));
    }, TIMEOUT_MS);
    pending.set(id, {
      resolve: (data) => {
        clearTimeout(timer);
        resolve(data);
      },
      reject: (error) => {
        clearTimeout(timer);
        reject(error);
      },
    });
    // Posted as a JSON string: WebKit hands dictionaries over as
    // NSDictionary, and the handler reads exactly one string.
    native.postMessage(JSON.stringify({ id, cmd, ...args }));
  });
}

// Rust calls this once per reply. Unknown ids are ignored: a reply that
// arrives after its timeout must not resurrect a settled promise.
window.__evoReply = (message) => {
  const entry = pending.get(message?.id);
  if (!entry) return;
  pending.delete(message.id);
  if (message.ok) entry.resolve(message.data);
  else
    entry.reject(
      Object.assign(new Error(message.error?.message ?? "Evo refused that."), {
        code: message.error?.code ?? "unknown",
      })
    );
};

// Events Rust may push (works changed, listening, paused). The page never
// depends on them to be correct about permissions: Rust refuses anyway.
window.__evoEvent = (event) => {
  window.dispatchEvent(new CustomEvent("evo-event", { detail: event }));
};

let samples = null;
async function mockRows() {
  if (samples) return samples;
  const response = await fetch("../data/works.json");
  if (!response.ok) throw new Error(`sample corpus: ${response.status}`);
  samples = await response.json();
  return samples;
}

async function mock(cmd, args) {
  if (cmd === "get_privacy")
    return { mode: "risky", paused: false, scopes: [], activity: [], storage: { path: "browser", bytes: 0 } };
  if (cmd === "get_diagnostics")
    return {
      log: { path: "browser", files: 0, bytes: 0, crashes: 0 },
      daemon: { running: false, label: "Not in the app" },
      ledger: { present: false, bytes: 0 },
      storage: { path: "browser", bytes: 0 },
      works: 0,
      version: "browser",
      build_time: "mock",
      commit: "mock",
    };
  if (cmd === "get_settings") return { onboarding_step: 0, appearance: "dusk" };
  if (cmd === "set_settings") return {
    onboarding_step: args.onboarding_step ?? 0,
    appearance: args.appearance ?? "dusk",
    keep_background: args.keep_background ?? false,
    quiet_desktop: args.quiet_desktop ?? false,
  };
  if (cmd === "get_works") {
    const rows = await mockRows().catch(() => []);
    return { works: rows.map((row, i) => ({ id: i + 1, ...row, resumable: true })), total: rows.length };
  }
  if (cmd === "get_about") return { version: "browser", build_time: "mock", commit: "mock" };
  if (cmd === "get_status") return { paused: false, capture: "mock", listening: false };
  if (cmd === "search") {
    const rows = await mockRows().catch(() => []);
    const q = args.query.toLowerCase();
    return {
      results: rows
        .filter((row) => String(row.raw ?? "").toLowerCase().includes(q))
        .map((row, i) => ({ id: i + 1, title: row.raw, score: 0, matched: "title", start_ms: 0 })),
      query: args.query,
    };
  }
  throw Object.assign(new Error("That needs the Evo app."), { code: "browser_only" });
}

export const api = {
  getWorks: () => send("get_works"),
  continueWork: (work, confirm = false) => send("continue_work", { work, confirm }),
  putThingsBack: () => send("put_things_back"),
  renameWork: (work, title) => send("rename_work", { work, title }),
  mergeWorks: (first, second) => send("merge_works", { first, second }),
  hideWork: (work, undo = false) => send("hide_work", { work, undo }),
  search: (query) => send("search", { query }),
  status: () => send("get_status"),
  voiceEvents: () => send("get_voice_events"),
  pause: () => send("pause"),
  resume: () => send("resume"),
  about: () => send("get_about"),
  privacy: () => send("get_privacy"),
  setMode: (mode) => send("set_permission_mode", { mode }),
  setScope: (kind, value, allowed) => send("set_scope", { kind, value, allowed }),
  undoActivity: (at) => send("undo_activity", { at }),
  diagnostics: () => send("get_diagnostics"),
  settings: () => send("get_settings"),
  setSettings: (patch) => send("set_settings", patch),
  quit: () => send("quit"),
  exportData: () => send("export_data"),
  deleteAll: () => send("delete_all"),
};

if (native) {
  let seenVoiceEvents = 0;
  const pollVoiceEvents = async () => {
    try {
      const { events = [] } = await api.voiceEvents();
      for (const event of events.slice(seenVoiceEvents)) {
        window.dispatchEvent(new CustomEvent("evo-voice-event", { detail: event }));
      }
      seenVoiceEvents = events.length;
    } catch {
      // The next poll retries; Rust remains the source of truth.
    }
  };
  setInterval(pollVoiceEvents, 250);
  pollVoiceEvents();
}
