//! Groups work intervals into "Works" — the bodies of work the desktop shows.
//!
//! A Work is a collection of intervals that share the same identity
//! (same URL host, same document, or same app), aggregated across time.
//! It answers: "What was I working on?" not "What did I do at 3:42 PM?"
//!
//! The grouping is simple and deterministic: intervals with the same
//! primary identity (the first non-None in the identity chain) belong
//! to the same Work. A Work accumulates:
//! - Total time across all its intervals
//! - All URLs, documents, and apps it touched
//! - The most recent interval (for "when did I last work on this?")
//! - Whether the person ever typed (production evidence)

use crate::identity::TaskKey;
use crate::intervals::WorkInterval;
use rusqlite::{params, Connection};
use std::collections::BTreeMap;

/// A body of work: all intervals sharing the same identity.
#[derive(Debug, Clone)]
pub struct Work {
    /// The primary identity (first non-None in the chain).
    pub identity: String,
    /// What kind of identity (document, url_host, window_title, app).
    pub identity_kind: IdentityKind,
    /// All intervals that belong to this work.
    pub intervals: Vec<WorkInterval>,
    /// Total time across all intervals (ms).
    pub total_time_ms: u64,
    /// All URLs touched (for restore).
    pub urls: Vec<String>,
    /// All document paths touched.
    pub documents: Vec<String>,
    /// All window titles seen (presentation evidence: the page names the
    /// person read, richer than a bare host).
    pub titles: Vec<String>,
    /// All app names used.
    pub apps: Vec<String>,
    /// Whether the person ever typed (production evidence).
    pub has_production: bool,
    /// Content-free input counters aggregated across all intervals. The
    /// keys-vs-scrolls mix distinguishes writing from reading.
    pub keys: u64,
    pub clicks: u64,
    pub scrolls: u64,
    /// How many file saves were attributed to this work: files produced
    /// through it, the strongest building evidence there is.
    pub attached_saves: usize,
    /// Most recent activity (epoch ms).
    pub last_active_ms: u64,
    /// The hypothesis layer's own session count (distinct sittings with a
    /// meaningful presence), overriding the interval heuristic when set.
    pub session_count_override: Option<usize>,
}

impl Work {
    /// A human-readable title for this work.
    pub fn title(&self) -> String {
        // Try to get the best title from the intervals
        if let Some(iv) = self.intervals.first() {
            return iv.title();
        }
        self.identity.clone()
    }

    /// Total time in human-readable form.
    pub fn time_summary(&self) -> String {
        let secs = self.total_time_ms / 1000;
        if secs < 60 {
            format!("{secs}s")
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Number of distinct sessions (intervals with real engagement:
    /// multiple events OR sustained attention >= 30 seconds).
    pub fn session_count(&self) -> usize {
        self.intervals
            .iter()
            .filter(|iv| iv.event_count >= 2 || iv.total_dwell_ms >= 30_000)
            .count()
    }

    /// The person's dominant engagement mode, from the evidence mix alone.
    ///
    /// Files were produced (saves attributed) → the person was building.
    /// Otherwise the input counters tell reading from writing: a
    /// typing-dominant bucket mix is composition (chat, prose, code in
    /// buffers that never hit disk); a click/scroll-dominant mix is
    /// navigation and reading; no input at all is passive attention.
    pub fn engagement(&self) -> Engagement {
        if self.attached_saves > 0 {
            return Engagement::Building;
        }
        if self.keys > self.clicks + self.scrolls {
            return Engagement::Writing;
        }
        if self.clicks + self.scrolls > 0 {
            return Engagement::Reading;
        }
        Engagement::Reading
    }

    /// The application a resource (document path or URL) was witnessed
    /// in, derived from the work's own intervals: every interval carries
    /// its app name alongside the document paths and URLs seen in it.
    /// The most recent witness wins (decided by interval end time, never
    /// by interval order). `None` where the resource was never seen with
    /// an app — the caller falls back to the OS default handler.
    /// This is the most recent observation, not proof of ownership
    /// (a merged interval may have seen the resource alongside another
    /// app); the work-level app list never decides for a document.
    pub fn app_for_resource(&self, resource: &str) -> Option<&str> {
        self.intervals
            .iter()
            .filter(|iv| {
                iv.document_paths.iter().any(|d| d == resource)
                    || iv.urls.iter().any(|u| u == resource)
            })
            .filter(|iv| !iv.task_key.app_name.trim().is_empty())
            .max_by_key(|iv| iv.end_ms)
            .map(|iv| iv.task_key.app_name.trim())
    }
}

/// How the person engaged with this work, from the evidence mix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engagement {
    /// Files were produced through this work.
    Building,
    /// Typing-dominant input: composition without observed file output.
    Writing,
    /// Click/scroll-dominant or passive attention: navigation and reading.
    Reading,
}

/// What kind of identity a work is grouped by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentityKind {
    Document,
    UrlHost,
    WindowTitle,
    App,
    /// A purpose-level work formed by the hypothesis layer over many
    /// resources and sittings.
    Purpose,
}

impl IdentityKind {
    pub fn label(&self) -> &'static str {
        match self {
            IdentityKind::Document => "document",
            IdentityKind::UrlHost => "site",
            IdentityKind::WindowTitle => "window",
            IdentityKind::App => "app",
            IdentityKind::Purpose => "work",
        }
    }
}

/// Groups intervals into Works by their primary identity.
///
/// The primary identity is the first non-None field in the TaskKey's
/// chain: document_path > url_host > window_title > app_name.
/// Two intervals with the same primary identity are the same Work,
/// regardless of when they occurred.
pub fn group_into_works(intervals: Vec<WorkInterval>) -> Vec<Work> {
    let mut works: BTreeMap<(String, IdentityKind), Work> = BTreeMap::new();

    for interval in intervals {
        let (identity, kind) = primary_identity(&interval.task_key);
        let key = (identity.clone(), kind);

        let work = works.entry(key).or_insert_with(|| Work {
            identity,
            identity_kind: kind,
            intervals: Vec::new(),
            total_time_ms: 0,
            urls: Vec::new(),
            documents: Vec::new(),
            titles: Vec::new(),
            apps: Vec::new(),
            has_production: false,
            keys: 0,
            clicks: 0,
            scrolls: 0,
            attached_saves: 0,
            last_active_ms: 0,
            session_count_override: None,
        });

        // Accumulate
        work.total_time_ms += interval.total_dwell_ms;
        work.last_active_ms = work.last_active_ms.max(interval.end_ms);
        work.keys += interval.keys;
        work.clicks += interval.clicks;
        work.scrolls += interval.scrolls;
        work.attached_saves += interval.attached_saves;
        if interval.typed_count > 0 {
            work.has_production = true;
        }
        for url in &interval.urls {
            if !work.urls.contains(url) {
                work.urls.push(url.clone());
            }
        }
        for title in &interval.titles {
            if !work.titles.contains(title) {
                work.titles.push(title.clone());
            }
        }
        for doc in &interval.document_paths {
            if !work.documents.contains(doc) {
                work.documents.push(doc.clone());
            }
        }
        if !work.apps.contains(&interval.task_key.app_name) {
            work.apps.push(interval.task_key.app_name.clone());
        }
        work.intervals.push(interval);
    }

    // Sort by last_active (most recent first), then by total_time
    let mut result: Vec<Work> = works.into_values().collect();
    result.sort_by(|a, b| {
        b.last_active_ms
            .cmp(&a.last_active_ms)
            .then(b.total_time_ms.cmp(&a.total_time_ms))
    });

    result
}

/// Returns the primary identity (first non-None in the chain) and its kind.
fn primary_identity(key: &TaskKey) -> (String, IdentityKind) {
    if let Some(doc) = &key.document_path {
        return (doc.clone(), IdentityKind::Document);
    }
    if let Some(host) = &key.url_host {
        return (host.clone(), IdentityKind::UrlHost);
    }
    if let Some(title) = &key.window_title {
        return (title.clone(), IdentityKind::WindowTitle);
    }
    (key.app_name.clone(), IdentityKind::App)
}

/// Queries all intervals from the database and groups them into Works.
/// Only returns works with at least `min_events` total events.
pub fn works_from_database(db: &Connection, limit: usize) -> Result<Vec<Work>, crate::LedgerError> {
    Ok(works_and_proposals_from_database(db, limit)?.0)
}

/// Works and the merge proposals between them, from the hypothesis layer.
pub fn works_and_proposals_from_database(
    db: &Connection,
    limit: usize,
) -> Result<(Vec<Work>, Vec<crate::hypotheses::MergeProposal>), crate::LedgerError> {
    // Grouping is the hypothesis layer: purpose-level works formed from
    // sittings, structural hard links, establishment, and attachment —
    // with the person's declarations as ground truth. The identity-chain
    // grouping (`group_into_works`) remains the resource-level view.
    // A year of capture measured ~15k intervals per 20× of the current log;
    // the previous 20k cap would silently truncate grouping at ~2 years.
    // 200k holds over a decade; if ever hit, grouping degrades loudly by
    // dropping the OLDEST intervals (the query orders newest-first).
    let intervals = crate::intervals::query_with_filter(db, 200_000, 1)?;
    let declarations = read_declarations(db);
    let (mut works, proposals) = crate::hypotheses::group_works(&intervals, &declarations);

    // Filter: only works with enough engagement to be "work" (not just a
    // glance), then the most recent `limit` — keeping any work a proposal
    // references so proposals stay resolvable.
    works.retain(|w| w.total_time_ms >= 10_000);
    let proposal_indices: std::collections::BTreeSet<usize> = proposals
        .iter()
        .flat_map(|p| [p.work_a, p.work_b])
        .collect();
    let mut filtered: Vec<Work> = Vec::new();
    let mut old_to_new: std::collections::BTreeMap<usize, usize> =
        std::collections::BTreeMap::new();
    for (old_index, work) in works.into_iter().enumerate() {
        if old_index < limit || proposal_indices.contains(&old_index) {
            old_to_new.insert(old_index, filtered.len());
            filtered.push(work);
        }
    }
    let proposals: Vec<crate::hypotheses::MergeProposal> = proposals
        .into_iter()
        .filter_map(|p| {
            let a = *old_to_new.get(&p.work_a)?;
            let b = *old_to_new.get(&p.work_b)?;
            Some(crate::hypotheses::MergeProposal {
                work_a: a,
                work_b: b,
                shared_sittings: p.shared_sittings,
            })
        })
        .collect();
    Ok((filtered, proposals))
}

/// Reads the person's declared same-work pairs from the ledger.
pub fn read_declarations(db: &Connection) -> crate::hypotheses::Declarations {
    let mut declarations = crate::hypotheses::Declarations::default();
    let Ok(mut stmt) = db.prepare("SELECT subject_a, subject_b FROM declarations") else {
        return declarations;
    };
    if let Ok(rows) = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        for pair in rows.flatten() {
            declarations.same_work.push(pair);
        }
    }
    declarations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intervals;
    use crate::ObservedEvent;

    fn ev(
        ts: u64,
        app: &str,
        title: &str,
        url: Option<&str>,
        doc: Option<&str>,
        dwell: u64,
    ) -> ObservedEvent {
        ObservedEvent {
            timestamp_ms: ts,
            app_name: app.to_string(),
            window_title: title.to_string(),
            url: url.map(String::from),
            document_path: doc.map(String::from),
            typed: false,
            dwell_ms: dwell,
            keys: 0,
            clicks: 0,
            scrolls: 0,
        }
    }

    #[test]
    fn same_url_host_groups_into_one_work() {
        let events = vec![
            ev(
                1000,
                "Chrome",
                "Page 1",
                Some("https://framer.com/project"),
                None,
                5000,
            ),
            ev(
                61000,
                "Chrome",
                "Page 2",
                Some("https://framer.com/other"),
                None,
                10000,
            ),
            ev(
                121000,
                "Chrome",
                "Page 3",
                Some("https://framer.com/third"),
                None,
                8000,
            ),
        ];
        let intervals = intervals::compute_intervals(&events);
        let works = group_into_works(intervals);
        assert_eq!(works.len(), 1, "all framer.com pages = one work");
        assert_eq!(works[0].identity, "framer.com");
        assert_eq!(works[0].identity_kind, IdentityKind::UrlHost);
        assert_eq!(works[0].total_time_ms, 23000);
    }

    #[test]
    fn different_sites_are_different_works() {
        let events = vec![
            ev(
                1000,
                "Chrome",
                "a",
                Some("https://framer.com/x"),
                None,
                10000,
            ),
            ev(
                61000,
                "Chrome",
                "b",
                Some("https://github.com/y"),
                None,
                10000,
            ),
        ];
        let intervals = intervals::compute_intervals(&events);
        let works = group_into_works(intervals);
        assert_eq!(
            works.len(),
            2,
            "framer.com and github.com are different works"
        );
    }

    #[test]
    fn same_document_groups_across_sessions() {
        let events = vec![
            ev(
                1000,
                "VS Code",
                "main.rs",
                None,
                Some("/src/main.rs"),
                30000,
            ),
            ev(
                3600000,
                "VS Code",
                "main.rs",
                None,
                Some("/src/main.rs"),
                60000,
            ), // 1 hour later
        ];
        let intervals = intervals::compute_intervals(&events);
        let works = group_into_works(intervals);
        assert_eq!(works.len(), 1, "same file across sessions = one work");
        assert_eq!(works[0].identity_kind, IdentityKind::Document);
        assert_eq!(works[0].session_count(), 2, "two separate sessions");
    }

    #[test]
    fn work_has_production_evidence() {
        let events = vec![ev(
            1000,
            "Chrome",
            "chat",
            Some("https://chat.openai.com/c"),
            None,
            30000,
        )];
        let mut typed_event = ev(
            61000,
            "Chrome",
            "chat",
            Some("https://chat.openai.com/c"),
            None,
            30000,
        );
        typed_event.typed = true;
        let all = vec![events[0].clone(), typed_event];
        let intervals = intervals::compute_intervals(&all);
        let works = group_into_works(intervals);
        assert!(works[0].has_production, "typing on the site = production");
    }

    #[test]
    fn most_recent_work_first() {
        let events = vec![
            ev(
                1000,
                "Chrome",
                "old",
                Some("https://old.example.com"),
                None,
                10000,
            ),
            ev(
                1000000,
                "Chrome",
                "new",
                Some("https://new.example.com"),
                None,
                10000,
            ),
        ];
        let intervals = intervals::compute_intervals(&events);
        let works = group_into_works(intervals);
        assert_eq!(
            works[0].identity, "new.example.com",
            "most recent work first"
        );
    }

    /// The engagement classification reads the evidence mix, not the app.
    /// Browsing with a few keystrokes in a search box must not read as
    /// writing; composing in a chat must not wait for a file save.
    #[test]
    fn engagement_classifies_the_input_mix() {
        // Browsing: many scrolls, few keys.
        let mut browsing = ev(
            1000,
            "Chrome",
            "page",
            Some("https://shop.example.com/products"),
            None,
            10_000,
        );
        browsing.keys = 3;
        browsing.clicks = 8;
        browsing.scrolls = 40;
        let intervals = intervals::compute_intervals(&[browsing]);
        let works = group_into_works(intervals);
        assert_eq!(works[0].engagement(), Engagement::Reading);

        // Composing: typing-dominant, nothing saved.
        let mut writing = ev(
            1000,
            "Chrome",
            "chat",
            Some("https://chat.example.com/c/1"),
            None,
            10_000,
        );
        writing.typed = true;
        writing.keys = 220;
        writing.clicks = 4;
        writing.scrolls = 6;
        let intervals = intervals::compute_intervals(&[writing]);
        let works = group_into_works(intervals);
        assert_eq!(works[0].engagement(), Engagement::Writing);

        // Producing: a save attributed to the work outweighs the mix.
        let mut ledger = crate::Ledger::open_memory().unwrap();
        let mut building = ev(
            1000,
            "ZCode",
            "app.rs — project",
            None,
            Some("/src/app.rs"),
            350_000,
        );
        building.typed = true;
        building.keys = 100;
        ledger.ingest(&[building]).unwrap();
        ledger
            .attach_production(&[crate::ProductionSave {
                timestamp_ms: 2000,
                path: "/src/app.rs".to_string(),
            }])
            .unwrap();
        let works = ledger.works(10).unwrap();
        assert_eq!(works[0].engagement(), Engagement::Building);
    }

    /// Window titles are presentation evidence: they survive identity
    /// resolution (a URL-host work keeps the page names the person read).
    #[test]
    fn titles_survive_into_works() {
        let events = vec![
            ev(
                1000,
                "Chrome",
                "Waitlist Template Copy — Framer",
                Some("https://framer.com/project/123"),
                None,
                10_000,
            ),
            ev(
                61_000,
                "Chrome",
                "Landing Page — Framer",
                Some("https://framer.com/project/123"),
                None,
                10_000,
            ),
        ];
        let intervals = intervals::compute_intervals(&events);
        let works = group_into_works(intervals);
        assert_eq!(works[0].identity, "framer.com");
        assert!(works[0]
            .titles
            .contains(&"Waitlist Template Copy — Framer".to_string()));
        assert!(works[0]
            .titles
            .contains(&"Landing Page — Framer".to_string()));
    }

    /// Each document restores through the app it was witnessed in —
    /// the work-level app list never decides for a document.
    #[test]
    fn app_for_resource_returns_the_witnessed_app_per_document() {
        let events = vec![
            ev(1000, "VS Code", "main.rs", None, Some("/src/main.rs"), 30000),
            ev(
                61000,
                "Preview",
                "report.pdf",
                None,
                Some("/docs/report.pdf"),
                30000,
            ),
        ];
        let intervals = intervals::compute_intervals(&events);
        let works = group_into_works(intervals);
        assert_eq!(works.len(), 2, "different documents = different works");
        let code = works
            .iter()
            .find(|w| w.identity == "/src/main.rs")
            .unwrap();
        assert_eq!(code.app_for_resource("/src/main.rs"), Some("VS Code"));
        assert_eq!(code.app_for_resource("/docs/report.pdf"), None);
        let pdf = works
            .iter()
            .find(|w| w.identity == "/docs/report.pdf")
            .unwrap();
        assert_eq!(pdf.app_for_resource("/docs/report.pdf"), Some("Preview"));
    }

    /// When a document was seen in two apps, the most recent witness
    /// wins — restoration follows the person's latest habit.
    #[test]
    fn app_for_resource_prefers_the_most_recent_witness() {
        let events = vec![
            ev(1000, "TextEdit", "notes.md", None, Some("/notes.md"), 30000),
            // An hour later the same document is open in another app.
            ev(
                3_661_000,
                "VS Code",
                "notes.md",
                None,
                Some("/notes.md"),
                30000,
            ),
        ];
        let intervals = intervals::compute_intervals(&events);
        let works = group_into_works(intervals);
        assert_eq!(works.len(), 1, "same document = one work");
        assert_eq!(works[0].app_for_resource("/notes.md"), Some("VS Code"));
    }

    /// URLs carry their witnessed app too; an unknown resource maps to
    /// nothing and the OS default handler decides.
    #[test]
    fn app_for_resource_covers_urls_and_unknown_resources() {
        let events = vec![ev(
            1000,
            "Safari",
            "Article",
            Some("https://example.com/article"),
            None,
            30000,
        )];
        let intervals = intervals::compute_intervals(&events);
        let works = group_into_works(intervals);
        assert_eq!(works.len(), 1);
        assert_eq!(
            works[0].app_for_resource("https://example.com/article"),
            Some("Safari")
        );
        assert_eq!(works[0].app_for_resource("https://other.example/"), None);
    }
}
