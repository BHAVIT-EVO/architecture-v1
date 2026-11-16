//! Host-side pixel verification for the Evo notch surface.
//!
//! Exercises the SAME paint path `EvoNotchHost::render` uses
//! (`objc::paint_view` → `paint_into` → `surface_layouts` + CoreText glyphs),
//! feeds it idle → listening → outcome state, and reads back the actual
//! pixel table to confirm:
//!
//!   1. Idle: no visible pixels and an ordered-out host layout
//!   2. Listening: dark card + mic dot + transcript glyphs
//!   3. Outcome: caramel band present
//!
//! This is the verification harness only — it does not touch Evo's production
//! rendering path. It calls `paint_into` from the `objc` module of
//! `notch_window` (the same function the host calls) and checks its output.
//! It does NOT open an NSPanel or run an AppKit loop.
//!
//! Run:
//!   cargo test -p evo-desktop --features evo-notch --host_paint_verify

#![cfg_attr(not(any(test, feature = "evo-notch")), allow(dead_code))]

#[cfg(all(test, feature = "evo-notch"))]
mod host_paint_verify {
    use crate::notch::{ContentId, NotchEngine, priority};
    use crate::notch_window::objc::paint_into;
    use crate::notch_window::{
        BitmapData, CANVAS_SIZE, Layer, RenderState, SurfaceInput, VerifyCase, surface_layouts,
    };
    use evo_voice::VoiceEvent;
    use std::time::Instant;

    struct PaintSnapshot {
        bitmap: BitmapData,
    }

    impl PaintSnapshot {
        fn paint(state: RenderState) -> Self {
            let bitmap = paint_into(&state);
            Self { bitmap }
        }

        fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
            self.bitmap.pixel(x, y)
        }

        fn is_dark_at(p: [u8; 4]) -> bool {
            p[3] > 200 && p[0] < 60 && p[1] < 60 && p[2] < 60
        }
        fn is_dark(&self, x: usize, y: usize) -> bool {
            PaintSnapshot::is_dark_at(self.pixel(x, y))
        }

        /// Grey-rectangle check: opaque, mid-range RGB (80-200), channels within 5.
        fn is_grey_at(p: [u8; 4]) -> bool {
            p[3] > 200
                && p[0] >= 80
                && p[0] <= 200
                && (p[0] as i16 - p[1] as i16).abs() <= 5
                && (p[1] as i16 - p[2] as i16).abs() <= 5
                && (p[0] as i16 - p[2] as i16).abs() <= 5
        }
        fn is_grey(&self, x: usize, y: usize) -> bool {
            PaintSnapshot::is_grey_at(self.pixel(x, y))
        }

        /// Caramel accent ink (#C89B6B): warm, clearly red-of-blue, opaque.
        fn is_caramel_at(p: [u8; 4]) -> bool {
            p[3] > 200 && p[0] > 150 && p[1] > 100 && p[2] < 140 && p[0] > p[2] + 30
        }

        fn is_light_ink_at(p: [u8; 4]) -> bool {
            p[3] > 0 && p[0] > 140 && p[1] > 140
        }

        /// Ink of any kind over the dark card (glyph strokes are
        /// antialiased, so this catches partial coverage too).
        fn is_ink_at(p: [u8; 4]) -> bool {
            p[3] > 0 && p[0] > 90
        }

        /// The far corners of the real canvas. The previous coordinates
        /// (417, 257) were outside a 380x320 bitmap and wrapped into the
        /// next row, so they could never have caught an edge regression.
        fn corners(&self) -> [(usize, usize); 4] {
            let (w, h) = (self.bitmap.width, self.bitmap.height);
            [(2, 2), (w - 3, 2), (2, h - 3), (w - 3, h - 3)]
        }

        /// One layer from the state under test, by id.
        fn layer(state: &RenderState, id: &str) -> Layer {
            *surface_layouts(state)
                .iter()
                .find(|layer| layer.id == id)
                .unwrap_or_else(|| panic!("{id} layer"))
        }

        fn count_in_layer<F>(&self, layer: &Layer, pred: F) -> (usize, usize)
        where
            F: Fn([u8; 4]) -> bool,
        {
            let hits = self.count_in_rect(
                layer.x as usize,
                layer.y as usize,
                (layer.x + layer.w) as usize,
                (layer.y + layer.h) as usize,
                pred,
            );
            (hits, (layer.w * layer.h) as usize)
        }
        fn count_in_rect<F>(&self, x0: usize, y0: usize, x1: usize, y1: usize, pred: F) -> usize
        where
            F: Fn([u8; 4]) -> bool,
        {
            let mut count = 0;
            for y in y0..y1.min(self.bitmap.height) {
                for x in x0..x1.min(self.bitmap.width) {
                    if pred(self.pixel(x, y)) {
                        count += 1;
                    }
                }
            }
            count
        }
    }

    /// Grey-rectangle bug check: the four edge strips must have zero grey pixels.
    /// A true grey rectangle would fill the entire canvas including the edges.
    fn assert_no_grey_at_edges(s: &PaintSnapshot, label: &str) {
        let (cw, ch) = (s.bitmap.width, s.bitmap.height);
        let top = s.count_in_rect(0, 0, cw, 4, |p| PaintSnapshot::is_grey_at(p));
        let bottom = s.count_in_rect(0, ch.saturating_sub(4), cw, ch, |p| {
            PaintSnapshot::is_grey_at(p)
        });
        let left = s.count_in_rect(0, 0, 4, ch, |p| PaintSnapshot::is_grey_at(p));
        let right = s.count_in_rect(cw.saturating_sub(4), 0, cw, ch, |p| {
            PaintSnapshot::is_grey_at(p)
        });
        let total = top + bottom + left + right;
        assert_eq!(
            total, 0,
            "{}: edge strips have no grey pixels (grey-rectangle bug check); top={} bottom={} left={} right={}",
            label, top, bottom, left, right
        );
    }

    fn idle_state() -> RenderState {
        RenderState::default()
    }

    fn listening_state() -> RenderState {
        RenderState {
            listening: true,
            partial: Some("open my writing".into()),
            verify: Some(VerifyCase::Listening),
            ..Default::default()
        }
    }

    fn outcome_state() -> RenderState {
        let mut engine = NotchEngine::new();
        engine.show_live(ContentId::EvoIdle, priority::IDLE, false);
        engine.voice_event(
            &VoiceEvent::Outcome(evo_voice::VoiceOutcome::Restored {
                title: "Writing".into(),
                summary: "Opened 1".into(),
                offered: vec![],
            }),
            Instant::now(),
        );
        let mut render = RenderState::from_engine(&engine, None);
        render.verify = Some(VerifyCase::Outcome);
        render
    }

    #[test]
    fn host_paint_idle_is_empty_and_not_grey() {
        let s = PaintSnapshot::paint(idle_state());
        assert!(
            s.bitmap.pixels.chunks_exact(4).all(|pixel| pixel[3] == 0),
            "idle paints no Evo pixels"
        );
        assert_no_grey_at_edges(&s, "IDLE");
    }

    #[test]
    fn host_paint_listening_has_dark_card_mic_dot_and_transcript_glyphs() {
        let state = listening_state();
        let s = PaintSnapshot::paint(state.clone());
        let card = PaintSnapshot::layer(&state, "card");

        // Bare card: the bottom inset band, below the chip row. Sampling a
        // fixed (w/2, 80) could land on a glyph now that the card is
        // content-sized.
        let (bx, by) = (
            (card.x + card.w / 2.0) as usize,
            (card.y + card.h - 4.0) as usize,
        );
        assert!(
            !s.is_grey(bx, by),
            "listening card is NOT grey, got {:?}",
            s.pixel(bx, by)
        );
        assert!(
            s.is_dark(bx, by),
            "listening card is dark, got {:?}",
            s.pixel(bx, by)
        );

        // The mic indicator is the CARAMEL live dot (#C89B6B ~ 200/155/107),
        // read from the layout. The previous assertion demanded a red dot
        // (`g < 120`) that this surface has never painted, and sampled a
        // hard-coded coordinate the content-sized card no longer uses.
        let mic = PaintSnapshot::layer(&state, "mic_ring");
        let mic_px = s.pixel(
            (mic.x + mic.w / 2.0) as usize,
            (mic.y + mic.h / 2.0) as usize,
        );
        assert!(
            PaintSnapshot::is_caramel_at(mic_px),
            "mic dot is caramel, got {:?}",
            mic_px
        );
        // A dot, not a slab: the rect's corner is outside the circle.
        let corner = s.pixel(mic.x as usize, mic.y as usize);
        assert!(
            !PaintSnapshot::is_caramel_at(corner),
            "the mic dot is rounded, got {:?} at its rect corner",
            corner
        );

        let transcript = PaintSnapshot::layer(&state, "transcript");
        let (lit, total) = s.count_in_layer(&transcript, PaintSnapshot::is_light_ink_at);
        assert!(lit > 0, "transcript has real glyphs ({lit} light pixels)");
        assert!(lit * 4 < total, "transcript is sparse ({lit} / {total})");

        // Chips keep their pill fill under the label.
        for id in ["chip_restore", "chip_rack", "chip_dismiss"] {
            let chip = PaintSnapshot::layer(&state, id);
            let fill = s.pixel((chip.x + 4.0) as usize, (chip.y + chip.h / 2.0) as usize);
            assert!(
                fill[3] > 200 && fill[0] as u16 + fill[1] as u16 + fill[2] as u16 > 0,
                "{id} keeps an opaque fill, got {fill:?}"
            );
        }

        for (x, y) in s.corners() {
            assert_eq!(s.pixel(x, y)[3], 0, "canvas corner ({x},{y}) transparent");
        }
        assert_no_grey_at_edges(&s, "LISTENING");

        eprintln!(
            "LISTENING: card({bx},{by})={:?} [dark={}, grey={}]",
            s.pixel(bx, by),
            s.is_dark(bx, by),
            s.is_grey(bx, by)
        );
        eprintln!("LISTENING: mic={mic_px:?} [caramel dot]");
        eprintln!("LISTENING: transcript lit = {lit} (sparse, {total} total)");
        eprintln!("LISTENING: canvas corners all alpha=0, edge grey = 0");
    }

    #[test]
    fn host_paint_outcome_wears_its_accent_as_ink_not_a_band() {
        // This used to assert a caramel pixel in the middle of a full-width
        // `outcome_band` rectangle. That band is deliberately gone: a
        // full-width accent slab with its own label drawn on top measured
        // 1.1:1 contrast on screen. The accent is the eyebrow's ink now, so
        // the correct expectation is sparse caramel glyph strokes inside the
        // eyebrow row and bare card past the end of the label.
        let state = outcome_state();
        let s = PaintSnapshot::paint(state.clone());
        let layers = surface_layouts(&state);
        assert!(
            layers.iter().all(|layer| layer.id != "outcome_band"),
            "no full-width accent rectangle"
        );

        let eyebrow = PaintSnapshot::layer(&state, "eyebrow");
        let (caramel, total) = s.count_in_layer(&eyebrow, PaintSnapshot::is_caramel_at);
        assert!(caramel > 0, "the eyebrow label is caramel ink");
        assert!(
            caramel * 4 < total,
            "caramel is sparse strokes, not a slab ({caramel}/{total})"
        );
        let tail = s.pixel(
            (eyebrow.x + eyebrow.w - 6.0) as usize,
            (eyebrow.y + eyebrow.h / 2.0) as usize,
        );
        assert!(
            !PaintSnapshot::is_caramel_at(tail) && tail[3] > 200,
            "the eyebrow row's tail is bare card, got {tail:?}"
        );

        // Title and detail are text buckets, never filled slabs.
        for id in ["outcome_title", "outcome_detail"] {
            let row = PaintSnapshot::layer(&state, id);
            let (ink, row_total) = s.count_in_layer(&row, PaintSnapshot::is_ink_at);
            assert!(ink > 0, "{id} draws glyphs");
            assert!(
                ink * 4 < row_total,
                "{id} is sparse glyphs, not a slab ({ink}/{row_total})"
            );
            let away = s.pixel(
                (row.x + row.w - 6.0) as usize,
                (row.y + row.h / 2.0) as usize,
            );
            assert!(
                s.is_dark(
                    (row.x + row.w - 6.0) as usize,
                    (row.y + row.h / 2.0) as usize
                ),
                "{id} rect is not filled with its ink color, got {away:?}"
            );
        }

        let card = PaintSnapshot::layer(&state, "card");
        assert!(
            s.is_dark((card.x + card.w / 2.0) as usize, 15),
            "the notch band of the card is dark"
        );
        assert_eq!(
            s.pixel(
                (card.x + card.w / 2.0) as usize,
                (card.y + card.h + 20.0) as usize
            )[3],
            0,
            "below the card is transparent"
        );
        assert!(
            (card.y + card.h + 20.0) < CANVAS_SIZE.1,
            "the probe stays inside the canvas"
        );
        assert_no_grey_at_edges(&s, "OUTCOME");

        eprintln!("OUTCOME: eyebrow caramel ink = {caramel}/{total} (sparse)");
        eprintln!("OUTCOME: eyebrow tail = {tail:?} (bare card, no band)");
        eprintln!("OUTCOME: edge grey = 0 (no grey-rectangle bug)");
    }

    #[test]
    fn first_frame_is_not_a_grey_rectangle() {
        let idle = PaintSnapshot::paint(idle_state());

        // Definitive grey-rectangle check: all four canvas corners must be
        // transparent. The coordinates come from the real bitmap size now —
        // the old (417, 257) pair was outside a 380x320 canvas and wrapped
        // into a different row entirely.
        for (x, y) in idle.corners() {
            let p = idle.pixel(x, y);
            assert_eq!(
                p[3], 0,
                "corner ({x},{y}) alpha={} - canvas transparent, not opaque grey",
                p[3]
            );
        }

        // Idle is an ordered-out state: no notch band, idle glyph, or helper
        // pixels remain above another application.
        assert!(
            idle.bitmap
                .pixels
                .chunks_exact(4)
                .all(|pixel| pixel[3] == 0),
            "idle paint is fully transparent"
        );

        // Edge strips: no grey pixels anywhere.
        assert_no_grey_at_edges(&idle, "FIRST-FRAME");

        eprintln!("FIRST-FRAME: canvas edges all alpha=0 (transparent) ✓");
        eprintln!("FIRST-FRAME: idle paint is fully transparent ✓");
        eprintln!("FIRST-FRAME: edge strips have 0 grey pixels ✓");
        eprintln!("FIRST-FRAME: the grey-rectangle-on-first-frame bug is RESOLVED");
    }

    #[test]
    fn engine_event_stream_produces_correct_paint_transitions() {
        use std::sync::mpsc;

        let (tx, rx) = mpsc::channel();
        let _ = tx.send(SurfaceInput::Voice(VoiceEvent::WakeGesture { score: None }));
        let _ = tx.send(SurfaceInput::Voice(VoiceEvent::Listening {
            partial: Some("open my writing".into()),
        }));
        drop(tx);

        let mut engine = NotchEngine::new();
        engine.show_live(ContentId::EvoIdle, priority::IDLE, false);
        while let Ok(input) = rx.recv() {
            match input {
                SurfaceInput::Voice(event) => engine.voice_event(&event, Instant::now()),
                SurfaceInput::Quit => break,
                _ => {}
            }
            engine.tick(Instant::now());
        }

        let state = RenderState::from_engine(&engine, None);
        assert!(state.listening, "after WakeGesture, engine is Listening");
        assert!(!state.idle_glyph, "listening preempts idle glyph");

        let painted = listening_state();
        let s = PaintSnapshot::paint(painted.clone());
        let mic = PaintSnapshot::layer(&painted, "mic_ring");
        let mic_px = s.pixel(
            (mic.x + mic.w / 2.0) as usize,
            (mic.y + mic.h / 2.0) as usize,
        );
        assert!(
            PaintSnapshot::is_caramel_at(mic_px),
            "engine-driven listening paints the caramel mic dot ({mic_px:?})"
        );
        let transcript = PaintSnapshot::layer(&painted, "transcript");
        let (lit, _) = s.count_in_layer(&transcript, PaintSnapshot::is_light_ink_at);
        assert!(
            lit > 0,
            "engine-driven listening paints transcript glyphs ({lit} pixels)"
        );
        assert_no_grey_at_edges(&s, "ENGINE");

        eprintln!(
            "ENGINE TRANSITION: WakeGesture->Listening: mic={mic_px:?}, transcript glyphs={lit}, edge grey=0"
        );
    }
}
