// Every string for Help, Diagnostics, the tray and the state panels.
export const HELP = {
  title: "Help",
  search: "Search the guide",
  guide: [
    ["How do I get back to something?", "Say “Hey Evo” and name it, or press ⌘K and search. Evo brings its window forward when the target still exists."],
    ["What are the objects on the path?", "Your recent work. The nearest is the newest; older work recedes toward the CRT."],
    ["Does Evo record my screen?", "No. Evo remembers the window you were in, not pictures of it."],
    ["Does anything leave this Mac?", "Nothing leaves this Mac. Voice and text are handled here, and there is no account."],
    ["Why is a work missing?", "If it was hidden or merged, Activity shows when and why, with Undo where possible."],
    ["How do I stop Evo listening?", "Pause everything from Activity, Settings, or the menu bar. Pausing stops microphone capture."],
    ["What does the CRT do?", "It is Evo. Click it or say “Hey Evo” and it listens."],
  ],
  tour: { label: "Replay the tour", hint: "The intro, from the start." },
  shortcuts: { label: "Keyboard shortcuts", hint: "Everything reachable without the mouse." },
  whatsNew: {
    label: "What's new",
    items: [
      ["0.1.0", "Home, the notch, search, settings, activity and this guide."],
    ],
  },
  report: { label: "Report a problem", hint: "You see exactly what is sent, and you approve it." },
  reportTitle: "Report a problem",
  reportIntro: "This is everything the report would contain. Nothing is sent until you approve it.",
  approve: "I've read this",
  send: "Send report",
  saved: "Saved on this Mac. Nothing was uploaded.",
  copy: "Copy",
};

export const SHORTCUTS = [
  ["⌘K", "Search your work"],
  ["Enter", "Open the selected work"],
  ["← →", "Move the selection"],
  ["S", "Switch Scene and List"],
  ["⌥Space", "Listen now"],
  ["⌥⌘P", "Pause everything"],
  ["?", "Open this guide"],
];

export const DIAG = {
  title: "Diagnostics",
  mic: "Microphone",
  permissions: "Permissions",
  voice: "Voice engine",
  storage: "Storage",
  paint: "Startup",
  memory: "Memory",
  fps: "Frame rate",
  test: "Test",
  testing: "Listening…",
  heard: "Heard something. The microphone works.",
  quiet: "Nothing heard. Check the input in System Settings, then try again.",
  refused: "The microphone is blocked. Allow it in System Settings, then try again.",
  window: "Window access",
  windowValue: "Asked by the installed app",
  engineValue: "On this Mac",
  storageValue: (works, kb) => `${works} works, ${kb} KB kept here`,
};

export const STATES = {
  empty: {
    title: "Nothing here yet.",
    body: "Open something and it will be waiting on the path.",
    action: "See how Evo works",
  },
  loading: { title: "Looking for your work…" },
  error: {
    title: "Evo couldn't read your work.",
    body: "The store didn't answer. Try again, or open a work directly.",
    action: "Try again",
  },
  offline: {
    title: "You're offline.",
    body: "Evo keeps working here. Voice and search still run on this Mac.",
    action: "Try again",
  },
  denied: {
    title: "The microphone is off.",
    body: "Allow the microphone in macOS Settings, then try again. Until then you can type or press ⌘K.",
    action: "Try again",
  },
};

export const TRAY = {
  status: (listening) => (listening ? "Listening for “Hey Evo”" : "Not listening"),
  pause: "Pause everything",
  resume: "Resume everything",
  open: "Open Evo",
  help: "Help",
  diagnostics: "Diagnostics",
  quit: "Quit Evo completely",
  quitHint: "Quit lives in the menu bar of the installed app.",
};
