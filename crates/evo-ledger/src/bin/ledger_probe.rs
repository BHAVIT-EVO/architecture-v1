//! Proves the ledger works end-to-end: events → intervals → works → search.
fn main() {
    let db_path = std::path::PathBuf::from(
        std::env::var("HOME").unwrap() + "/Library/Application Support/evo/evo.db",
    );
    let ledger_path = std::path::PathBuf::from(
        std::env::var("HOME").unwrap() + "/Library/Application Support/evo/ledger.db",
    );

    println!("Source: {db_path:?}");
    println!("Ledger: {ledger_path:?}");

    // Read events from the live SQLite
    let output = std::process::Command::new("sqlite3")
        .arg("-json")
        .arg(&db_path)
        .arg(
            r#"
            SELECT ts_ms, app_id, window_title,
                   COALESCE(normalized_url, '') as url,
                   COALESCE(download_path, '') as doc,
                   input_chars, dwell_ms
            FROM canonical_events
            WHERE ts_ms > (SELECT MAX(ts_ms) - 604800000 FROM canonical_events)
            ORDER BY ts_ms
        "#,
        )
        .output()
        .expect("sqlite3 read failed");

    if !output.status.success() {
        eprintln!(
            "sqlite3 failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(1);
    }

    let json = String::from_utf8_lossy(&output.stdout);

    // Parse events
    let mut events = Vec::new();
    for line in json.lines() {
        let trimmed = line.trim().trim_start_matches('[').trim_end_matches(']');
        if !trimmed.starts_with('{') {
            continue;
        }
        let ts = extract_json_u64(trimmed, "ts_ms").unwrap_or(0);
        if ts == 0 {
            continue;
        }
        let app = extract_json_str(trimmed, "app_id").unwrap_or_default();
        if app.is_empty() {
            continue;
        }
        let title = extract_json_str(trimmed, "window_title").unwrap_or_default();
        let url = extract_json_str(trimmed, "url").filter(|u| !u.is_empty());
        let doc = extract_json_str(trimmed, "doc").filter(|d| !d.is_empty());
        let typed = extract_json_u64(trimmed, "input_chars").unwrap_or(0) > 0;
        let dwell = extract_json_u64(trimmed, "dwell_ms").unwrap_or(0);
        events.push(evo_ledger::ObservedEvent {
            timestamp_ms: ts,
            app_name: app,
            window_title: title,
            url,
            document_path: doc,
            typed,
            dwell_ms: dwell.min(600000),
            keys: 0,
            clicks: 0,
            scrolls: 0,
        });
    }
    println!("Parsed {} events", events.len());

    // Create ledger, ingest
    let mut ledger = evo_ledger::Ledger::open(&ledger_path).expect("ledger open");
    let created = ledger.ingest(&events).expect("ingest");
    println!("Created {} intervals", created);

    // Get Works (grouped intervals)
    let works = ledger.works(15).expect("works");
    println!("\n{}", "═".repeat(78));
    println!("YOUR WORK ({} works)", works.len());
    println!("{}", "─".repeat(78));
    for (i, work) in works.iter().enumerate() {
        println!(
            "\n{:2}. {} [{}]",
            i + 1,
            work.title().chars().take(50).collect::<String>(),
            work.identity_kind.label()
        );
        println!(
            "    Identity: {}",
            work.identity.chars().take(40).collect::<String>()
        );
        println!(
            "    Time: {} | Sessions: {} | Apps: {}",
            work.time_summary(),
            work.session_count(),
            work.apps.len()
        );
        if !work.urls.is_empty() {
            println!("    URLs: {} (showing 3)", work.urls.len());
            for url in work.urls.iter().take(3) {
                println!("      → {}", url.chars().take(60).collect::<String>());
            }
        }
        if !work.documents.is_empty() {
            println!("    Docs: {} (showing 3)", work.documents.len());
            for doc in work.documents.iter().take(3) {
                println!("      → {}", doc.chars().take(60).collect::<String>());
            }
        }
        if work.has_production {
            println!("    ✓ Has typing (production evidence)");
        }
    }

    // Search test
    println!("\n{}", "═".repeat(78));
    println!("SEARCH");
    println!("{}", "─".repeat(78));
    for query in [
        "pitch",
        "evo",
        "framer",
        "waitlist",
        "presentation",
        "chrome",
    ] {
        let results = ledger.search(query, 3).expect("search");
        println!("\n  '{}' → {} results", query, results.len());
        for r in results.iter().take(2) {
            println!(
                "    → {} ({} events)",
                r.interval.title().chars().take(50).collect::<String>(),
                r.interval.event_count
            );
        }
    }

    // Size
    let size = ledger.size_bytes(&ledger_path);
    println!("\n{}", "═".repeat(78));
    println!(
        "Ledger: {} bytes ({:.2} MB) | {} events | {} intervals | {} works",
        size,
        size as f64 / 1e6,
        events.len(),
        created,
        works.len()
    );
}

fn extract_json_u64(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\":");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
