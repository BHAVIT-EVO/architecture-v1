//! Page-level resource identity.
//!
//! A host is not a page, and a work is not a host: "youtube.com" spans twenty
//! unrelated videos and "claude.ai" spans unrelated chats, while one real work
//! spans many hosts. The resource atom for grouping is therefore the *page*:
//! the stable, canonical identity of one addressable thing a person attended.
//!
//! All rules here are shape-based or web-convention-based — no application
//! names, no domain lists:
//!
//! * **Opaque identifiers are identity.** A query parameter whose value looks
//!   like a machine-minted token (long, mixed-case alphanumeric, hex, UUID)
//!   is what distinguishes one video, one playlist, one document from
//!   another on the same site. Values that read as words are not identity.
//! * **Universal web conventions are honored**: `utm_*`, `fbclid`, `gclid`
//!   are the tracking standard and are stripped wherever they appear;
//!   `q` is the universal search parameter and its text IS the page (each
//!   search is its own page); `www.` is stripped as the null subdomain.
//! * **Path is identity.** `host/path` survives whole; a path ending in an
//!   opaque segment (a chat id, a document id) is already page-grain.
//!
//! Everything else — the raw URL, the window title, the app — remains
//! evidence on the interval; only the grouping key changes.

/// The canonical page identity of a URL, or `None` when the string is not a
/// URL. Two attendance records of the same page canonicalize equal.
pub fn canonical_page(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    if rest.is_empty() {
        return None;
    }
    let (authority, tail) = match rest.split_once('/') {
        Some((a, t)) => (a, t),
        None => (rest, ""),
    };
    let host = authority
        .split('@')
        .next_back()
        .unwrap_or(authority)
        .split(':')
        .next()
        .unwrap_or(authority);
    let host = host.trim_start_matches("www.");
    if host.is_empty() || !host.contains('.') {
        return None; // not a domain
    }

    let (path, query) = match tail.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (tail, None),
    };
    let path = path.split('#').next().unwrap_or(path);
    let path = path.trim_end_matches('/');

    let mut identity_params: Vec<(String, String)> = Vec::new();
    if let Some(query) = query {
        for pair in query.split('&').filter(|p| !p.is_empty()) {
            let (name, value) = match pair.split_once('=') {
                Some((n, v)) => (n, v),
                None => (pair, ""),
            };
            // The tracking standard: never identity. Includes the
            // search-engine tracking family (everything but the query
            // itself on a search page).
            if name.starts_with("utm_")
                || name.starts_with("gs_")
                || name.starts_with("si")
                || matches!(
                    name,
                    "fbclid"
                        | "gclid"
                        | "msclkid"
                        | "ref"
                        | "referrer"
                        | "oq"
                        | "ei"
                        | "ved"
                        | "sa"
                        | "sourceid"
                        | "ie"
                        | "oe"
                        | "mtid"
                )
            {
                continue;
            }
            // The universal search parameter: its text is the page.
            if name == "q" {
                identity_params.push((name.to_string(), value.to_string()));
                continue;
            }
            // Opaque machine tokens are identity; word-valued parameters
            // (filters, sessions, display options) are not.
            if is_opaque_token(value) {
                identity_params.push((name.to_string(), value.to_string()));
            }
        }
    }
    identity_params.sort();

    let mut page = if path.is_empty() {
        format!("https://{host}")
    } else {
        format!("https://{host}/{path}")
    };
    if !identity_params.is_empty() {
        let rendered: Vec<String> = identity_params
            .iter()
            .map(|(n, v)| format!("{n}={v}"))
            .collect();
        page.push('?');
        page.push_str(&rendered.join("&"));
    }
    Some(page)
}

/// The opaque identifier tokens a canonical page carries.
///
/// A collection (a playlist, a thread, a shared document family) is not
/// discoverable from one URL: the token that glues pages together is the
/// one SEVERAL pages share. Callers therefore extract tokens per page and
/// hard-link pages that share one — a global, deterministic fact, with no
/// per-site knowledge anywhere.
pub fn opaque_tokens(page: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    if let Some((_, path)) = page.split_once("://").map(|(_, rest)| ((), rest)) {
        // Opaque path segments (chat ids, document ids).
        for segment in path.split('?').next().unwrap_or(path).split('/') {
            if is_opaque_token(segment) && !tokens.contains(&segment.to_string()) {
                tokens.push(segment.to_string());
            }
        }
    }
    if let Some(query) = page.split_once('?').map(|(_, q)| q) {
        for pair in query.split('&') {
            if let Some((_, value)) = pair.split_once('=') {
                if is_opaque_token(value) && !tokens.contains(&value.to_string()) {
                    tokens.push(value.to_string());
                }
            }
        }
    }
    tokens
}

/// Decodes `%XX` percent-encoding. A value that decodes to a sentence was
/// never a machine token — URL-encoding merely makes prose look mixed.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 && i + 2 <= bytes.len() - 1 + 1 {
            let hex = &value[i + 1..(i + 3).min(value.len())];
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        // '+' is space in query encoding.
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Whether a value looks machine-minted rather than human-worded.
///
/// Shape only: a token is opaque when it is at least 6 characters, is not a
/// plain word (a single case-run of letters), and mixes character classes
/// (letters AND digits, or hex digits, or dashes/dots typical of UUIDs).
/// "hiking" is a word; "q2VTtop1Ztk" is a token; "PLiM-TFJI81RkXy" is a
/// token; "utm" is a word.
fn is_opaque_token(value: &str) -> bool {
    let value = percent_decode(value.trim());
    let value = value.trim();
    if value.chars().count() < 6 || value.chars().count() > 128 {
        return false;
    }
    // Decoded text containing whitespace is a sentence, not a token:
    // "s = s[1:].strip()" was never machine-minted, however it was encoded.
    if value.contains(char::is_whitespace) {
        return false;
    }
    let has_digit = value.chars().any(|c| c.is_ascii_digit());
    let has_letter = value.chars().any(|c| c.is_ascii_alphabetic());
    // Mixed-case or mixed letter/digit runs are machine-minted; anything
    // else — a word, a lowercase domain like "youtube.com", a hyphenated
    // phrase — is human-readable and NOT an identifier. Separators alone
    // prove nothing: every domain has dots.
    let mixed_case = value.chars().any(|c| c.is_ascii_uppercase())
        && value.chars().any(|c| c.is_ascii_lowercase());
    mixed_case || (has_digit && has_letter)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every test URL below is drawn from the real observation log.
    #[test]
    fn youtube_video_and_playlist_identity() {
        // watch?v=ID&list=PLAYLIST: both opaque tokens kept, order-stable.
        let a = canonical_page(
            "https://www.youtube.com/watch?v=q2VTtop1Ztk&list=PLiM-TFJI81RkXy&index=4",
        )
        .unwrap();
        let b = canonical_page("https://www.youtube.com/watch?list=PLiM-TFJI81RkXy&v=q2VTtop1Ztk")
            .unwrap();
        assert_eq!(a, b, "parameter order must not change page identity");
        assert!(a.contains("v=q2VTtop1Ztk"));
        assert!(a.contains("list=PLiM-TFJI81RkXy"));
        assert!(!a.contains("index="), "word-valued filter dropped");

        // Both opaque tokens are carried for global collection discovery.
        let tokens = opaque_tokens(&a);
        assert!(tokens.contains(&"q2VTtop1Ztk".to_string()));
        assert!(tokens.contains(&"PLiM-TFJI81RkXy".to_string()));
    }

    #[test]
    fn different_videos_on_one_host_are_different_pages() {
        let a = canonical_page("https://www.youtube.com/watch?v=eswW2BoFAUs").unwrap();
        let b = canonical_page("https://www.youtube.com/watch?v=teILJMaplT4").unwrap();
        assert_ne!(a, b, "the video id, not the host, is the identity");
    }

    #[test]
    fn chat_pages_are_path_identified() {
        let chat =
            canonical_page("https://claude.ai/chat/f5c116b5-5fea-4f64-bfda-ef61c96c886e").unwrap();
        assert_eq!(
            chat,
            "https://claude.ai/chat/f5c116b5-5fea-4f64-bfda-ef61c96c886e"
        );
        let other = canonical_page("https://claude.ai/chat/0743b1a2-other").unwrap();
        assert_ne!(chat, other, "each chat is its own page");
    }

    #[test]
    fn search_pages_keep_their_query() {
        let search = canonical_page(
            "https://www.google.com/search?q=how+to+screen+cast+mac+to+redmi+tv&rlz=1E5",
        )
        .unwrap();
        assert!(
            search.contains("q=how+to+screen+cast"),
            "the query text is the page"
        );
        assert!(!search.contains("rlz="), "tracking/nonce param dropped");
        let other = canonical_page("https://www.google.com/search?q=laptop+deals").unwrap();
        assert_ne!(search, other);
    }

    #[test]
    fn category_pages_are_path_identified() {
        // Arc'teryx category page: word-valued query params are dropped;
        // path segments (including opaque ones like wid-kjyr4dq9) are part
        // of the page address and stay.
        let a = canonical_page(
            "https://arcteryx.com/us/en/c/mens/footwear-hike/wid-kjyr4dq9?activity=trail",
        )
        .unwrap();
        assert_eq!(
            a, "https://arcteryx.com/us/en/c/mens/footwear-hike/wid-kjyr4dq9",
            "activity filter is not identity, the path is"
        );
        let b = canonical_page("https://www.arcteryx.com/us/en/c/mens/footwear-hike/").unwrap();
        assert_ne!(a, b, "different category paths are different pages");
    }

    #[test]
    fn tracking_standard_is_stripped_everywhere() {
        let a = canonical_page(
            "https://example.com/page?utm_source=newsletter&utm_campaign=hike&id=88f2a1Bc",
        )
        .unwrap();
        assert!(!a.contains("utm_"));
        assert!(a.contains("id=88f2a1Bc"), "the opaque id survives");
    }

    #[test]
    fn non_urls_are_rejected() {
        assert!(canonical_page("not a url").is_none());
        assert!(canonical_page("localhost").is_none());
        assert!(canonical_page("https://").is_none());
        assert!(canonical_page("file:///Users/a/report.md").is_none());
    }

    #[test]
    fn opaque_detection_separates_tokens_from_words() {
        assert!(is_opaque_token("q2VTtop1Ztk"));
        assert!(is_opaque_token("PLiM-TFJI81RkXy"));
        assert!(is_opaque_token("f5c116b5-5fea-4f64-bfda-ef61c96c886e"));
        assert!(is_opaque_token("6a9a869fc1ae2e6ef84e439f"));
        assert!(!is_opaque_token("hiking"));
        assert!(!is_opaque_token("products"));
        assert!(!is_opaque_token("trail"));
        assert!(!is_opaque_token("v"));
        // Domains are words, not identifiers: a dot separator must never
        // make a host look machine-minted (the mega-component regression).
        assert!(!is_opaque_token("youtube.com"));
        assert!(!is_opaque_token("music.example.com"));
        assert!(!is_opaque_token("how-to-page"));
        assert!(!is_opaque_token("playlist"));
        // URL-encoded prose decodes to a sentence, not a token: the
        // "s = s[1:].strip()" query must never look machine-minted.
        assert!(!is_opaque_token("s+%3D+s%5B1%3A%5D.strip%28%29"));
        assert!(!is_opaque_token("how%20to%20screen%20cast"));
        // Real tokens survive decoding.
        assert!(is_opaque_token("PLiM-TFJI81RkXy"));
    }
}
