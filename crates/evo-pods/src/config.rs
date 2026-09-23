//! Tunables: what the pod system may spend, never what it may know.

/// How entering a pod treats windows that provably do not belong to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainMode {
    /// Translucent veil over foreign windows — present but demoted.
    /// Default: nothing disappears, the subtraction reads at a glance.
    Dim,
    /// Minimize foreign windows (IS-0022's park).
    Park,
    /// Hide whole foreign apps (IS-0022's enter). Most subtractive.
    Hide,
}

impl ContainMode {
    pub const DEFAULT: ContainMode = ContainMode::Dim;
}

/// Pod budgets and floors. Each value is a limit of what the system may
/// honestly spend — a 5-line stage is a truth statement, not a preference.
#[derive(Debug, Clone, PartialEq)]
pub struct PodConfig {
    pub contain: ContainMode,
    /// Veil opacity, 0.0..=1.0. Enough to mute, never enough to hide.
    pub veil_alpha: f32,
    /// Satellites beside the hero beyond the resume point.
    pub satellite_cap: usize,
    /// Companion slots in the rail.
    pub rail_cap: usize,
    /// Suggestion floor: minimum companion weight (attention ms) before a
    /// focused foreign window may be offered to a pod. Two minutes of a
    /// work's memory, never less.
    pub suggest_min_weight_ms: f64,
    /// >= 40 % of the active screen region -> hero when nothing is learned
    /// yet; the bar shows footprints, so a heroless pod still renders.
    pub max_hotkeys: usize,
}

impl Default for PodConfig {
    fn default() -> Self {
        Self {
            contain: ContainMode::DEFAULT,
            veil_alpha: 0.45,
            satellite_cap: 3,
            rail_cap: 4,
            suggest_min_weight_ms: 120_000.0,
            max_hotkeys: 9,
        }
    }
}
