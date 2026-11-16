//! The stage: Evo's one painted backdrop, behind every screen.
//!
//! `EXPERIENCE.md` §1: Evo lives on one painted backdrop, full-bleed behind
//! every screen — a meadow and sky by day, a pixel-art forest clearing at
//! dusk. Until the art exists this module paints the honest placeholder the
//! spec allows: a soft vertical wash plus grain, never a flat colour.
//!
//! The art files are read from `assets/stage/stage-day.jpg` and
//! `assets/stage/stage-dusk.jpg` at runtime *when they exist*. Nothing here
//! invents a picture: if the files are absent the placeholder is what she
//! sees, and the placeholder is designed rather than tolerated.

use egui::{Color32, CornerRadius, Rect};

/// Where the painted art for a mode lives, relative to the crate.
pub fn art_path(dusk: bool) -> std::path::PathBuf {
    let name = if dusk {
        "stage-dusk.jpg"
    } else {
        "stage-day.jpg"
    };
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("stage")
        .join(name)
}

/// Whether the painted art for this mode is present on disk.
///
/// The caller may use this to state, honestly, that the placeholder is
/// showing. It is deliberately cheap: one `exists` per screen, not per frame.
pub fn art_present(dusk: bool) -> bool {
    art_path(dusk).is_file()
}

/// Paints the stage into `rect`. Call once, before any content.
///
/// Three layers, in order: the vertical wash (light from above), a warm
/// bloom near the top centre where the eyed light of the clearing falls,
/// and a sparse deterministic grain so no band edge is ever visible. The
/// grain is hashed from position, not random, so it does not crawl.
pub fn paint(painter: &egui::Painter, rect: Rect, dusk: bool) {
    let top = if dusk {
        Color32::from_rgb(0x0A, 0x12, 0x0E)
    } else {
        Color32::from_rgb(0xE6, 0xEF, 0xF4)
    };
    let bottom = if dusk {
        Color32::from_rgb(0x1D, 0x30, 0x24)
    } else {
        Color32::from_rgb(0xF7, 0xEF, 0xE1)
    };

    // The wash. 64 bands with a one-pixel overlap: at this count the steps
    // are below the eye's threshold for a band, which is what a gradient
    // primitive would have given us.
    const BANDS: usize = 64;
    let band_height = (rect.height() / BANDS as f32).max(1.0);
    for index in 0..BANDS {
        let t = index as f32 / (BANDS - 1) as f32;
        let band = Rect::from_min_size(
            egui::pos2(rect.min.x, rect.min.y + band_height * index as f32),
            egui::vec2(rect.width(), band_height + 1.0),
        );
        painter.rect_filled(band, CornerRadius::ZERO, lerp(top, bottom, ease(t)));
    }

    // The bloom: concentric translucent ellipses at the top centre, where
    // the reference puts the light. Nine rings is enough to read as a glow
    // and cheap enough to paint every frame.
    let centre = egui::pos2(rect.center().x, rect.min.y - rect.height() * 0.06);
    let max_radius = rect.width() * 0.62;
    let glow = if dusk {
        Color32::from_rgb(0x8F, 0xD9, 0xA9)
    } else {
        Color32::from_rgb(0xFF, 0xFB, 0xEC)
    };
    for ring in (1..=9).rev() {
        let fraction = ring as f32 / 9.0;
        let radius = max_radius * fraction;
        let alpha = if dusk { 5.0 } else { 9.0 } * (1.0 - fraction * 0.7);
        painter.circle_filled(
            centre,
            radius,
            Color32::from_rgba_unmultiplied(glow.r(), glow.g(), glow.b(), alpha as u8),
        );
    }

    // Grain. One speck every 22 points on a jittered grid, deterministic
    // from the cell coordinates, at an alpha low enough to be felt and not
    // seen. Day specks are darker than the ground; dusk specks are lighter,
    // because the light in the clearing is in front of the trees.
    let speck = if dusk {
        Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 11)
    } else {
        Color32::from_rgba_unmultiplied(0x2A, 0x33, 0x2C, 6)
    };
    let cell = 22.0f32;
    let cols = (rect.width() / cell).ceil() as u32;
    let rows = (rect.height() / cell).ceil() as u32;
    for row in 0..rows {
        for col in 0..cols {
            let h = hash(col, row);
            // Two thirds of cells stay empty, so the grain has air in it.
            if !h.is_multiple_of(3) {
                continue;
            }
            let jitter_x = ((h >> 8) % 100) as f32 / 100.0 * cell;
            let jitter_y = ((h >> 16) % 100) as f32 / 100.0 * cell;
            let at = egui::pos2(
                rect.min.x + col as f32 * cell + jitter_x,
                rect.min.y + row as f32 * cell + jitter_y,
            );
            if rect.contains(at) {
                painter.rect_filled(
                    Rect::from_min_size(at, egui::vec2(1.0, 1.0)),
                    CornerRadius::ZERO,
                    speck,
                );
            }
        }
    }
}

/// The soft light a card edge picks up from the stage: a thin inner rim.
/// Cards float on the art; without this they read as cut-outs.
pub fn rim(dusk: bool) -> Color32 {
    if dusk {
        Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 18)
    } else {
        Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 150)
    }
}

/// The shadow under a card. Long and soft, never a drop shadow with a hard
/// edge: the card is a sheet of paper a few centimetres above the ground.
pub fn card_shadow(dusk: bool) -> egui::Shadow {
    egui::Shadow {
        offset: [0, 22],
        blur: 56,
        spread: 0,
        color: if dusk {
            Color32::from_rgba_unmultiplied(0, 0, 0, 110)
        } else {
            Color32::from_rgba_unmultiplied(0x3A, 0x4A, 0x3E, 40)
        },
    }
}

fn ease(t: f32) -> f32 {
    // Slightly slower at the top than the bottom: the sky opens up.
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

/// A stable 64-bit hash of a cell, used for the grain only.
fn hash(x: u32, y: u32) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ (x as u64).wrapping_mul(0x1000_0000_01b3);
    h ^= (y as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_placeholder_is_never_one_colour() {
        // A flat fill is the exact failure the spec names. Two different
        // inputs must produce two different colours at the same point.
        let top = lerp(
            Color32::from_rgb(10, 18, 14),
            Color32::from_rgb(23, 39, 29),
            0.0,
        );
        let bottom = lerp(
            Color32::from_rgb(10, 18, 14),
            Color32::from_rgb(23, 39, 29),
            1.0,
        );
        assert_ne!(top, bottom);
        assert!(bottom.g() > top.g());
    }

    #[test]
    fn grain_is_deterministic() {
        assert_eq!(hash(3, 7), hash(3, 7));
        assert_ne!(hash(3, 7), hash(7, 3));
    }
}
