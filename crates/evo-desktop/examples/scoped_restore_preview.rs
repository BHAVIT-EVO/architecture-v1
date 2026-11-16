//! Synthetic native preview. No storage reads, daemon connection, or native
//! restoration/containment; the operation displays its exact intended request.
//! Build: cargo build --offline -p evo-desktop --example scoped_restore_preview -j1

use evo_daemon::threads::DisplayThread;
use evo_daemon::work_understanding::{WorkPhase, WorkUnderstanding};
use evo_desktop::{rack, scoped_restore, theme, ui};

struct Preview {
    works: Vec<DisplayThread>,
    scope: scoped_restore::ScopedRestoreState,
    stack_open: bool,
    merge_armed: Option<usize>,
}

fn work(id: u64, name: &str, resources: Vec<String>) -> DisplayThread {
    DisplayThread {
        thread_id: id,
        name: name.into(),
        bundle: evo_threads::ResumeBundle {
            resume_point: resources.first().cloned().unwrap_or_default(),
            resume_reason: String::new(),
            restore_set: resources,
            open_deltas: vec![],
        },
        is_work: true,
        engagement: 1.0,
        has_open_loop: false,
        urgency_reason: None,
        understanding: WorkUnderstanding {
            title: name.into(),
            phase: WorkPhase::Building,
            trail: vec![],
            completed: vec![],
            pending: vec![],
            next_step: String::new(),
            narrative: "Synthetic fixture; no personal history.".into(),
        },
        last_active_ms: None,
        member_subjects: vec![],
        skipped_members: vec![],
    }
}

impl eframe::App for Preview {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui::paint_environment(ui.painter(), ui.max_rect());
        ui::caption(
            ui,
            "Scoped restore preview — synthetic works. No socket or desktop effects.",
        );
        let request = ui::scroll(ui, |ui| {
            let mut query = String::new();
            rack::rack(
                ui,
                &self.works,
                &[],
                &mut self.stack_open,
                &mut self.merge_armed,
                &mut query,
            )
        });
        if let Some(rack::RackAction::RestoreSelected(id)) = request {
            self.scope.open_for(id, &self.works);
        }
        if scoped_restore::sheet(ui.ctx(), &mut self.scope, &self.works)
            && let Some(request) = self.scope.begin_request(&self.works)
        {
            self.scope.complete(request.id, Ok(evo_voice::VoiceOutcome::Failed {
                reason: format!("Preview only: would send RESOURCE for work {} and exactly the locator above. No request was sent.", request.thread_id),
            }));
        }
    }
}

fn main() -> eframe::Result {
    let long = format!(
        "https://example.test/research/{}",
        "long-locator-without-spaces".repeat(45)
    );
    eframe::run_native(
        "Evo — scoped restore preview",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 720.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(Preview {
                works: vec![
                    work(
                        42,
                        "Notes",
                        vec![
                            "file:///synthetic/café notes.md".into(),
                            long,
                            "https://example.test/third".into(),
                            "https://example.test/fourth".into(),
                            "https://example.test/fifth".into(),
                        ],
                    ),
                    work(7, "Empty work", vec![]),
                ],
                scope: scoped_restore::ScopedRestoreState::default(),
                stack_open: false,
                merge_armed: None,
            }))
        }),
    )
}
