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

    /// Absorb one user claim into the evidence, dedupe-safe. Returns
    /// whether the surface actually changed.
    pub fn absorb_claim(&mut self, claim: &PodClaim) -> bool {
        match claim {
            PodClaim::Document(doc) if !self.documents.contains(doc) => {
                self.documents.push(doc.clone());
                true
            }
            PodClaim::Title(title) if !self.titles.contains(title) => {
                self.titles.push(title.clone());
                true
            }
            _ => false,
        }
    }
}

/// One witnessed, still-open window the person said belongs to this work
/// ("Add to Pod — open windows now"). Claims are ours, not the engine's:
/// they merge into the matching evidence immediately and are persisted in
/// the addon ledger so an engine refresh never silently un-owns them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PodClaim {
    /// The OS reported a document (path/URL) for the window.
    Document(String),
    /// Only the window title was knowable.
    Title(String),
}

impl PodClaim {
    pub fn subject(&self) -> &str {
        match self {
            PodClaim::Document(v) | PodClaim::Title(v) => v,
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            PodClaim::Document(_) => "claimed document",
            PodClaim::Title(_) => "claimed window",
        }
    }
}

/// One named thing a person added so the pod can re-open it ("Add to
/// Pod — open into this pod"): an app, a file, a folder, or a URL. No
/// app is ever named in code; what the person added is what opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PodResource {
    App(String),
    File(String),
    Folder(String),
    Url(String),
}

impl PodResource {
    pub fn value(&self) -> &str {
        match self {
            PodResource::App(v)
            | PodResource::File(v)
            | PodResource::Folder(v)
            | PodResource::Url(v) => v,
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            PodResource::App(_) => "app",
            PodResource::File(_) => "file",
            PodResource::Folder(_) => "folder",
            PodResource::Url(_) => "link",
        }
    }

    /// The one-command open, always via /usr/bin/open — the OS resolves
    /// the handler for files/folders/URLs; `open -a` by name (never a
    /// second instance) for apps.
    pub fn open_spec(&self) -> crate::surface::CommandSpec {
        let args = match self {
            PodResource::App(name) => vec!["-a".to_string(), name.clone()],
            other => vec![other.value().to_string()],
        };
        crate::surface::CommandSpec {
            program: "/usr/bin/open".into(),
            args,
        }
    }
}

/// The addon ledger: user-owned claims + resources per pod, line-based
/// (`pod-<id>\t<entry>\t<kind>\t<value>`), in the same spirit as
/// `stage::recipe_store` (engine data is read; what the person added is
/// remembered). Values are sanitized to single lines.
pub mod addon_store {
    use super::{PodClaim, PodResource};

    pub fn export(pairs: &[(String, Vec<PodResource>, Vec<PodClaim>)]) -> String {
        let mut out = String::new();
        for (pod_label, resources, claims) in pairs {
            for resource in resources {
                let kind = match resource {
                    PodResource::App(_) => "app",
                    PodResource::File(_) => "file",
                    PodResource::Folder(_) => "folder",
                    PodResource::Url(_) => "url",
                };
                out.push_str(&format!(
                    "{pod_label}\tresource\t{kind}\t{}\n",
                    sanitize(resource.value())
                ));
            }
            for claim in claims {
                let kind = match claim {
                    PodClaim::Document(_) => "doc",
                    PodClaim::Title(_) => "title",
                };
                out.push_str(&format!(
                    "{pod_label}\tclaim\t{kind}\t{}\n",
                    sanitize(claim.subject())
                ));
            }
        }
        out
    }

    pub fn import(text: &str) -> Vec<(u64, Vec<PodResource>, Vec<PodClaim>)> {
        let mut map: std::collections::BTreeMap<u64, (Vec<PodResource>, Vec<PodClaim>)> =
            std::collections::BTreeMap::new();
        for line in text.lines() {
            let mut parts = line.splitn(4, '\t');
            let (Some(label), Some(entry), Some(kind), Some(value)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let Some(id) = label
                .strip_prefix("pod-")
                .and_then(|s| s.parse::<u64>().ok())
            else {
                continue;
            };
            if value.trim().is_empty() {
                continue;
            }
            let slot = map.entry(id).or_default();
            match (entry, kind) {
                ("resource", "app") => push_unique(&mut slot.0, PodResource::App(value.into())),
                ("resource", "file") => push_unique(&mut slot.0, PodResource::File(value.into())),
                ("resource", "folder") => {
                    push_unique(&mut slot.0, PodResource::Folder(value.into()))
                }
                ("resource", "url") => push_unique(&mut slot.0, PodResource::Url(value.into())),
                ("claim", "doc") => push_unique(&mut slot.1, PodClaim::Document(value.into())),
                ("claim", "title") => push_unique(&mut slot.1, PodClaim::Title(value.into())),
                _ => {}
            }
        }
        map.into_iter()
            .map(|(id, (resources, claims))| (id, resources, claims))
            .collect()
    }

    fn push_unique<T: PartialEq>(vec: &mut Vec<T>, item: T) {
        if !vec.contains(&item) {
            vec.push(item);
        }
    }

    fn sanitize(value: &str) -> String {
        value.replace(['\t', '\n', '\r'], " ").trim().to_string()
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
    /// "Add to Pod" resources: named things the pod can re-open.
    pub resources: Vec<PodResource>,
    /// "Add to Pod" claims: windows the person said are ours.
    pub claims: Vec<PodClaim>,
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
            resources: Vec::new(),
            claims: Vec::new(),
            state: PodState::Inactive,
        }
    }

    pub fn badge_count(&self, kind: BadgeKind) -> usize {
        self.badges.iter().filter(|b| b.kind == kind).count()
    }
}
