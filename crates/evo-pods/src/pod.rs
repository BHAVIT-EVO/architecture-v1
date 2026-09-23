//! The pod: identity, honest badges, evidence surface.
//!
//! All assembly here is a pure projection of the engine's public views —
//! the pod itself is not a grouping decision; it is the thread wearing a
//! face.

use evo_threads::{Delta, ResumeBundle, ThreadView};
use std::collections::BTreeMap;

/// A pod is identified by its thread id — one engine work, one pod.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PodId(pub u64);

/// A deterministic accent: same thread, same color, forever — the visual
/// identity travels with the work, not with the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PodColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl PodColor {
    /// HSV with hue = (id * golden-angle) mod 360° → runs of sequential
    /// ids land on visibly distinct hues; nobody ever picks.
    pub fn from_thread(id: u64) -> Self {
        let h = (id as f64 * 137.507_764) % 360.0;
        let (s, v) = (0.72, 0.92);
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let (r1, g1, b1) = match (h / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = v - c;
        Self {
            r: ((r1 + m) * 255.0).round() as u8,
            g: ((g1 + m) * 255.0).round() as u8,
            b: ((b1 + m) * 255.0).round() as u8,
        }
    }
}

/// How the bar labels a looping state: a faithful projection of engine
/// deltas. There is no badge the engine didn't witness.
#[derive(Debug, Clone, PartialEq)]
pub struct PodBadge {
    pub kind: BadgeKind,
    /// The resource the delta concerns (for the tooltip).
    pub subject: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BadgeKind {
    /// Typed after the last save — bytes exist only in a buffer.
    UnsavedEdits,
    /// A finalized-looking surface changed after its last finalization.
    Uncommitted,
    /// A download the person never since attended.
    DownloadReady,
    /// Typed on, never saved: draft-shaped.
    UnfinishedDraft,
    /// Shared identity with another pod — displayed, never hidden.
    Contested,
}

impl BadgeKind {
    pub fn label(&self) -> &'static str {
        match self {
            BadgeKind::UnsavedEdits => "unsaved",
            BadgeKind::Uncommitted => "uncommitted",
            BadgeKind::DownloadReady => "download ready",
            BadgeKind::UnfinishedDraft => "unfinished",
            BadgeKind::Contested => "shared",
        }
    }

    /// Badge sort: the costliest silent states first.
    pub fn rank(&self) -> u8 {
        match self {
            BadgeKind::UnsavedEdits => 0,
            BadgeKind::UnfinishedDraft => 1,
            BadgeKind::Uncommitted => 2,
            BadgeKind::DownloadReady => 3,
            BadgeKind::Contested => 4,
        }
    }
}

/// Projects engine deltas into badges, one per (kind, subject).
pub fn badges_from_bundle(bundle: &ResumeBundle) -> Vec<PodBadge> {
    bundle
        .open_deltas
        .iter()
        .map(|d| match d {
            Delta::UnsavedEdits { resource } => PodBadge {
                kind: BadgeKind::UnsavedEdits,
                subject: resource.clone(),
            },
            Delta::UncommittedChanges { root } => PodBadge {
                kind: BadgeKind::Uncommitted,
                subject: root.clone(),
            },
            Delta::DownloadedNotYetUsed { path } => PodBadge {
                kind: BadgeKind::DownloadReady,
                subject: path.clone(),
            },
            Delta::UnfinishedDraftSurface { resource } => PodBadge {
                kind: BadgeKind::UnfinishedDraft,
                subject: resource.clone(),
            },
        })
        .collect()
}

/// The work's evidence, projected for window matching and restores.
/// Shape-identical to IS-0022's room surface: documents stronger than
/// pages stronger than titles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PodSurfaces {
    /// Raw URLs the work touched (browser tabs, agent chats).
    pub urls: Vec<String>,
    /// Document file paths/URLs the work touched (editor files).
    pub documents: Vec<String>,
    /// Window titles captured for the work.
    pub titles: Vec<String>,
    /// Application names witnessed for the work.
    pub apps: Vec<String>,
    /// The app each resource was most recently witnessed in (the work's
    /// own witness, not a registry — a miss opens via the OS handler).
    pub resource_apps: BTreeMap<String, String>,
}

impl PodSurfaces {
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
            && self.documents.is_empty()
            && self.urls.is_empty()
            && self.titles.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PodState {
    /// Off screen.
    Inactive,
    /// The desktop is arranged/veiled for this pod.
    Active,
    /// Apps quit, recipe persisted; rehydrate in one key.
    Mothballed,
}

/// The bar's unit. Carries the engine's bundle verbatim: the pod is the
/// thread's face, so it keeps the whole face.
#[derive(Debug, Clone, PartialEq)]
pub struct Pod {
    pub id: PodId,
    /// Engine-derived name (the daemon's naming contract).
    pub name: String,
    pub color: PodColor,
    /// One line the engine is honestly entitled to ("Last edited …").
    pub reason: String,
    /// Ranking love: episodes × anchors from the daemon.
    pub engagement: f64,
    /// The engine's resume bundle for this thread.
    pub bundle: ResumeBundle,
    pub surfaces: PodSurfaces,
    pub badges: Vec<PodBadge>,
    /// Member subjects (the merge-proposal vocabulary; membership only).
    pub member_subjects: Vec<String>,
    /// Companion memory (suggestions), decayed weights in ms.
    pub companions: Vec<(String, f64)>,
    /// Strongest anchor (identity carrier) — the merge-gesture citation.
    pub strongest_anchor: Option<String>,
    /// Contested shared identity, displayed — never hidden.
    pub contested: Vec<String>,
    pub state: PodState,
}

impl Pod {
    pub fn from_views(
        view: &ThreadView,
        bundle: &ResumeBundle,
        name: impl Into<String>,
        reason: impl Into<String>,
        engagement: f64,
        member_subjects: Vec<String>,
        surfaces: PodSurfaces,
    ) -> Self {
        let mut badges = badges_from_bundle(bundle);
        for contested in &view.contested {
            badges.push(PodBadge {
                kind: BadgeKind::Contested,
                subject: contested.clone(),
            });
        }
        badges.sort_by_key(|b| (b.kind.rank(), b.subject.clone()));
        badges.dedup_by(|a, b| a.kind == b.kind && a.subject == b.subject);

        Self {
            id: PodId(view.id),
            name: name.into(),
            color: PodColor::from_thread(view.id),
            reason: reason.into(),
            engagement,
            bundle: bundle.clone(),
            surfaces,
            badges,
            member_subjects,
            companions: view.companions.clone(),
            strongest_anchor: view.anchors.first().map(|a| a.resource.clone()),
            contested: view.contested.clone(),
            state: PodState::Inactive,
        }
    }

    pub fn badge_count(&self, kind: BadgeKind) -> usize {
        self.badges.iter().filter(|b| b.kind == kind).count()
    }
}
