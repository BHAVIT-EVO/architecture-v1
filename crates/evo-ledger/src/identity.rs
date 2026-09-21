//! Identity chain: resolves an observed event's task key.
//!
//! The chain (measured against the labeled ground truth):
//!
//! 1. `document_path` (confidence 0.85) — the file being edited. Most stable.
//! 2. `page` (confidence 0.80) — the canonical PAGE of the active URL
//!    (video id, chat id, search query, or host+path; see `pages`). A host
//!    is NOT a page: "youtube.com" spans unrelated videos while one real
//!    work spans many hosts. Page OVERRIDES window_title because browser
//!    titles are ephemeral; page overrides host because host over-groups.
//!    `url_host` survives for display.
//! 3. `window_title` (confidence 0.70) — for apps without URLs or documents.
//! 4. `app_name` (confidence 0.60) — the fallback. Groups by application,
//!    which is the coarsest possible identity.

use crate::pages::canonical_page;
use crate::ObservedEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct TaskKey {
    pub document_path: Option<String>,
    /// The canonical page identity of the active URL (never the bare host).
    pub page: Option<String>,
    pub window_title: Option<String>,
    /// The URL host, retained for display and search only — never grouping.
    pub url_host: Option<String>,
    pub app_name: String,
    pub confidence: f64,
}

pub struct IdentityChain;

impl IdentityChain {
    /// Resolves an event's task key by walking the identity chain.
    ///
    /// The FIRST non-empty signal in priority order becomes the grouping
    /// identity. Page is checked before window title because browser tab
    /// titles change on every navigation while the page identity is stable.
    pub fn resolve(event: &ObservedEvent) -> TaskKey {
        let document_path = event.document_path.clone().filter(|p| !p.trim().is_empty());

        let page = event
            .url
            .clone()
            .filter(|u| !u.trim().is_empty())
            .and_then(|u| canonical_page(&u));

        let url_host = event
            .url
            .clone()
            .filter(|u| !u.trim().is_empty())
            .and_then(|u| extract_host(&u));

        let window_title: Option<String> = {
            let wt = &event.window_title;
            if wt.trim().is_empty() {
                None
            } else {
                Some(wt.clone())
            }
        };

        // Priority: document_path > page > window_title > app_name
        let confidence = if document_path.is_some() {
            0.85
        } else if page.is_some() {
            0.80
        } else if window_title.is_some() {
            0.70
        } else {
            0.60
        };

        TaskKey {
            document_path,
            page,
            window_title,
            url_host,
            app_name: event.app_name.clone(),
            confidence,
        }
    }

    /// Whether two task keys refer to the same resource.
    ///
    /// The comparison walks the chain in the same priority order as
    /// `resolve`. The first signal that BOTH keys have is the decider:
    /// if it matches, same resource; if it differs, different resource.
    pub fn same_work(a: &TaskKey, b: &TaskKey) -> bool {
        // Document path: if both have one, they must match.
        if let (Some(a_doc), Some(b_doc)) = (&a.document_path, &b.document_path) {
            return a_doc == b_doc;
        }
        // Page: if both have one, they must match. Two different videos on
        // the same host are different pages; two visits to the same search
        // or the same chat are the same page.
        if let (Some(a_page), Some(b_page)) = (&a.page, &b.page) {
            return a_page == b_page;
        }
        // Window title: if both have one, they must match.
        if let (Some(a_title), Some(b_title)) = (&a.window_title, &b.window_title) {
            return a_title == b_title;
        }
        // App name: always present, must match.
        a.app_name == b.app_name
    }
}

/// Extracts the host (domain) from a URL string.
/// "https://www.example.com/path?q=1" → "example.com"
fn extract_host(url: &str) -> Option<String> {
    // Must start with a protocol to be a URL we can extract a host from
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = stripped.split('/').next()?;
    let host = host.trim_start_matches("www.");
    // Must contain a dot to look like a domain (rejects "not-a-url", "localhost" is edge case)
    if host.is_empty() || !host.contains('.') {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(app: &str, title: &str, url: Option<&str>, doc: Option<&str>) -> ObservedEvent {
        ObservedEvent {
            timestamp_ms: 1000,
            app_name: app.to_string(),
            window_title: title.to_string(),
            url: url.map(String::from),
            document_path: doc.map(String::from),
            typed: false,
            dwell_ms: 5000,
            keys: 0,
            clicks: 0,
            scrolls: 0,
        }
    }

    #[test]
    fn document_path_outranks_everything() {
        let event = ev(
            "VS Code",
            "some title",
            Some("https://example.com"),
            Some("/src/main.rs"),
        );
        let key = IdentityChain::resolve(&event);
        assert_eq!(key.document_path, Some("/src/main.rs".to_string()));
        assert_eq!(key.confidence, 0.85);
    }

    #[test]
    fn page_outranks_window_title() {
        // Browser: the page is the identity, not the changing title. Two
        // visits to the SAME page are one resource despite different
        // titles; two different pages on one host are different resources
        // (the mirror problem: one host spans unrelated works).
        let a = IdentityChain::resolve(&ev(
            "Chrome",
            "Page 1",
            Some("https://example.com/a?x=1"),
            None,
        ));
        let b = IdentityChain::resolve(&ev(
            "Chrome",
            "Page 2",
            Some("https://example.com/a?x=1"),
            None,
        ));
        assert!(
            IdentityChain::same_work(&a, &b),
            "same page = same resource despite different titles"
        );
        let c =
            IdentityChain::resolve(&ev("Chrome", "Page 2", Some("https://example.com/b"), None));
        assert!(
            !IdentityChain::same_work(&a, &c),
            "different pages on one host are different resources"
        );
        assert_eq!(a.confidence, 0.80, "page confidence");
    }

    #[test]
    fn window_title_when_no_url() {
        let event = ev("Terminal", "My Project — Terminal", None, None);
        let key = IdentityChain::resolve(&event);
        assert_eq!(key.window_title, Some("My Project — Terminal".to_string()));
        assert_eq!(key.confidence, 0.70);
    }

    #[test]
    fn app_name_is_the_fallback() {
        let event = ev("Terminal", "", None, None);
        let key = IdentityChain::resolve(&event);
        assert_eq!(key.app_name, "Terminal");
        assert_eq!(key.confidence, 0.60);
    }

    #[test]
    fn same_document_means_same_work() {
        let a = IdentityChain::resolve(&ev("VS Code", "title A", None, Some("/doc.rs")));
        let b = IdentityChain::resolve(&ev("Other App", "title B", None, Some("/doc.rs")));
        assert!(IdentityChain::same_work(&a, &b));
    }

    #[test]
    fn different_domains_mean_different_work() {
        let a = IdentityChain::resolve(&ev("Chrome", "x", Some("https://github.com/a"), None));
        let b = IdentityChain::resolve(&ev("Chrome", "x", Some("https://gitlab.com/b"), None));
        assert!(!IdentityChain::same_work(&a, &b));
    }

    #[test]
    fn different_apps_different_work() {
        let a = IdentityChain::resolve(&ev("Terminal", "", None, None));
        let b = IdentityChain::resolve(&ev("Chrome", "", None, None));
        assert!(!IdentityChain::same_work(&a, &b));
    }

    #[test]
    fn host_extraction_strips_www_and_path() {
        assert_eq!(
            extract_host("https://www.example.com/path?q=1"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_host("http://api.service.io/v2/endpoint"),
            Some("api.service.io".to_string())
        );
        assert_eq!(extract_host("not-a-url"), None);
    }
}
