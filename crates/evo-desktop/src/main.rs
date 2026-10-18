//! Evo desktop shell entry point.
//!
//! Launches a real macOS-native Evo window. No browser is involved.

use evo_desktop::app::EvoApp;
use evo_desktop::theme;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Evo")
            .with_inner_size([980.0, 720.0])
            // The dark chrome: the environment runs edge to edge, under the
            // traffic lights, so the window is one material (DESIGN.md:
            // Environment). `with_titlebar_shown(false)` makes the bar
            // transparent in egui 0.35; combined with the full-size content
            // view the header shares its top band with the traffic lights.
            .with_titlebar_shown(false)
            .with_fullsize_content_view(true)
            // Evo's header carries the wordmark; the native title text would
            // duplicate it under the traffic lights.
            .with_title_shown(false)
            // The declared floor is the one the layout is actually designed to
            // degrade to: below `theme::STACK_BREAKPOINT` the detail view stacks
            // its contextual column under the main column instead of splitting.
            // Stating a wider minimum than that would deny the user a narrowness
            // the design already handles.
            .with_min_inner_size([theme::MIN_WINDOW_WIDTH, 520.0]),
        ..Default::default()
    };
    eframe::run_native("Evo", options, Box::new(|cc| Ok(Box::new(EvoApp::new(cc)))))
}
