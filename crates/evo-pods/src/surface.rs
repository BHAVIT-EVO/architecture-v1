//! The platform boundary a pod runtime acts through.
//!
//! Everything the live macOS implementation can do with the Accessibility
//! permission Evo already holds (no private Spaces machinery), plus the
//! veil overlay and shell launches. Both legs are mockable: the host app's
//! tests drive [`crate::runtime::PodRuntime`] against a `Box<dyn
//! PodSurface>` without a display.

/// A physical or fractional rectangle in global screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Frame {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// A frame is usable only if it has positive area on finite coordinates.
    pub fn usable(&self) -> bool {
        self.w > 0.0 && self.h > 0.0 && self.x.is_finite() && self.y.is_finite()
    }

    /// The fraction (0..=1) this frame sits inside `screen`, for anchoring
    /// a learned placement to a specific display geometry.
    pub fn fraction_in(&self, screen: &Frame) -> (f64, f64, f64, f64) {
        if !screen.usable() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        (
            (self.x - screen.x) / screen.w,
            (self.y - screen.y) / screen.h,
            (self.w / screen.w).min(1.0),
            (self.h / screen.h).min(1.0),
        )
    }
}

/// One live window of an application, as the surface reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct PodWindow {
    /// Owning application pid.
    pub pid: i32,
    /// Owning application localized name.
    pub owner: String,
    /// The window's exact title ("" when the OS exposes none).
    pub title: String,
    /// The OS-reported `AXDocument`: a URL for browsers (the current tab),
    /// a file URL or path for editors, absent for others.
    pub ax_document: Option<String>,
    /// The system window id (stable across the window's life).
    pub window_id: u32,
    /// Whether the window is currently minimized.
    pub minimized: bool,
    /// Global screen frame.
    pub frame: Frame,
}

/// One running application, as the surface reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodApp {
    pub pid: i32,
    /// Localized application name (what the work's `apps` list records).
    pub name: String,
    /// Whether the application is currently hidden.
    pub hidden: bool,
    /// Regular GUI application: the dock and system agents are never
    /// touched, they belong to no one's work.
    pub regular: bool,
}

/// A launch the platform should perform (e.g. `open -na ...` under a
/// wrapper). Pure value: policy decided it; the platform only executes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

/// The ops a Pod runtime needs. Numbers match the platform's actual
/// vocabulary: pids, system window ids, global frames.
pub trait PodSurface {
    /// The running applications (regular GUI apps at minimum).
    fn running_apps(&self) -> Result<Vec<PodApp>, String>;
    /// All windows of one application.
    fn windows_of(&self, pid: i32) -> Result<Vec<PodWindow>, String>;
    /// Global arranged screen frames, main first.
    fn screens(&self) -> Vec<Frame>;

    /// Hides an application (state preserved, instantly reversible).
    fn hide_app(&mut self, pid: i32) -> Result<(), String>;
    /// Unhides an application previously hidden by us.
    fn unhide_app(&mut self, pid: i32) -> Result<(), String>;
    /// Quits an application (mothball). Soft: the app may refuse.
    fn quit_app(&mut self, pid: i32) -> Result<(), String>;
    /// Minimizes one window (parks it).
    fn minimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String>;
    /// Restores one minimized window.
    fn unminimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String>;
    /// Raises one window (brings it to the front of its app).
    fn raise_window(&mut self, pid: i32, window_id: u32) -> Result<(), String>;
    /// Applies a global screen frame to one window (position + size).
    fn move_window(&mut self, pid: i32, window_id: u32, frame: Frame) -> Result<(), String>;
    /// Makes an application frontmost.
    fn activate_app(&mut self, pid: i32) -> Result<(), String>;

    /// Opens a translucent veil exactly over `frame`. Returns a handle.
    /// Veils ignore the mouse and are owned by us; a Lease closes them.
    fn veil_open(&mut self, frame: Frame, alpha: f32) -> Result<u64, String>;
    /// Moves an open veil (foreign window dragged elsewhere).
    fn veil_update(&mut self, handle: u64, frame: Frame) -> Result<(), String>;
    /// Closes a veil we opened.
    fn veil_close(&mut self, handle: u64) -> Result<(), String>;

    /// Executes a shell command (instance launches, codesign). Detached:
    /// does not wait.
    fn run(&mut self, cmd: CommandSpec) -> Result<(), String>;

    /// Test/diagnostic hook: a surface may expose its recent verb log.
    /// Real surfaces have nothing to report; fakes return their record.
    fn debug_log(&self) -> Vec<String> {
        Vec::new()
    }
}
