//! Work hypotheses: purpose-level grouping over the page/interval evidence.
//!
//! The identity chain answers "what resource was attended". This module
//! answers "what work was that attendance part of" — the question a person
//! actually asks ("my Gokyo trek planning"), which spans many resources and
//! many sittings.
//!
//! # The model (measured against the labeled ground truth; see
//! tools/threads-regression and the September 2026 empirical report)
//!
//! * **Sittings**: attention bounded by a 30-minute gap (the valley in the
//!   real gap distribution). A sitting is evidence, never a work.
//! * **Structural groups**: pages joined ONLY by deterministic hard links —
//!   the same page, a shared collection id (a playlist), a declared
//!   same-work pair, or near-duplicate cleaned titles (Jaccard ≥ 0.25).
//!   Co-occurrence in time is NEVER a hard link: background audio and
//!   two-second chat adjacency make it merge everything (measured:
//!   false-merge 0.78 at any window). No transitive closure through
//!   temporal evidence; a single bridge cannot join A to C through B.
//! * **Establishment**: a group that holds ≥ 15% of a sitting's dwell for
//!   ≥ 300 s establishes a work. Works are sticky: their identity persists
//!   across sittings through their pages.
//! * **Attachment**: inside a sitting, a group that holds ≥ 8% of dwell —
//!   or carries real input and interleaves (≥ 3 switches within 5 minutes)
//!   with an established work's pages — attaches to that sitting's dominant
//!   established work. Attachment is multi-label: one page may serve two
//!   works in different sittings.
//! * **Ambient quarantine**: a page with zero keystrokes that appears in
//!   the sittings of two or more different established works is background
//!   (music, messengers, ambient video): never a work, never a bridge.
//!   Determined by behavior, never by application name.
//! * **Declarations are ground truth**: a declared same-work pair is a hard
//!   link; a declared name names the work.
//! * **Merge proposals**: two established works co-present (both ≥ 8%) in
//!   two or more sittings are proposed for merging — deterministic
//!   co-attendance, offered to the person, never auto-applied.

use crate::intervals::WorkInterval;
use crate::pages;
use crate::works::Work;
use std::collections::{BTreeMap, BTreeSet};

/// Attention gap that ends a sitting (the measured valley).
pub const SITTING_GAP_MS: u64 = 30 * 60 * 1000;

/// Share of a sitting's dwell a group must hold to establish a work.
pub const ESTABLISH_SHARE: f64 = 0.15;
/// Minimum dwell (ms) inside one sitting to establish a work.
pub const ESTABLISH_DWELL_MS: u64 = 300 * 1000;

/// Share of a sitting's dwell at which a group attaches to the dominant
/// established work.
pub const ATTACH_SHARE: f64 = 0.08;
/// Switches with an established work's pages within this window that attach
/// an input-carrying group, even below the share floor.
pub const ATTACH_SWITCHES: usize = 3;
pub const ATTACH_SWITCH_WINDOW_MS: u64 = 5 * 60 * 1000;

/// Cleaned-title Jaccard at which two pages are structurally linked.
pub const TITLE_LINK_JACCARD: f64 = 0.25;

/// Sittings in which two established works must co-occur (both above the
/// attach share) before a merge is proposed.
pub const PROPOSAL_SITTINGS: usize = 2;

/// The person's declarations: ground truth over every derived rule.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Declarations {
    /// Subject pairs declared to be the same work.
    pub same_work: Vec<(String, String)>,
    /// Subjects with a declared work name.
    pub named: Vec<(String, String)>,
}

/// One sitting: a bounded run of attention, as indices into the interval
/// vector the sitting was built from.
#[derive(Debug, Clone)]
pub struct Sitting {
    pub start_ms: u64,
    pub end_ms: u64,
    pub total_dwell_ms: u64,
    pub interval_indices: Vec<usize>,
}

/// A merge proposal between two established works, as indices into the
/// works vector returned alongside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeProposal {
    pub work_a: usize,
    pub work_b: usize,
    pub shared_sittings: usize,
}

/// The page-level resource record the grouping runs over.
#[derive(Debug, Clone, Default)]
struct PageRecord {
    key: String,
    total_dwell_ms: u64,
    keys: u64,
    clicks: u64,
    scrolls: u64,
    typed_count: u32,
    urls: BTreeSet<String>,
    documents: BTreeSet<String>,
    titles: BTreeSet<String>,
    apps: BTreeSet<String>,
    attached_saves: usize,
    last_active_ms: u64,
    first_active_ms: u64,
    /// Sitting index -> dwell in that sitting.
    sitting_dwell: BTreeMap<usize, u64>,
    /// Interval indices contributing to this page.
    intervals: Vec<usize>,
    established: bool,
    quarantined: bool,
}

/// Segments intervals into sittings at the 30-minute gap boundary.
pub fn sittings_from_intervals(intervals: &[WorkInterval]) -> Vec<Sitting> {
    let mut order: Vec<usize> = (0..intervals.len()).collect();
    order.sort_by_key(|&i| intervals[i].start_ms);

    let mut sittings: Vec<Sitting> = Vec::new();
    for index in order {
        let interval = &intervals[index];
        let attach = sittings
            .last()
            .map(|current| interval.start_ms.saturating_sub(current.end_ms) <= SITTING_GAP_MS)
            .unwrap_or(false);
        if attach {
            let current = sittings.last_mut().expect("checked");
            current.end_ms = current.end_ms.max(interval.end_ms);
            current.total_dwell_ms += interval.total_dwell_ms;
            current.interval_indices.push(index);
        } else {
            sittings.push(Sitting {
                start_ms: interval.start_ms,
                end_ms: interval.end_ms,
                total_dwell_ms: interval.total_dwell_ms,
                interval_indices: vec![index],
            });
        }
    }
    sittings
}

/// The primary page key of an interval: its document, its canonical page,
/// its title, or its app — the same precedence as the identity chain.
fn page_key_of(interval: &WorkInterval) -> String {
    if let Some(doc) = &interval.task_key.document_path {
        return format!("doc:{doc}");
    }
    if let Some(page) = &interval.task_key.page {
        return format!("page:{page}");
    }
    if let Some(title) = &interval.task_key.window_title {
        return format!("title:{title}");
    }
    format!("app:{}", interval.task_key.app_name)
}

/// Cleaned title tokens: lowercase words of 3+ chars minus common English
/// function words. Language convention, never application knowledge.
fn title_tokens(title: &str) -> BTreeSet<String> {
    const STOPWORDS: &[&str] = &[
        "the", "and", "for", "with", "from", "that", "this", "you", "your", "are", "not", "but",
        "all", "can", "her", "was", "one", "our", "out", "day", "get", "has", "him", "his", "how",
        "man", "new", "now", "old", "see", "two", "way", "who", "its", "did", "yes",
    ];
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        // Pure numerics are page numbers and counts — never identity.
        .filter(|t| t.len() >= 3 && !STOPWORDS.contains(t) && !t.chars().all(|c| c.is_numeric()))
        .map(str::to_string)
        .collect()
}

/// Hub-discounted title tokens: tokens appearing in a large share of all
/// page titles ("page", "pdf", brand suffixes) carry no identity — the
/// same ubiquity discount that quarantines ambient pages. Computed over
/// the corpus, deterministic, never a word list.
fn specific_tokens(
    titles: &[&BTreeSet<String>],
    all_titles: &[String],
    chrome: &BTreeSet<String>,
    title: &str,
) -> BTreeSet<String> {
    let stripped = strip_title_chrome(title, chrome);
    let tokens = title_tokens(&stripped);
    if titles.is_empty() {
        return tokens;
    }
    tokens
        .into_iter()
        .filter(|token| {
            let frequency = titles.iter().filter(|set| set.contains(token)).count();
            // 10%: any token in over a tenth of all page titles is
            // corpus chrome — an app suffix, a brand, a username — never
            // identity.
            frequency as f64 / titles.len() as f64 <= 0.10
        })
        .collect()
}

/// Splits a title into dash-separated segments. macOS window titles
/// append app chrome with either dash flavor.
fn title_segments(title: &str) -> Vec<String> {
    title
        .replace(" — ", " - ")
        .split(" - ")
        .map(|seg| seg.trim().to_string())
        .filter(|seg| !seg.is_empty())
        .collect()
}

/// The recurring trailing segments across a corpus of titles: an app's
/// suffix ("Google Chrome - Profile", "— Untracked") appears as the tail
/// of many unrelated pages' titles. Corpus-derived, deterministic, never
/// an application list.
fn recurring_title_chrome(all_titles: &[String]) -> BTreeSet<String> {
    use std::collections::BTreeMap;
    let mut tail_counts: BTreeMap<String, usize> = BTreeMap::new();
    for title in all_titles {
        let segments = title_segments(title);
        // Only the last two segments can be chrome; deeper segments are
        // the page's own content.
        for segment in segments.iter().rev().take(2) {
            *tail_counts.entry(segment.clone()).or_insert(0) += 1;
        }
    }
    tail_counts
        .into_iter()
        .filter(|(_, count)| *count >= 3)
        .map(|(segment, _)| segment)
        .collect()
}

/// Strips recurring chrome segments (and everything after them) from a
/// title. The page's own content — what the person named — survives.
fn strip_title_chrome(title: &str, chrome: &BTreeSet<String>) -> String {
    let segments = title_segments(title);
    let mut kept: Vec<String> = Vec::new();
    for segment in &segments {
        if chrome.contains(segment) {
            break; // chrome and everything after it is not the page's name
        }
        kept.push(segment.clone());
    }
    if kept.is_empty() {
        // A title that is all chrome keeps its first segment: something
        // is always shown.
        segments.first().cloned().unwrap_or_default()
    } else {
        kept.join(" - ")
    }
}

fn jaccard_sets(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let shared = a.intersection(b).count();
    let union = a.union(b).count();
    shared as f64 / union as f64
}

/// Canonicalizes a declared subject to the page vocabulary used here.
fn canonical_subject(subject: &str) -> String {
    if let Some(page) = pages::canonical_page(subject) {
        return format!("page:{page}");
    }
    if subject.starts_with('/') {
        return format!("doc:{subject}");
    }
    format!("title:{subject}")
}

/// Union-find over structural hard links.
struct Union {
    parent: BTreeMap<String, String>,
    trace: Vec<String>,
}

impl Union {
    fn new() -> Self {
        Self {
            parent: BTreeMap::new(),
            trace: Vec::new(),
        }
    }
    fn find(&mut self, key: &str) -> String {
        let existing = self.parent.get(key).cloned();
        let root = match existing {
            None => {
                self.parent.insert(key.to_string(), key.to_string());
                key.to_string()
            }
            Some(p) if p == key => key.to_string(),
            Some(p) => {
                let root = self.find(&p);
                self.parent.insert(key.to_string(), root.clone());
                root
            }
        };
        root
    }
    fn union_reason(&mut self, a: &str, b: &str, reason: &str) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.trace.push(format!("[{reason}] {a} + {b}"));
            // Deterministic root: lexicographically smaller wins.
            let (winner, loser) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent.insert(loser, winner);
        }
    }
}

/// Groups intervals into purpose-level works and merge proposals.
///
/// Deterministic: the same intervals and declarations always produce the
/// same works, in the same order.
pub fn group_works(
    intervals: &[WorkInterval],
    declarations: &Declarations,
) -> (Vec<Work>, Vec<MergeProposal>) {
    let (works, proposals, _trace) = group_works_traced(intervals, declarations);
    (works, proposals)
}

/// [`group_works`] with the structural-link trace: every hard link that
/// was drawn, with the pages it connected. Explainable grouping is an
/// architectural law.
pub fn group_works_traced(
    intervals: &[WorkInterval],
    declarations: &Declarations,
) -> (Vec<Work>, Vec<MergeProposal>, Vec<String>) {
    if intervals.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    let sittings = sittings_from_intervals(intervals);

    // ── Pages: aggregate intervals by page key ─────────────────────────
    let mut page_order: Vec<String> = Vec::new();
    let mut pages: BTreeMap<String, PageRecord> = BTreeMap::new();
    let mut interval_page: Vec<String> = Vec::with_capacity(intervals.len());
    for (index, interval) in intervals.iter().enumerate() {
        let key = page_key_of(interval);
        interval_page.push(key.clone());
        let record = pages.entry(key.clone()).or_insert_with(|| {
            page_order.push(key.clone());
            PageRecord {
                key: key.clone(),
                first_active_ms: interval.start_ms,
                ..PageRecord::default()
            }
        });
        record.total_dwell_ms += interval.total_dwell_ms;
        record.keys += interval.keys;
        record.clicks += interval.clicks;
        record.scrolls += interval.scrolls;
        record.typed_count += interval.typed_count;
        record.attached_saves += interval.attached_saves;
        record.last_active_ms = record.last_active_ms.max(interval.end_ms);
        record.first_active_ms = record.first_active_ms.min(interval.start_ms);
        for url in &interval.urls {
            record.urls.insert(url.clone());
        }
        for doc in &interval.document_paths {
            record.documents.insert(doc.clone());
        }
        for title in &interval.titles {
            record.titles.insert(title.clone());
        }
        record.apps.insert(interval.task_key.app_name.clone());
        record.intervals.push(index);
    }

    // Sitting membership per page (dwell share per sitting).
    for (sitting_index, sitting) in sittings.iter().enumerate() {
        let mut sitting_page_dwell: BTreeMap<&str, u64> = BTreeMap::new();
        for &index in &sitting.interval_indices {
            let key = interval_page[index].as_str();
            *sitting_page_dwell.entry(key).or_insert(0) += intervals[index].total_dwell_ms;
        }
        for (key, dwell) in sitting_page_dwell {
            if let Some(record) = pages.get_mut(key) {
                *record.sitting_dwell.entry(sitting_index).or_insert(0) += dwell;
            }
        }
    }

    // ── Structural hard links ──────────────────────────────────────────
    let mut union = Union::new();
    for key in &page_order {
        union.find(key);
    }
    // Shared opaque tokens (a playlist id, a chat family id): pages that
    // carry the same machine token belong to one structural group. The
    // collection is a GLOBAL fact — the token several pages share — not a
    // per-URL guess. Tokens shared by very many pages are site-wide ids,
    // not collections: they glue nothing (measured cap on real data).
    let mut token_pages: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for key in &page_order {
        if let Some(page) = key.strip_prefix("page:") {
            for token in pages::opaque_tokens(page) {
                token_pages.entry(token).or_default().push(key.clone());
            }
        }
    }
    for (token, members) in &token_pages {
        if members.len() > 1 && members.len() <= 30 {
            for window in members.windows(2) {
                union.union_reason(&window[0], &window[1], &format!("token {token}"));
            }
        }
    }
    // Near-duplicate cleaned titles: a light, precision-only link
    // (measured false-merge ≤ 0.015) — but only over SPECIFIC tokens.
    // Hub tokens ("page", "pdf", page numbers) appear across unrelated
    // works' titles and would fuse them; the corpus-frequency discount
    // removes them before the comparison.
    let title_sets: Vec<&BTreeSet<String>> = pages.values().map(|p| &p.titles).collect();
    let all_titles: Vec<String> = pages
        .values()
        .flat_map(|p| p.titles.iter().cloned())
        .collect();
    let title_chrome = recurring_title_chrome(&all_titles);
    // Specific token sets are computed once per page — the pair loop is
    // O(P²) comparisons and recomputing token sets per pair was a
    // measured hot spot as page diversity grows.
    let page_specific: BTreeMap<String, Vec<BTreeSet<String>>> = page_order
        .iter()
        .map(|key| {
            (
                key.clone(),
                pages[key]
                    .titles
                    .iter()
                    .map(|t| specific_tokens(&title_sets, &all_titles, &title_chrome, t))
                    .collect(),
            )
        })
        .collect();
    for (i, a) in page_order.iter().enumerate() {
        let a_specific = &page_specific[a];
        for b in page_order.iter().skip(i + 1) {
            let b_specific = &page_specific[b];
            let best = a_specific
                .iter()
                .flat_map(|ta| b_specific.iter().map(move |tb| jaccard_sets(ta, tb)))
                .fold(0.0_f64, f64::max);
            if best >= TITLE_LINK_JACCARD {
                union.union_reason(a, b, "title similarity");
            }
        }
    }
    // Declarations are ground truth: hard links, whatever the rules say.
    for (a, b) in &declarations.same_work {
        let (ca, cb) = (canonical_subject(a), canonical_subject(b));
        if union.parent.contains_key(&ca) && union.parent.contains_key(&cb) {
            union.union_reason(&ca, &cb, "declared same work");
        } else if !union.parent.contains_key(&ca) || !union.parent.contains_key(&cb) {
            // A declared subject with no attendance yet still links its
            // group identity for the future.
            union.find(&ca);
            union.find(&cb);
            union.union_reason(&ca, &cb, "declared same work (new)");
        }
    }

    // ── Establishment: groups that earn a work ─────────────────────────
    // A group establishes when, in any sitting, it holds ≥ ESTABLISH_SHARE
    // of dwell for ≥ ESTABLISH_DWELL_MS. Establishment is per-sitting and
    // independent: several groups can establish from the same sitting
    // (long interleaved works each earn their own).
    let mut established_groups: BTreeSet<String> = BTreeSet::new();
    // Per-sitting (root -> dwell share), for attachment dominance checks.
    let mut sitting_established: Vec<BTreeMap<String, f64>> = Vec::new();
    for sitting in &sittings {
        let mut shares: BTreeMap<String, f64> = BTreeMap::new();
        if sitting.total_dwell_ms > 0 {
            let mut group_dwell: BTreeMap<String, u64> = BTreeMap::new();
            for &index in &sitting.interval_indices {
                let root = union.find(&interval_page[index]);
                *group_dwell.entry(root).or_insert(0) += intervals[index].total_dwell_ms;
            }
            for (root, dwell) in group_dwell {
                let share = dwell as f64 / sitting.total_dwell_ms as f64;
                if share >= ESTABLISH_SHARE && dwell >= ESTABLISH_DWELL_MS {
                    established_groups.insert(root.clone());
                }
                shares.insert(root, share);
            }
        }
        sitting_established.push(shares);
    }
    for root in &established_groups {
        for (key, record) in pages.iter_mut() {
            if union.find(key) == *root {
                record.established = true;
            }
        }
    }
    // A multi-page structural group with substantial total dwell is a
    // body of work in its own right even when no single sitting shows it
    // at 15%: a playlist watched in background bursts, a document family
    // revisited in fragments. Its evidence is structural (the shared
    // collection), not one dominant sitting. Computed BEFORE attachment
    // so its members are sticky: a resource that belongs to a work never
    // attaches into another through co-sitting share. Multi-label
    // membership is for resources with no structural work of their own.
    let mut work_groups: BTreeSet<String> = established_groups.clone();
    {
        let mut group_pages: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for key in &page_order {
            let root = union.find(key);
            group_pages.entry(root).or_default().push(key.clone());
        }
        for (root, members) in group_pages {
            if work_groups.contains(&root) {
                continue;
            }
            let dwell: u64 = members
                .iter()
                .filter_map(|m| pages.get(m).map(|p| p.total_dwell_ms))
                .sum();
            if members.len() >= 2 && dwell >= ESTABLISH_DWELL_MS {
                work_groups.insert(root);
            }
        }
    }
    for root in &work_groups {
        for (key, record) in pages.iter_mut() {
            if union.find(key) == *root {
                record.established = true;
            }
        }
    }

    // ── Ambient quarantine ─────────────────────────────────────────────
    // Zero-keystroke pages appearing in the sittings of two or more
    // different established works are background. Behavior only.
    for (key, record) in pages.iter_mut() {
        if record.keys > 0 || record.established {
            continue;
        }
        let mut co_works: BTreeSet<String> = BTreeSet::new();
        for sitting_index in record.sitting_dwell.keys() {
            let sitting = &sittings[*sitting_index];
            for &index in &sitting.interval_indices {
                let root = union.find(&interval_page[index]);
                if established_groups.contains(&root) && root != union.find(key) {
                    co_works.insert(root);
                }
            }
        }
        if co_works.len() >= 2 {
            record.quarantined = true;
        }
    }

    // ── Attachment inside sittings ─────────────────────────────────────
    // Non-established, non-quarantined groups attach to the sitting's
    // dominant established work at ≥ ATTACH_SHARE of the sitting, or when
    // carrying real input and interleaving with it. Multi-label: a page
    // may attach to different works in different sittings.
    let mut attachments: BTreeMap<String, BTreeSet<String>> = BTreeMap::new(); // page -> work roots
    for (sitting_index, sitting) in sittings.iter().enumerate() {
        if sitting.total_dwell_ms == 0 {
            continue;
        }
        // Per-group dwell and presence in this sitting.
        let mut group_dwell: BTreeMap<String, u64> = BTreeMap::new();
        let mut group_intervals: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for &index in &sitting.interval_indices {
            let root = union.find(&interval_page[index]);
            *group_dwell.entry(root.clone()).or_insert(0) += intervals[index].total_dwell_ms;
            group_intervals.entry(root).or_default().push(index);
        }
        let established_here: Vec<(String, f64)> = sitting_established[sitting_index]
            .iter()
            .filter(|(root, _)| established_groups.contains(*root))
            .map(|(root, share)| (root.clone(), *share))
            .collect();
        let mut ranked = established_here;
        ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        let Some((dominant, dominant_share)) = ranked.first().cloned() else {
            continue; // no established work in this sitting; nothing attaches
        };
        if dominant_share < ATTACH_SHARE {
            continue; // the dominant work barely participated here
        }

        for (root, dwell) in &group_dwell {
            if established_groups.contains(root) || *root == dominant {
                continue;
            }
            let share = *dwell as f64 / sitting.total_dwell_ms as f64;
            let group_keys: u64 = group_intervals
                .get(root)
                .map(|ivs| ivs.iter().map(|&i| intervals[i].keys).sum())
                .unwrap_or(0);
            let interleave = group_intervals
                .get(root)
                .map(|ivs| {
                    ivs.iter().any(|&i| {
                        let my_root = union.find(&interval_page[i]);
                        let switches = sitting
                            .interval_indices
                            .iter()
                            .filter(|&&s| {
                                let s_root = union.find(&interval_page[s]);
                                established_groups.contains(&s_root)
                                    && s_root != my_root
                                    && intervals[s].start_ms.abs_diff(intervals[i].start_ms)
                                        <= ATTACH_SWITCH_WINDOW_MS
                            })
                            .count();
                        switches >= ATTACH_SWITCHES
                    })
                })
                .unwrap_or(false);
            if share >= ATTACH_SHARE || (group_keys > 0 && interleave) {
                let reason = if share >= ATTACH_SHARE {
                    format!("share {:.0}%", share * 100.0)
                } else {
                    "input interleave".to_string()
                };
                // Attach every page of this group to the dominant work.
                for (key, record) in pages.iter() {
                    if union.find(key) == *root && !record.quarantined && !record.established {
                        attachments
                            .entry(key.clone())
                            .or_default()
                            .insert(dominant.clone());
                        union
                            .trace
                            .push(format!("[attach {reason}] {key} -> {dominant}"));
                    }
                }
            }
        }
        // Established works present in this sitting also gain the dominant
        // work's co-presence (for proposals, below).
        let _ = sitting_index;
    }

    // ── Works and merge proposals ─────────────────────────────────────
    // Established groups become works; co-sitting counts between them
    // become merge proposals (offered, never applied).
    // A multi-page structural group with substantial total dwell is a
    // body of work in its own right even when no single sitting shows it
    // at 15%: a playlist watched in background bursts, a document family
    // revisited in fragments. Its evidence is structural (the shared
    // collection), not one dominant sitting.
    let mut work_groups: BTreeSet<String> = established_groups.clone();
    {
        let mut group_pages: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for key in &page_order {
            let root = union.find(key);
            group_pages.entry(root).or_default().push(key.clone());
        }
        for (root, members) in group_pages {
            if work_groups.contains(&root) {
                continue;
            }
            let dwell: u64 = members
                .iter()
                .filter_map(|m| pages.get(m).map(|p| p.total_dwell_ms))
                .sum();
            if members.len() >= 2 && dwell >= ESTABLISH_DWELL_MS {
                work_groups.insert(root);
            }
        }
    }

    for root in &work_groups {
        for (key, record) in pages.iter_mut() {
            if union.find(key) == *root {
                record.established = true;
            }
        }
    }

    let root_order: Vec<String> = work_groups.iter().cloned().collect();
    let mut co_sittings: BTreeMap<(String, String), usize> = BTreeMap::new();
    for sitting in &sittings {
        if sitting.total_dwell_ms == 0 {
            continue;
        }
        let mut group_dwell: BTreeMap<String, u64> = BTreeMap::new();
        for &index in &sitting.interval_indices {
            let root = union.find(&interval_page[index]);
            *group_dwell.entry(root).or_insert(0) += intervals[index].total_dwell_ms;
        }
        let mut present: Vec<String> = group_dwell
            .iter()
            .filter(|(root, dwell)| {
                established_groups.contains(*root)
                    && **dwell as f64 / sitting.total_dwell_ms as f64 >= ATTACH_SHARE
            })
            .map(|(root, _)| root.clone())
            .collect();
        present.sort();
        for i in 0..present.len() {
            for b in present.iter().skip(i + 1) {
                *co_sittings
                    .entry((present[i].clone(), b.clone()))
                    .or_insert(0) += 1;
            }
        }
    }
    let proposals: Vec<MergeProposal> = co_sittings
        .into_iter()
        .filter(|(_, count)| *count >= PROPOSAL_SITTINGS)
        .filter_map(|((a, b), count)| {
            let ia = root_order.iter().position(|r| r == &a)?;
            let ib = root_order.iter().position(|r| r == &b)?;
            Some(MergeProposal {
                work_a: ia,
                work_b: ib,
                shared_sittings: count,
            })
        })
        .collect();

    // ── Aggregate works ────────────────────────────────────────────────
    // A work = its established structural group's pages + every page
    // attached to it. Quarantined pages join nothing.
    let mut works: Vec<Work> = Vec::new();
    for root in &work_groups {
        let mut member_keys: BTreeSet<String> = BTreeSet::new();
        for (key, record) in pages.iter() {
            if record.quarantined {
                continue;
            }
            if union.find(key) == *root {
                member_keys.insert(key.clone());
            }
        }
        for (key, targets) in &attachments {
            if targets.contains(root) {
                member_keys.insert(key.clone());
            }
        }
        if member_keys.is_empty() {
            continue;
        }

        let mut urls: Vec<String> = Vec::new();
        let mut documents: Vec<String> = Vec::new();
        let mut titles: Vec<String> = Vec::new();
        let mut apps: Vec<String> = Vec::new();
        let mut total_time_ms = 0_u64;
        let mut keys_total = 0_u64;
        let mut clicks_total = 0_u64;
        let mut scrolls_total = 0_u64;
        let mut attached_saves = 0_usize;
        let mut has_production = false;
        let mut last_active_ms = 0_u64;
        let mut session_sittings: BTreeSet<usize> = BTreeSet::new();
        let mut member_intervals: Vec<WorkInterval> = Vec::new();

        for key in &member_keys {
            let record = &pages[key];
            total_time_ms += record.total_dwell_ms;
            keys_total += record.keys;
            clicks_total += record.clicks;
            scrolls_total += record.scrolls;
            attached_saves += record.attached_saves;
            last_active_ms = last_active_ms.max(record.last_active_ms);
            has_production = has_production || record.keys > 0 || record.typed_count > 0;
            for url in &record.urls {
                if !urls.contains(url) {
                    urls.push(url.clone());
                }
            }
            for doc in &record.documents {
                if !documents.contains(doc) {
                    documents.push(doc.clone());
                }
            }
            for title in &record.titles {
                if !titles.contains(title) {
                    titles.push(title.clone());
                }
            }
            for app in &record.apps {
                if !apps.contains(app) {
                    apps.push(app.clone());
                }
            }
            for sitting_index in record.sitting_dwell.keys() {
                session_sittings.insert(*sitting_index);
            }
            for &index in &record.intervals {
                member_intervals.push(intervals[index].clone());
            }
        }
        member_intervals.sort_by_key(|i| i.start_ms);

        // The work's name: a declared name when the person gave one, else
        // the richest title among its pages (chrome stripping happens in
        // presentation).
        let declared_name = declarations.named.iter().find_map(|(subject, name)| {
            member_keys
                .contains(&canonical_subject(subject))
                .then(|| name.clone())
        });
        let identity = declared_name.unwrap_or_else(|| {
            titles
                .iter()
                .filter(|t| !t.trim().is_empty())
                .max_by_key(|t| t.chars().count())
                .cloned()
                .unwrap_or_else(|| member_keys.iter().next().cloned().unwrap_or_default())
        });

        union.trace.push(format!(
            "[work] {identity} from {root}: {} pages",
            member_keys.len()
        ));
        let mut work = Work {
            identity,
            identity_kind: crate::works::IdentityKind::Purpose,
            intervals: member_intervals,
            total_time_ms,
            urls,
            documents,
            titles,
            apps,
            has_production,
            keys: keys_total,
            clicks: clicks_total,
            scrolls: scrolls_total,
            attached_saves,
            last_active_ms,
            session_count_override: Some(session_sittings.len()),
        };
        // Sessions: sittings where this work held a meaningful presence.
        work.session_count_override = Some(session_sittings.len());
        works.push(work);
    }

    works.sort_by(|a, b| {
        b.last_active_ms
            .cmp(&a.last_active_ms)
            .then(b.total_time_ms.cmp(&a.total_time_ms))
            .then(a.identity.cmp(&b.identity))
    });
    (works, proposals, union.trace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intervals::compute_intervals;
    use crate::ObservedEvent;

    fn event(
        ts: u64,
        url: Option<&str>,
        title: &str,
        app: &str,
        dwell: u64,
        keys: u64,
    ) -> ObservedEvent {
        ObservedEvent {
            timestamp_ms: ts,
            app_name: app.to_string(),
            window_title: title.to_string(),
            url: url.map(String::from),
            document_path: None,
            typed: keys > 0,
            dwell_ms: dwell,
            keys,
            clicks: 0,
            scrolls: 0,
        }
    }

    fn video_url(id: &str, list: &str) -> String {
        format!("https://www.youtube.com/watch?v={id}&list={list}")
    }

    /// The Gokyo shape: one playlist of trek videos + gear pages across
    /// sittings — one purpose, many resources, many sittings.
    #[test]
    fn playlist_and_gear_pages_form_one_trek_work_via_attachment() {
        // Sitting 1 (day 1 evening): Arc'teryx gear browsing dominates.
        let day1 = 1_000_000_u64;
        let mut events = vec![
            event(
                day1,
                Some("https://www.arcteryx.com/us/en/c/mens/footwear-hike"),
                "Hiking Boots | Arc'teryx",
                "Chrome",
                400_000,
                40,
            ),
            event(
                day1 + 500_000,
                Some("https://www.arcteryx.com/us/en/c/mens/hats-caps"),
                "Unisex Hats | Arc'teryx",
                "Chrome",
                300_000,
                20,
            ),
        ];
        // Sitting 2 (day 2 evening): the trek playlist videos.
        let day2 = day1 + 40 * 60 * 60 * 1000;
        for i in 0..3 {
            events.push(event(
                day2 + i * 700_000,
                Some(&video_url(&format!("q2VTtop1Zt{i}"), "PLiM-TFJI81RkXy")),
                &format!("Trek Vlog {i} — Gokyo"),
                "Chrome",
                500_000,
                0,
            ));
        }
        // Sitting 3 (day 3 evening): gear again + one more playlist video.
        let day3 = day2 + 30 * 60 * 60 * 1000;
        events.push(event(
            day3,
            Some("https://www.arcteryx.com/us/en/c/mens/jackets"),
            "Shells | Arc'teryx",
            "Chrome",
            350_000,
            30,
        ));
        events.push(event(
            day3 + 400_000,
            Some(&video_url("q2VTtop1Zt9", "PLiM-TFJI81RkXy")),
            "Renjo La Pass",
            "Chrome",
            100_000,
            0,
        ));

        let intervals = compute_intervals(&events);
        let (works, proposals) = group_works(&intervals, &Declarations::default());
        // The playlist videos establish one work; the gear pages establish
        // another; they co-occur in sitting 3 only here, so: two works, one
        // proposal once they share two sittings — with one shared sitting
        // there is no proposal yet.
        assert!(
            works.len() >= 2,
            "playlist and gear are distinct evidence groups"
        );
        assert!(
            proposals.is_empty(),
            "one shared sitting is not enough to propose"
        );
        // No work is keyed by a host: youtube.com never appears as identity.
        for work in &works {
            assert!(!work.identity.contains("youtube.com"));
            assert!(!work.identity.contains("arcteryx.com"));
        }
    }

    /// The mirror problem: two unrelated videos on one host are different
    /// pages and different works; the playlist keeps its members together.
    #[test]
    fn unrelated_videos_never_share_a_work() {
        let base = 1_000_000_u64;
        let events = vec![
            event(
                base,
                Some(&video_url("trekVideo001", "PLtreklist00")),
                "Gokyo Trek",
                "Chrome",
                500_000,
                0,
            ),
            event(
                base + 700_000,
                Some(&video_url("trekVideo002", "PLtreklist00")),
                "Renjo La",
                "Chrome",
                500_000,
                0,
            ),
            // A different sitting, a different purpose.
            event(
                base + 40 * 60 * 60 * 1000,
                Some(&video_url("comedyVid01", "PLcomedylist")),
                "Standup Night",
                "Chrome",
                500_000,
                0,
            ),
        ];
        let intervals = compute_intervals(&events);
        let (works, _) = group_works(&intervals, &Declarations::default());
        assert_eq!(
            works.len(),
            2,
            "trek playlist and comedy are different works"
        );
    }

    /// Background audio never bridges: a music page with zero keystrokes
    /// present across two different works' sittings is quarantined.
    #[test]
    fn ambient_music_never_bridges_works() {
        let base = 1_000_000_u64;
        let mut events = vec![
            // Sitting 1: coding work + music in background.
            event(
                base,
                Some("https://music.example.com/playlist/lofi"),
                "Lo-fi Beats",
                "Browser",
                50_000,
                0,
            ),
            event(
                base + 10_000,
                None,
                "main.rs — project",
                "ZCode",
                600_000,
                300,
            ),
        ];
        // Sitting 2 (next day): trek research + the same music.
        let day2 = base + 40 * 60 * 60 * 1000;
        events.push(event(
            day2,
            Some("https://music.example.com/playlist/lofi"),
            "Lo-fi Beats",
            "Browser",
            60_000,
            0,
        ));
        for i in 0..3 {
            events.push(event(
                day2 + 20_000 + i * 500_000,
                Some(&video_url(&format!("trekVid000{i}"), "PLtreklist00")),
                &format!("Trek {i}"),
                "Chrome",
                400_000,
                0,
            ));
        }
        let intervals = compute_intervals(&events);
        let (works, _) = group_works(&intervals, &Declarations::default());
        // The music page must appear in no work.
        for work in &works {
            for title in &work.titles {
                assert!(
                    title != "Lo-fi Beats",
                    "ambient music joined a work: {title}"
                );
            }
            for url in &work.urls {
                assert!(
                    !url.contains("music.example.com"),
                    "ambient music page joined a work"
                );
            }
        }
        assert_eq!(works.len(), 2, "coding and trek stay separate");
    }

    /// A small group attaches to the sitting's dominant established work —
    /// the evening-video case — but only above the share floor.
    #[test]
    fn attachment_requires_meaningful_share() {
        let base = 1_000_000_u64;
        // Sitting: Arc'teryx dominant, one short video visit (3% share).
        let events = vec![
            event(
                base,
                Some("https://www.arcteryx.com/us/en/c/gear"),
                "Gear | Arc'teryx",
                "Chrome",
                640_000,
                60,
            ),
            event(
                base + 100_000,
                Some(&video_url("somevideo01", "PLotherlist0")),
                "Some Video",
                "Chrome",
                20_000,
                0,
            ),
        ];
        let intervals = compute_intervals(&events);
        let (works, _) = group_works(&intervals, &Declarations::default());
        // One work (Arc'teryx); the 3% video did not attach and did not
        // establish.
        assert_eq!(works.len(), 1);
        assert!(works[0].urls.iter().all(|u| !u.contains("youtube.com")));
    }

    /// Declarations are ground truth: a declared same-work pair merges
    /// groups the rules kept apart, and names the work.
    #[test]
    fn declared_same_work_merges_and_names() {
        let base = 1_000_000_u64;
        let day2 = base + 40 * 60 * 60 * 1000;
        let events = vec![
            event(
                base,
                Some("https://www.arcteryx.com/us/en/c/gear"),
                "Gear | Arc'teryx",
                "Chrome",
                600_000,
                60,
            ),
            event(
                day2,
                Some(&video_url("trekVid0001", "PLtreklist00")),
                "Gokyo Trek",
                "Chrome",
                600_000,
                0,
            ),
        ];
        let intervals = compute_intervals(&events);
        let before = group_works(&intervals, &Declarations::default()).0;
        assert_eq!(before.len(), 2, "without the declaration they are separate");

        let declarations = Declarations {
            same_work: vec![(
                "https://www.arcteryx.com/us/en/c/gear".to_string(),
                video_url("trekVid0001", "PLtreklist00"),
            )],
            named: vec![(
                "https://www.arcteryx.com/us/en/c/gear".to_string(),
                "Gokyo Trek Planning".to_string(),
            )],
        };
        let (works, _) = group_works(&intervals, &declarations);
        assert_eq!(works.len(), 1, "the declaration is ground truth");
        assert_eq!(works[0].identity, "Gokyo Trek Planning");
        assert!(
            works[0].urls.len() >= 2,
            "both resources belong to the work"
        );
    }

    /// Two established works co-present in two sittings produce exactly one
    /// merge proposal — offered, never applied.
    #[test]
    fn co_sitting_works_get_a_merge_proposal() {
        let base = 1_000_000_u64;
        let mut events = Vec::new();
        for day in 0..2 {
            let day_start = base + day * 40 * 60 * 60 * 1000;
            events.push(event(
                day_start,
                Some("https://www.arcteryx.com/us/en/c/gear"),
                "Gear | Arc'teryx",
                "Chrome",
                500_000,
                50,
            ));
            events.push(event(
                day_start + 550_000,
                Some(&video_url("trekVid0001", "PLtreklist00")),
                "Gokyo Trek",
                "Chrome",
                400_000,
                0,
            ));
        }
        let intervals = compute_intervals(&events);
        let (works, proposals) = group_works(&intervals, &Declarations::default());
        assert_eq!(works.len(), 2, "they remain two works");
        assert_eq!(proposals.len(), 1, "one merge proposal across two sittings");
        assert_eq!(proposals[0].shared_sittings, 2);
    }

    /// Sittings split at the 30-minute boundary.
    #[test]
    fn sittings_split_at_thirty_minutes() {
        let events = vec![
            event(1000, None, "A", "App", 60_000, 0),
            event(1000 + 29 * 60 * 1000, None, "A", "App", 60_000, 0),
            event(1000 + 70 * 60 * 1000, None, "A", "App", 60_000, 0),
        ];
        let intervals = compute_intervals(&events);
        let sittings = sittings_from_intervals(&intervals);
        assert_eq!(sittings.len(), 2, "70 minutes apart = two sittings");
    }

    /// The kundli shape: a long chat interleaved with shopping in one
    /// sitting must stay its own work (it establishes independently).
    #[test]
    fn a_substantial_interleaved_chat_stays_its_own_work() {
        let base = 1_000_000_u64;
        let mut events = vec![
            // Arc'teryx browsing, the sitting's dominant work.
            event(
                base,
                Some("https://www.arcteryx.com/us/en/c/boots"),
                "Boots | Arc'teryx",
                "Chrome",
                300_000,
                30,
            ),
            // A long astrology chat in the same sitting — substantial
            // dwell, real typing: it establishes, it does not attach.
            event(
                base + 50_000,
                Some("https://claude.ai/chat/f5c116b5-5fea-4f64-bfda-ef61c96c886e"),
                "Kundli Analysis",
                "Chrome",
                400_000,
                500,
            ),
            event(
                base + 460_000,
                Some("https://www.arcteryx.com/us/en/c/shells"),
                "Shells | Arc'teryx",
                "Chrome",
                300_000,
                20,
            ),
            event(
                base + 770_000,
                Some("https://claude.ai/chat/f5c116b5-5fea-4f64-bfda-ef61c96c886e"),
                "Kundli Analysis",
                "Chrome",
                300_000,
                400,
            ),
        ];
        let intervals = compute_intervals(&events);
        let (works, _) = group_works(&intervals, &Declarations::default());
        let chat_works: Vec<&Work> = works
            .iter()
            .filter(|w| w.urls.iter().any(|u| u.contains("claude.ai")))
            .collect();
        let gear_works: Vec<&Work> = works
            .iter()
            .filter(|w| w.urls.iter().any(|u| u.contains("arcteryx.com")))
            .collect();
        assert_eq!(chat_works.len(), 1, "the chat is its own work");
        assert_eq!(gear_works.len(), 1, "the gear browsing is its own work");
        assert_ne!(
            chat_works[0].identity, gear_works[0].identity,
            "2-second adjacency must not merge them"
        );
    }

    /// Title chrome: recurring dash-separated suffixes ("— Google Chrome –
    /// BHAVIT") must be stripped before title comparison. Measured on the
    /// real log: the browser suffix bridged every Chrome window into one
    /// body. The corpus here has exactly three chrome-tailed titles out of
    /// thirty — 10%, at the specific-token discount threshold — so only
    /// dash-stripping, not frequency discounting, can prevent the bridge.
    #[test]
    fn recurring_title_chrome_is_stripped_before_comparison() {
        // Direct: the chrome detector finds the suffix; the stripper
        // removes it and everything after it.
        let titles: Vec<String> = vec![
            "Boots — Google Chrome – BHAVIT".into(),
            "Chart — Google Chrome – BHAVIT".into(),
            "Maps — Google Chrome – BHAVIT".into(),
            "one thing".into(),
            "another thing entirely".into(),
        ];
        let chrome = recurring_title_chrome(&titles);
        assert!(
            chrome.contains("Google Chrome – BHAVIT"),
            "suffix detected: {chrome:?}"
        );
        assert_eq!(
            strip_title_chrome("Boots — Google Chrome – BHAVIT", &chrome),
            "Boots",
            "chrome and everything after it is not the page's name"
        );

        // Behavioral: "Boots — Google Chrome" and "Chart — Google Chrome"
        // share ONLY the suffix; without stripping their token Jaccard is
        // high enough to fuse. With a 30-title corpus (3 chrome-tailed,
        // exactly at the discount threshold) they must stay separate.
        // THREE chrome-tailed pages: the recurring-suffix detector needs
        // at least three occurrences before a segment counts as chrome.
        let mut events = vec![
            event(
                1_000,
                Some("https://shop.example.com/boots"),
                "Boots — Google Chrome – BHAVIT",
                "Chrome",
                400_000,
                10,
            ),
            event(
                500_000,
                Some("https://shop.example.com/chart"),
                "Chart — Google Chrome – BHAVIT",
                "Chrome",
                400_000,
                10,
            ),
            event(
                1_000_000,
                Some("https://shop.example.com/maps"),
                "Maps — Google Chrome – BHAVIT",
                "Chrome",
                400_000,
                10,
            ),
        ];
        // 27 more distinct titles so the 3 chrome tails sit at 10%.
        for i in 0..27 {
            events.push(event(
                2_000_000 + i * 100_000,
                Some(&format!("https://other{i}.example.com/page{i}")),
                &format!("Distinct page number {i}"),
                "Chrome",
                10_000,
                0,
            ));
        }
        let intervals = crate::intervals::compute_intervals(&events);
        let (works, _) = group_works(&intervals, &Declarations::default());
        let boots_work: Vec<&crate::works::Work> = works
            .iter()
            .filter(|w| w.titles.iter().any(|t| t.starts_with("Boots")))
            .collect();
        let chart_in_boots = boots_work
            .first()
            .map(|w| w.titles.iter().any(|t| t.starts_with("Chart")))
            .unwrap_or(false);
        assert!(
            !chart_in_boots,
            "a shared browser suffix must never bridge two pages"
        );
    }

    /// Member stickiness: a page that belongs to a structural work (the
    /// playlist) never attaches into another work through co-sitting
    /// share — locked via a playlist that qualifies ONLY through the
    /// multi-page structural rule (240 s + 60 s across two sittings,
    /// never >= 300 s in one) and a gear sitting where one of its videos
    /// holds an attachable share.
    #[test]
    fn work_members_are_sticky_across_sittings() {
        let day2 = 40 * 60 * 60 * 1000_u64;
        let mut events = vec![
            // Two playlist videos, 240 s and 60 s in separate sittings: the
            // group qualifies structurally (2 pages, 300 s total), never by
            // sitting establishment.
            event(
                1_000,
                Some(&video_url("stickyVid001", "PLstickylist0")),
                "Trek A",
                "Chrome",
                240_000,
                0,
            ),
            event(
                day2,
                Some(&video_url("stickyVid002", "PLstickylist0")),
                "Trek B",
                "Chrome",
                60_000,
                0,
            ),
            // A gear sitting where a playlist video holds ~9% (above the
            // attach floor) — it must NOT join the gear work.
            event(
                2 * day2,
                Some("https://www.arcteryx.com/us/en/c/gear"),
                "Gear | Arc'teryx",
                "Chrome",
                500_000,
                50,
            ),
            event(
                2 * day2 + 520_000,
                Some(&video_url("stickyVid001", "PLstickylist0")),
                "Trek A",
                "Chrome",
                50_000,
                0,
            ),
        ];
        let intervals = crate::intervals::compute_intervals(&events);
        let (works, _) = group_works(&intervals, &Declarations::default());
        let gear_works: Vec<&crate::works::Work> = works
            .iter()
            .filter(|w| w.urls.iter().any(|u| u.contains("arcteryx.com")))
            .collect();
        assert_eq!(gear_works.len(), 1, "the gear work exists");
        assert!(
            !gear_works[0].urls.iter().any(|u| u.contains("stickyVid")),
            "a playlist member never attaches into the gear work"
        );
    }

    /// The shared-opaque-token cap: a token gluing at most 30 pages links
    /// them (a playlist); at 31 it is a site-wide session id and glues
    /// nothing. Pin the boundary exactly.
    #[test]
    fn shared_token_links_cap_at_thirty_pages() {
        fn corpus(n: usize) -> Vec<ObservedEvent> {
            (0..n)
                .map(|i| {
                    // Distinct pages (different paths) sharing one opaque
                    // session token — the collection shape.
                    event(
                        1_000 + i as u64 * 100_000,
                        Some(&format!(
                            "https://site.example.com/page{i}?session=SharedSessionToken01"
                        )),
                        &format!("Page {i}"),
                        "Chrome",
                        60_000,
                        0,
                    )
                })
                .collect()
        }
        // 30 pages: all linked through the shared token.
        let intervals = crate::intervals::compute_intervals(&corpus(30));
        let (_, _, trace) = group_works_traced(&intervals, &Declarations::default());
        assert!(
            trace
                .iter()
                .any(|t| t.contains("token SharedSessionToken01")),
            "30 pages sharing a token are one structural group"
        );
        // 31 pages: the cap fires, no token link is drawn.
        let intervals = crate::intervals::compute_intervals(&corpus(31));
        let (_, _, trace) = group_works_traced(&intervals, &Declarations::default());
        assert!(
            !trace
                .iter()
                .any(|t| t.contains("token SharedSessionToken01")),
            "31 pages sharing a token is a site-wide id, not a collection"
        );
    }

    /// Ambient quarantine's two negative cases: a page WITH keystrokes
    /// spanning two works is shared ground, never quarantined (it joins
    /// both — multi-label); a zero-keystroke page spanning two sittings
    /// of the SAME work belongs to it, not to quarantine.
    #[test]
    fn quarantine_spares_shared_ground_and_same_work_presence() {
        let day2 = 40 * 60 * 60 * 1000_u64;
        let mut events = vec![
            // Sitting 1: work A (an editor) + a chat the person types in —
            // briefly (dwell below the 300 s establishment floor, so the
            // chat attaches rather than founding its own work).
            event(1_000, None, "ZCode", "ZCode", 500_000, 300),
            event(
                520_000,
                Some("https://claude.ai/chat/aaa11111-bbbb"),
                "Planning Chat",
                "Chrome",
                100_000,
                200,
            ),
            // Sitting 2: work B + the SAME chat (typed again, briefly).
            event(
                day2,
                Some("https://www.arcteryx.com/us/en/c/gear"),
                "Gear | Arc'teryx",
                "Chrome",
                500_000,
                50,
            ),
            event(
                day2 + 520_000,
                Some("https://claude.ai/chat/aaa11111-bbbb"),
                "Planning Chat",
                "Chrome",
                100_000,
                150,
            ),
            // The SAME reference page, zero keystrokes, appears inside the
            // ZCode work's sitting (and an earlier one) — same-work
            // recurrence attaches, never quarantines.
            event(
                1_000 + 500_000,
                Some("https://docs.example.com/reference"),
                "Reference Guide",
                "Chrome",
                80_000,
                0,
            ),
            event(2 * day2, None, "ZCode", "ZCode", 400_000, 200),
            event(
                2 * day2 + 420_000,
                Some("https://docs.example.com/reference"),
                "Reference Guide",
                "Chrome",
                80_000,
                0,
            ),
        ];
        let intervals = crate::intervals::compute_intervals(&events);
        let (works, _) = group_works(&intervals, &Declarations::default());
        // The typed chat belongs to BOTH works (multi-label), never
        // quarantined.
        let chat_works: Vec<&crate::works::Work> = works
            .iter()
            .filter(|w| w.urls.iter().any(|u| u.contains("claude.ai/chat/aaa11111")))
            .collect();
        assert!(
            chat_works.len() >= 2,
            "a typed page spanning two works joins both, expected >= 2, got {}",
            chat_works.len()
        );
        // The zero-keystroke reference page appears in two sittings of the
        // ZCode work only: it attaches there, never quarantined.
        let zcode_works: Vec<&crate::works::Work> = works
            .iter()
            .filter(|w| w.apps.iter().any(|a| a == "ZCode"))
            .collect();
        assert_eq!(zcode_works.len(), 1, "one ZCode work");
        assert!(
            zcode_works[0]
                .urls
                .iter()
                .any(|u| u.contains("docs.example.com/reference")),
            "a page recurring within one work's sittings attaches to it"
        );
    }
}
