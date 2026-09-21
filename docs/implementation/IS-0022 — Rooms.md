# IS-0022 — Rooms

Status: Frozen

Depends On:

* Constitution
* Architecture
* IS-0019 — Restoration Model
* IS-0021 — Restoration Derivation

---

## 1. Purpose

This specification defines Rooms: the model by which Evo turns a work into
a *place* on the desktop, and keeps the person's web presence per work
isolated and RAM-honest.

Restoration (IS-0019) reopens a work's resources; Rooms remove the
context switch that remains after restoration — the juggling of nine
windows across three works. One action ("enter this room") makes the
work's context the whole desktop; one action ("leave") restores exactly
what entering changed.

This specification defines:

* Room semantics: enter, leave, switch, cycle;
* the lease: the honest ledger of what Enter changed;
* browser worlds: per-work isolated browser sessions (Stage 2);
* web surfaces: the work's pages hosted inside Evo's window (Stage 3);
* hibernation: the RAM contract — a work's web presence is paid for only
  while occupied;
* the runtime: where room state lives so commands never wait on a window.

This specification does not define:

* window matching evidence (the ledger's canonical page identity);
* restoration itself (IS-0019);
* workspace formation (IS-0012).

---

## 2. Rooms

A **room** is the desktop arranged for one work: every app that does not
belong to the work is hidden, every window of a work app that does not
belong to the work is parked, and the work's own windows are raised in
evidence order (document-bearing windows end frontmost).

* **Enter** establishes the room and returns a **lease**: every hide, park,
  unhide, and raise it performed.
* **Leave** replays the lease in reverse and restores the desktop exactly.
* **Switch** is leave-then-enter; the room being left parks its browsers.
* **Cycle** (⌘⇧E, the menu bar) switches to the next work by recency.

Invariants:

1. **The lease is the only truth.** Nothing is restored that Enter did
   not change; nothing Enter changed is left unrestored.
2. **Evo never hides itself.** The desktop process's pid is protected.
3. **No policing.** The room establishes; what the person opens or unhides
   afterwards is their business.
4. **A vanished work leaves its room.** If a work disappears from the
   ledger (merged, faded), its room is left honestly — containment
   without its work is a trap.

### 2.1 Platform truths (macOS)

* `NSRunningApplication.hide()`/`unhide()` are asynchronous **and lie about
  their return value** when called from a non-foreground process: the call
  reports failure while the effect lands seconds later. The lease records
  every hide *attempt*; un-hiding an app that is already visible is
  harmless, while an unrestored hide would trap an app off-screen.
* Window identity comes from the private-but-exported
  `_AXUIElementGetWindow`; windows with id 0 are unaddressable and are
  never parked or raised.
* Park (minimize) and raise are settable accessibility attributes — no
  private Spaces APIs, no SIP changes.

---

## 3. Browser worlds (Stage 2)

A **browser world** is a per-work, self-contained Chrome user-data
directory Evo owns under its storage root
(`browser-worlds/EvoWork-<identity hash>`).

* Enabled per work by the person ("isolated browser session"); off by
  default.
* On Enter, the work's web resources open in the world — separate logins,
  separate tabs, one window Evo created. The world is seeded with the
  work's display name before first launch, so Chrome labels it
  "Evo · <work>" instead of an anonymous "Person 2".
* A world is **parked** when its room is left or switched: the Chrome
  instance terminates (SIGTERM — Chrome saves its session on the way
  down), and the next Enter **resumes** it (tabs as left) rather than
  reopening stale restore sets.
* A live world is never launched twice; a world without processes parks
  as a no-op.

Invariants:

1. The person's own Chrome is never touched — no profiles inside it, no
   renames within it, no singleton-lock handoffs that swallow arguments.
2. RAM for a work's browser is paid only while its room is occupied.
3. Sessions survive: park saves, resume restores, restart keeps (the
   world directory persists).

---

## 4. Web surfaces (Stage 3)

The **room browser** hosts the active work's pages as panes inside Evo's
own window: a tab strip of the work's pages, back/forward, and one native
WKWebView pane. The person reads a work's web presence without leaving
Evo at all.

* **Lazy panes.** Tabs are URL labels; a WKWebView materializes only when
  its tab is viewed. A work with twenty-five pages costs one web process
  until more tabs are opened.
* **Per-work persistent sessions.** Each work gets a
  `WKWebsiteDataStore` keyed by a deterministic UUID derived from the
  work's identity (`dataStoreForIdentifier:`) — logins and storage
  survive Evo restarts, and no two works share a jar.
* **Hibernation.** Closing the panel, leaving the room, switching rooms,
  or hiding the window releases every pane; released panes take their
  WebContent processes with them. The data store on disk is untouched.
* **Evo hosts the work's web presence; it does not become a browser.** No
  address bar, no history UI — back/forward and the work's own tabs.

### 4.1 Platform truths (WebKit in Rust)

* WebKit throws both NSExceptions and C++ exceptions. Rust aborts if any
  foreign exception unwinds through a Rust frame, so every risky WebKit
  call runs inside a compiled `@try/@catch` shim (`objc/objc_try.mm`).
* The trampoline into the shim must be `extern "C-unwind"`: a plain
  `extern "C"` frame aborts any foreign unwind *before the shim's catch
  can take it*.
* Constructors are instance methods: `[[WKWebView alloc] initWithFrame:
  configuration:]`, `[[NSUUID alloc] initWithUUIDString:]` — the class-
  method spellings do not exist and throw.

---

## 5. The room runtime

Room state (the controller, the surfaces, the active room, the isolated
preference, the web panes) lives in one shared runtime
(`Arc<Mutex<RoomRuntime>>`), held by both the window app and the menu-bar
presence.

* **Commands execute on the main thread from wherever they come.** The
  menu-bar item and the global hotkey run Enter/Leave/Cycle directly —
  the desktop never waits for a window to paint.
* **The window never dies.** Closing it hides Evo (panes hibernate with
  it); "Open Evo" restores it through AppKit. A menu-bar app whose loop
  can lose its window is a menu-bar app whose commands stop working.
* **No repaint requests from inside menu tracking.** Requesting a
  repaint from within the menu action can dispatch an egui frame
  reentrantly into AppKit's run loop; eframe is not reentrant.
* **Never bitwise-copy shared handles out of statics.** Copying an
  `Option<Arc<_>>` out of a `static mut` (e.g. `read_volatile`) skips the
  refcount increment while the copy's drop still decrements it — every
  command leaks one decrement until the runtime is freed under the app's
  feet. Clone through a reference instead.

Invariants:

1. One name domain: the active room, the menu bar, the isolated
   preference, and the parking ledger all speak the work's *display
   name*. Mixing domains (ledger identity vs display name) makes parks
   and dots silently miss.
2. The runtime's surfaces stay parallel to the engine's thread order —
   an index into one is an index into the other, or Enter opens the
   wrong work's room.
3. A room transition from any surface leaves the same paper trail
   (pending note) the window shows at its next frame.
