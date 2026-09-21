//! Stage 3 probe: exercise the guarded pane creation exactly as the app
//! does, outside the app, so failures are attributable.
//!
//! Run: cargo run --release -p evo-desktop --bin webview_probe

fn main() {
    println!("EVO-PROBE: hydrated surfaces for a work with 3 urls");
    let mut surfaces = evo_desktop::websurface::WebSurfaces::new();
    surfaces.hydrate(
        "Short Hike & Porter Rendezvous",
        &[
            "https://example.com".to_string(),
            "https://example.org".to_string(),
            "not-a-url.txt".to_string(),
        ],
    );
    println!("EVO-PROBE: tabs = {}", surfaces.pane_count());
    println!("EVO-PROBE: label 0 = {:?}", surfaces.tab_label(0));

    // Layout with no window: the pane creation still runs (guarded), the
    // attach is skipped because there is no content view.
    surfaces.layout_active((10.0, 10.0, 400.0, 300.0));
    println!("EVO-PROBE: layout attempted (no window) — alive");

    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("EVO-PROBE: active url = {:?}", surfaces.active_url());
    println!("EVO-PROBE: hibernating");
    surfaces.hibernate();
    println!("EVO-PROBE: done — no crash");
}
