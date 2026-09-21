use crate::intervals;
use crate::ObservedEvent;
use rusqlite::{params, Connection};

pub fn ingest(db: &mut Connection, events: &[ObservedEvent]) -> Result<usize, crate::LedgerError> {
    if events.is_empty() {
        return Ok(0);
    }
    let tx = db.transaction().map_err(crate::LedgerError::Sqlite)?;
    let mut intervals_created = 0;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO events (timestamp_ms, app_name, window_title, url, document_path, typed, dwell_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        ).map_err(crate::LedgerError::Sqlite)?;
        for event in events {
            stmt.execute(params![
                event.timestamp_ms as i64,
                event.app_name,
                event.window_title,
                event.url,
                event.document_path,
                event.typed as i64,
                event.dwell_ms as i64,
            ])
            .map_err(crate::LedgerError::Sqlite)?;
        }
    }
    let computed = intervals::compute_intervals(events);
    {
        let mut stmt = tx
            .prepare_cached(
                "INSERT INTO intervals
             (task_key_doc, task_key_page, task_key_title, task_key_host, task_key_app,
              confidence, start_ms, end_ms, event_count, total_dwell_ms,
              typed_count, status, search_text, urls_json, titles_json,
              keys, clicks, scrolls)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .map_err(crate::LedgerError::Sqlite)?;
        for interval in &computed {
            let search_text = build_search_text(interval);
            stmt.execute(params![
                interval.task_key.document_path,
                interval.task_key.page,
                interval.task_key.window_title,
                interval.task_key.url_host,
                interval.task_key.app_name,
                interval.task_key.confidence,
                interval.start_ms as i64,
                interval.end_ms as i64,
                interval.event_count as i64,
                interval.total_dwell_ms as i64,
                interval.typed_count as i64,
                interval.status.as_str(),
                search_text,
                interval.urls.join("\n"),
                interval.titles.join("\n"),
                interval.keys as i64,
                interval.clicks as i64,
                interval.scrolls as i64,
            ])
            .map_err(crate::LedgerError::Sqlite)?;
            intervals_created += 1;
        }
    }
    tx.commit().map_err(crate::LedgerError::Sqlite)?;
    Ok(intervals_created)
}

fn build_search_text(interval: &crate::intervals::WorkInterval) -> String {
    let mut parts: Vec<String> = Vec::new();
    for app in &interval.app_names {
        parts.push(app.clone());
    }
    if let Some(doc) = &interval.task_key.document_path {
        parts.push(doc.clone());
        if let Some(file) = doc.rsplit('/').next() {
            parts.push(file.to_string());
        }
    }
    if let Some(title) = &interval.task_key.window_title {
        parts.push(title.clone());
    }
    if let Some(host) = &interval.task_key.url_host {
        parts.push(host.clone());
    }
    parts.push(interval.task_key.app_name.clone());
    for title in &interval.titles {
        parts.push(title.clone());
    }
    for url in &interval.urls {
        parts.push(url.clone());
    }
    for doc in &interval.document_paths {
        parts.push(doc.clone());
        if let Some(file) = doc.rsplit('/').next() {
            parts.push(file.to_string());
        }
    }
    parts.join(" ")
}
