//! Native work layout planning.
//!
//! Inspired by Setpiece's useful separation between a workspace (the apps
//! that belong to a work) and a layout (where those windows go). Evo does not
//! embed or proxy applications: this module only produces deterministic,
//! normalized slots for the OS window layer to apply.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Full,
    Halves,
    Thirds,
    Quads,
    MainSide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    /// Normalized coordinates in the visible display frame.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspacePlan {
    pub kind: LayoutKind,
    pub slots: Vec<Slot>,
}

/// Select a calm layout for the number of windows restored by one work.
/// Extra windows reuse the final slot rather than spilling into an ugly
/// cascade; callers can minimize or leave overflow untouched explicitly.
pub fn plan_for(window_count: usize) -> WorkspacePlan {
    let kind = match window_count {
        0 | 1 => LayoutKind::Full,
        2 => LayoutKind::Halves,
        3 => LayoutKind::MainSide,
        4..=6 => LayoutKind::Quads,
        _ => LayoutKind::Quads,
    };
    let slots = match kind {
        LayoutKind::Full => vec![Slot {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }],
        LayoutKind::Halves => (0..2)
            .map(|i| Slot {
                x: i as f32 * 0.5,
                y: 0.0,
                width: 0.5,
                height: 1.0,
            })
            .collect(),
        LayoutKind::Thirds => (0..3)
            .map(|i| Slot {
                x: i as f32 / 3.0,
                y: 0.0,
                width: 1.0 / 3.0,
                height: 1.0,
            })
            .collect(),
        LayoutKind::Quads => (0..4)
            .map(|i| Slot {
                x: (i % 2) as f32 * 0.5,
                y: (i / 2) as f32 * 0.5,
                width: 0.5,
                height: 0.5,
            })
            .collect(),
        LayoutKind::MainSide => vec![
            Slot {
                x: 0.0,
                y: 0.0,
                width: 0.68,
                height: 1.0,
            },
            Slot {
                x: 0.68,
                y: 0.0,
                width: 0.32,
                height: 1.0,
            },
        ],
    };
    WorkspacePlan { kind, slots }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_setpiece_style_layouts_without_cascades() {
        assert_eq!(plan_for(1).kind, LayoutKind::Full);
        assert_eq!(plan_for(2).slots.len(), 2);
        assert_eq!(plan_for(3).kind, LayoutKind::MainSide);
        assert_eq!(plan_for(6).slots.len(), 4);
    }

    #[test]
    fn slots_are_normalized_and_non_empty() {
        for slot in plan_for(4).slots {
            assert!(slot.x >= 0.0 && slot.y >= 0.0);
            assert!(slot.x + slot.width <= 1.0);
            assert!(slot.y + slot.height <= 1.0);
        }
    }
}
