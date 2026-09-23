//! Shared fixtures: a fake pod and a fake desktop surface.

#![allow(dead_code)]

use evo_pods::pod::{Pod, PodSurfaces};
use evo_pods::surface::{Frame, PodApp, PodSurface, PodWindow};
use evo_threads::ResumeBundle;

pub fn app(pid: i32, name: &str, hidden: bool) -> PodApp {
    PodApp {
        pid,
        name: name.to_string(),
        hidden,
        regular: true,
    }
}

pub fn window(pid: i32, window_id: u32, title: &str, ax_document: Option<&str>) -> PodWindow {
    window_with_frame(pid, window_id, title, ax_document, Frame::new(0.0, 0.0, 800.0, 600.0))
}

pub fn window_with_frame(
    pid: i32,
    window_id: u32,
    title: &str,
    ax_document: Option<&str>,
    frame: Frame,
) -> PodWindow {
    PodWindow {
        pid,
        owner: format!("app-{pid}"),
        title: title.to_string(),
        ax_document: ax_document.map(str::to_string),
        window_id,
        minimized: false,
        frame,
    }
}

/// One pod with a hero document, a satellite URL, a companion, an anchor,
/// and a witnessed app.
pub fn pod_fixture() -> Pod {
    let view = view_fixture();
    let bundle = ResumeBundle {
        resume_point: "file:///repo/main.ts".into(),
        resume_reason: "Last edited main.ts".into(),
        restore_set: vec!["https://docs.example/api".into()],
        open_deltas: vec![],
    };
    let surfaces = PodSurfaces {
        urls: vec!["https://docs.example/api".into()],
        documents: vec!["file:///repo/main.ts".into()],
        titles: vec![],
        apps: vec!["Editor".into()],
        resource_apps: Default::default(),
    };
    Pod::from_views(&view, &bundle, "work", "reason", 10.0, vec![], surfaces)
}

pub fn view_fixture() -> evo_threads::ThreadView {
    evo_threads::ThreadView {
        id: 7,
        status: evo_threads::Status::Active,
        episodes: vec![],
        anchors: vec![evo_threads::Anchor {
            resource: "file:///repo/main.ts".into(),
            kind: evo_threads::AnchorKind::Mutation,
            strength: 1.0,
        }],
        companions: vec![("ref-notes.md".into(), 50_000.0)],
        contested: vec![],
        fork: None,
    }
}

/// A drive-by fake surface: records every verb in call order.
pub struct FakeSurface {
    pub apps: Vec<PodApp>,
    pub windows: Vec<PodWindow>,
    pub screen: Frame,
    pub log: Vec<String>,
    pub veil_count: u64,
}

impl Default for FakeSurface {
    fn default() -> Self {
        Self::base()
    }
}

impl FakeSurface {
    pub fn base() -> Self {
        Self {
            apps: vec![],
            windows: vec![],
            screen: Frame::new(0.0, 0.0, 1000.0, 800.0),
            log: vec![],
            veil_count: 0,
        }
    }
}

impl PodSurface for FakeSurface {
    fn running_apps(&self) -> Result<Vec<PodApp>, String> {
        Ok(self.apps.clone())
    }
    fn windows_of(&self, pid: i32) -> Result<Vec<PodWindow>, String> {
        Ok(self.windows.iter().filter(|w| w.pid == pid).cloned().collect())
    }
    fn screens(&self) -> Vec<Frame> {
        vec![self.screen]
    }
    fn hide_app(&mut self, pid: i32) -> Result<(), String> {
        self.log.push(format!("hide:{pid}"));
        Ok(())
    }
    fn unhide_app(&mut self, pid: i32) -> Result<(), String> {
        self.log.push(format!("unhide:{pid}"));
        Ok(())
    }
    fn quit_app(&mut self, pid: i32) -> Result<(), String> {
        self.log.push(format!("quit:{pid}"));
        Ok(())
    }
    fn minimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
        self.log.push(format!("park:{pid}:{window_id}"));
        Ok(())
    }
    fn unminimize_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
        self.log.push(format!("unpark:{pid}:{window_id}"));
        Ok(())
    }
    fn raise_window(&mut self, pid: i32, window_id: u32) -> Result<(), String> {
        self.log.push(format!("raise:{pid}:{window_id}"));
        Ok(())
    }
    fn move_window(&mut self, pid: i32, window_id: u32, frame: Frame) -> Result<(), String> {
        self.log.push(format!("mv:{pid}:{window_id}:{}:{}", frame.x, frame.y));
        Ok(())
    }
    fn activate_app(&mut self, pid: i32) -> Result<(), String> {
        self.log.push(format!("activate:{pid}"));
        Ok(())
    }
    fn veil_open(&mut self, frame: Frame, _alpha: f32) -> Result<u64, String> {
        self.veil_count += 1;
        self.log.push(format!("veil:+:{}:{}", frame.x, frame.w));
        Ok(self.veil_count)
    }
    fn veil_update(&mut self, handle: u64, _frame: Frame) -> Result<(), String> {
        self.log.push(format!("veil:~:{handle}"));
        Ok(())
    }
    fn veil_close(&mut self, handle: u64) -> Result<(), String> {
        self.log.push(format!("veil:-:{handle}"));
        Ok(())
    }
    fn run(&mut self, cmd: evo_pods::surface::CommandSpec) -> Result<(), String> {
        self.log.push(format!("run:{}", cmd.program));
        Ok(())
    }
}
