//! Minimal resume screen rendering for the Evo MVP.
//!
//! The UI is intentionally canonical: it renders persisted Workspace
//! understanding without inventing restoration semantics.

use evo_workspace::attachment::Attachment;
use evo_workspace::workspace::Workspace;
use evo_workspace::workspace_id::WorkspaceId;

use std::cmp::Ordering;
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

/// Picks the most recent Workspace to present on the resume screen.
pub fn select_display_workspace(workspaces: &[Workspace]) -> Option<&Workspace> {
    workspaces.iter().max_by(compare_workspaces)
}

/// Renders the MVP resume screen from canonical Workspace understanding.
pub fn render_resume_screen(workspaces: &[Workspace]) -> String {
    match select_display_workspace(workspaces) {
        Some(workspace) => render_workspace_screen(workspace, workspaces.len()),
        None => render_empty_screen(),
    }
}

/// Renders the same canonical state as a single local HTML page.
pub fn render_resume_html(workspaces: &[Workspace]) -> String {
    match select_display_workspace(workspaces) {
        Some(workspace) => render_workspace_html(workspace, workspaces.len()),
        None => render_empty_html(),
    }
}

fn render_workspace_screen(workspace: &Workspace, total_workspaces: usize) -> String {
    let mut out = String::new();
    let snapshot_count = workspace.snapshots().len();
    let latest_snapshot = workspace.snapshots().last();

    writeln!(&mut out, "Evo remembers this body of work").unwrap();
    writeln!(
        &mut out,
        "State source: durable Workspace/Snapshot storage"
    )
    .unwrap();
    writeln!(&mut out).unwrap();

    writeln!(&mut out, "Current work").unwrap();
    writeln!(
        &mut out,
        "  Workspace: {}",
        workspace_display_name(workspace.id())
    )
    .unwrap();
    writeln!(&mut out, "  Identity: {}", short_workspace_id(workspace.id())).unwrap();
    writeln!(&mut out, "  Lifecycle: {:?}", workspace.lifecycle()).unwrap();
    writeln!(
        &mut out,
        "  Artifacts: {}",
        render_artifact_list(workspace.attachments())
    )
    .unwrap();
    if total_workspaces > 1 {
        writeln!(
            &mut out,
            "  Also remembered: {} other workspace(s)",
            total_workspaces - 1
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();

    writeln!(&mut out, "Latest known state").unwrap();
    match latest_snapshot {
        Some(snapshot) => {
            writeln!(
                &mut out,
                "  Snapshot: {} of {}",
                snapshot_count,
                snapshot_count
            )
            .unwrap();
            writeln!(
                &mut out,
                "  Captured at: {}",
                format_system_time(*snapshot.captured_at())
            )
            .unwrap();
            writeln!(
                &mut out,
                "  Attachments: {}",
                render_artifact_list(snapshot.attachments())
            )
            .unwrap();
        }
        None => {
            writeln!(&mut out, "  No snapshots yet").unwrap();
        }
    }
    writeln!(&mut out).unwrap();

    writeln!(&mut out, "Snapshot history").unwrap();
    if snapshot_count == 0 {
        writeln!(&mut out, "  No historical snapshots yet").unwrap();
    } else {
        for (index, snapshot) in workspace.snapshots().iter().enumerate() {
            writeln!(
                &mut out,
                "  {}. {} | {:?} | {}",
                index + 1,
                format_system_time(*snapshot.captured_at()),
                snapshot.lifecycle(),
                render_artifact_list(snapshot.attachments())
            )
            .unwrap();
        }
    }
    writeln!(&mut out).unwrap();

    writeln!(&mut out, "Primary action").unwrap();
    writeln!(&mut out, "  Resume this workspace").unwrap();

    out
}

fn render_empty_screen() -> String {
    let mut out = String::new();
    writeln!(&mut out, "Evo remembers no body of work yet").unwrap();
    writeln!(&mut out, "State source: durable Workspace/Snapshot storage").unwrap();
    writeln!(&mut out).unwrap();
    writeln!(&mut out, "Current work").unwrap();
    writeln!(&mut out, "  No canonical Workspace has been formed yet").unwrap();
    writeln!(&mut out).unwrap();
    writeln!(&mut out, "Primary action").unwrap();
    writeln!(&mut out, "  Wait for the first canonical observation").unwrap();
    out
}

fn render_workspace_html(workspace: &Workspace, total_workspaces: usize) -> String {
    let snapshot_count = workspace.snapshots().len();
    let latest_snapshot = workspace.snapshots().last();
    let mut out = String::new();

    writeln!(
        &mut out,
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">"
    )
    .unwrap();
    writeln!(
        &mut out,
        "<meta http-equiv=\"refresh\" content=\"2\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
    )
    .unwrap();
    writeln!(
        &mut out,
        "<title>Evo remembers this body of work</title><style>{}</style></head><body>",
        html_styles()
    )
    .unwrap();
    writeln!(
        &mut out,
        "<main class=\"shell\"><section class=\"hero\"><p class=\"eyebrow\">Canonical workspace</p><h1>Evo remembers this body of work</h1><p class=\"lede\">State source: durable Workspace/Snapshot storage.</p><a class=\"primary\" href=\"#latest\">Resume this work</a></section>"
    )
    .unwrap();
    writeln!(&mut out, "<section class=\"grid\">").unwrap();
    writeln!(
        &mut out,
        "<article class=\"card\"><h2>Current work</h2><dl><dt>Workspace</dt><dd>{}</dd><dt>Identity</dt><dd>{}</dd><dt>Lifecycle</dt><dd>{:?}</dd><dt>Artifacts</dt><dd>{}</dd><dt>Other workspaces</dt><dd>{}</dd></dl></article>",
        escape_html(&workspace_display_name(workspace.id())),
        escape_html(&short_workspace_id(workspace.id())),
        workspace.lifecycle(),
        escape_html(&render_artifact_list(workspace.attachments())),
        total_workspaces.saturating_sub(1)
    )
    .unwrap();
    writeln!(&mut out, "<article class=\"card\" id=\"latest\"><h2>Latest known state</h2>").unwrap();
    match latest_snapshot {
        Some(snapshot) => {
            writeln!(
                &mut out,
                "<p class=\"stat\">Snapshot {}/{}</p><p>Captured at {}</p><p>Attachments: {}</p>",
                snapshot_count,
                snapshot_count,
                escape_html(&format_system_time(*snapshot.captured_at())),
                escape_html(&render_artifact_list(snapshot.attachments()))
            )
            .unwrap();
        }
        None => {
            writeln!(&mut out, "<p>No snapshots yet.</p>").unwrap();
        }
    }
    writeln!(&mut out, "</article>").unwrap();
    writeln!(&mut out, "<article class=\"card\"><h2>Snapshot history</h2>").unwrap();
    if snapshot_count == 0 {
        writeln!(&mut out, "<p>No historical snapshots yet.</p>").unwrap();
    } else {
        writeln!(&mut out, "<ol class=\"history\">").unwrap();
        for snapshot in workspace.snapshots() {
            writeln!(
                &mut out,
                "<li><span>{}</span><span>{:?}</span><span>{}</span></li>",
                escape_html(&format_system_time(*snapshot.captured_at())),
                snapshot.lifecycle(),
                escape_html(&render_artifact_list(snapshot.attachments()))
            )
            .unwrap();
        }
        writeln!(&mut out, "</ol>").unwrap();
    }
    writeln!(
        &mut out,
        "</article></section></main></body></html>"
    )
    .unwrap();

    out
}

fn render_empty_html() -> String {
    let mut out = String::new();
    writeln!(
        &mut out,
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Evo remembers no body of work yet</title><style>{}</style></head><body><main class=\"shell\"><section class=\"hero\"><p class=\"eyebrow\">Canonical workspace</p><h1>Evo remembers no body of work yet</h1><p class=\"lede\">State source: durable Workspace/Snapshot storage.</p><a class=\"primary\" href=\"#latest\">Wait for the first canonical observation</a></section><section class=\"grid\"><article class=\"card\" id=\"latest\"><h2>Current work</h2><p>No canonical Workspace has been formed yet.</p></article></section></main></body></html>",
        html_styles()
    )
    .unwrap();
    out
}

fn compare_workspaces(a: &&Workspace, b: &&Workspace) -> Ordering {
    display_rank(a).cmp(&display_rank(b))
}

fn display_rank(workspace: &Workspace) -> (u8, (u8, u64, u32), String) {
    let lifecycle_rank = match workspace.lifecycle() {
        evo_workspace::lifecycle::WorkspaceLifecycle::Active => 1,
        evo_workspace::lifecycle::WorkspaceLifecycle::Superseded => 0,
    };
    (
        lifecycle_rank,
        snapshot_time_key(latest_snapshot_time(workspace)),
        workspace.id().to_string(),
    )
}

fn latest_snapshot_time(workspace: &Workspace) -> SystemTime {
    workspace
        .snapshots()
        .last()
        .map(|snapshot| *snapshot.captured_at())
        .unwrap_or(UNIX_EPOCH)
}

fn workspace_display_name(id: &WorkspaceId) -> String {
    format!("Workspace {}", short_workspace_id(id))
}

fn short_workspace_id(id: &WorkspaceId) -> String {
    short_value(id.to_string().as_str())
}

fn short_artifact_id(id: &str) -> String {
    short_value(id)
}

fn short_value(value: &str) -> String {
    value.chars().take(12).collect()
}

fn render_artifact_list(attachments: &[Attachment]) -> String {
    if attachments.is_empty() {
        return "none".to_string();
    }

    attachments
        .iter()
        .map(|attachment| short_artifact_id(attachment.artifact_id().as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_system_time(time: SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("unix+{}s", duration.as_secs()),
        Err(err) => format!("unix-{}s", err.duration().as_secs()),
    }
}

fn snapshot_time_key(time: SystemTime) -> (u8, u64, u32) {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => (1, duration.as_secs(), duration.subsec_nanos()),
        Err(err) => {
            let duration = err.duration();
            (0, duration.as_secs(), duration.subsec_nanos())
        }
    }
}

fn html_styles() -> &'static str {
    " :root { color-scheme: light; --bg: #f4efe7; --panel: rgba(255,255,255,0.88); --text: #132238; --muted: #5d6b7a; --accent: #134e4a; --accent-2: #e2a93b; } body { margin: 0; min-height: 100vh; font-family: ui-serif, Georgia, 'Times New Roman', serif; color: var(--text); background: radial-gradient(circle at top left, rgba(19,78,74,0.16), transparent 32%), radial-gradient(circle at top right, rgba(226,169,59,0.16), transparent 28%), linear-gradient(180deg, #f9f4ea 0%, var(--bg) 100%); } .shell { max-width: 1040px; margin: 0 auto; padding: 48px 24px 64px; } .hero { display: grid; gap: 12px; margin-bottom: 28px; } .eyebrow { margin: 0; text-transform: uppercase; letter-spacing: 0.18em; font-size: 0.75rem; color: var(--muted); } h1 { margin: 0; font-size: clamp(2.5rem, 5vw, 4.5rem); line-height: 0.95; max-width: 10ch; } .lede { margin: 0; font-size: 1.1rem; color: var(--muted); max-width: 48rem; } .primary { display: inline-flex; align-items: center; justify-content: center; width: fit-content; padding: 14px 20px; border-radius: 999px; background: var(--accent); color: white; text-decoration: none; font-weight: 700; box-shadow: 0 12px 30px rgba(19,78,74,0.18); } .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 18px; } .card { background: var(--panel); border: 1px solid rgba(19,34,56,0.08); border-radius: 24px; padding: 22px; box-shadow: 0 18px 40px rgba(19,34,56,0.08); backdrop-filter: blur(12px); } .card h2 { margin: 0 0 12px; font-size: 1.2rem; } dl { margin: 0; display: grid; grid-template-columns: max-content 1fr; gap: 8px 14px; } dt { color: var(--muted); } dd { margin: 0; font-weight: 600; } .history { margin: 0; padding-left: 20px; display: grid; gap: 10px; } .history li { display: grid; gap: 3px; } .stat { font-size: 1.2rem; font-weight: 700; margin: 0 0 8px; }"
}

fn escape_html(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect::<Vec<_>>(),
            '>' => "&gt;".chars().collect::<Vec<_>>(),
            '"' => "&quot;".chars().collect::<Vec<_>>(),
            '\'' => "&#39;".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_workspace::attachment::{Attachment, ResourceRole};
    use evo_workspace::confidence::ConfidenceScore;
    use evo_workspace::lifecycle::WorkspaceLifecycle;
    use evo_workspace::snapshot::Snapshot;
    use evo_workspace::workspace::Workspace;
    use evo_workspace::workspace_id::WorkspaceId;
    use evo_artifact::artifact_id::ArtifactId;

    use std::str::FromStr;
    use std::time::Duration;

    fn attachment(id: &str, confidence: f32) -> Attachment {
        Attachment::new(
            ArtifactId::new(id).unwrap(),
            ConfidenceScore::new(confidence).unwrap(),
            // These fixtures exercise UI text assembly, which reads the
            // Attachment Set without regard to role; Primary is the plain
            // "part of the work" role.
            ResourceRole::Primary,
        )
    }

    fn snapshot(offset_secs: u64, attachments: Vec<Attachment>) -> Snapshot {
        Snapshot::new(
            UNIX_EPOCH.checked_add(Duration::from_secs(offset_secs)).unwrap(),
            WorkspaceLifecycle::Active,
            attachments,
        )
    }

    fn workspace(id: &str, snapshots: Vec<Snapshot>, attachments: Vec<Attachment>) -> Workspace {
        Workspace::new(
            WorkspaceId::from_str(id).unwrap(),
            WorkspaceLifecycle::Active,
            attachments,
            snapshots,
        )
    }

    #[test]
    fn render_resume_screen_shows_canonical_workspace_state() {
        let first_attachment = attachment("artifact-alpha", 0.8);
        let second_attachment = attachment("artifact-beta", 0.9);
        let workspace = workspace(
            "123e4567-e89b-12d3-a456-426614174000",
            vec![
                snapshot(10, vec![first_attachment.clone()]),
                snapshot(20, vec![first_attachment.clone(), second_attachment.clone()]),
            ],
            vec![first_attachment.clone(), second_attachment.clone()],
        );

        let rendered = render_resume_screen(&[workspace]);

        assert!(rendered.contains("Evo remembers this body of work"));
        assert!(rendered.contains("Workspace 123e4567-e89"));
        assert!(rendered.contains("Identity: 123e4567-e89"));
        assert!(rendered.contains("Artifacts: artifact-alp, artifact-bet"));
        assert!(rendered.contains("Snapshot: 2 of 2"));
        assert!(rendered.contains("Latest known state"));
        assert!(rendered.contains("Primary action"));
        assert!(rendered.contains("Resume this workspace"));
    }

    #[test]
    fn render_resume_screen_handles_empty_state() {
        let rendered = render_resume_screen(&[]);
        assert!(rendered.contains("no body of work yet"));
        assert!(rendered.contains("Wait for the first canonical observation"));
    }

    #[test]
    fn select_display_workspace_prefers_latest_snapshot_then_identity() {
        let older = workspace(
            "123e4567-e89b-12d3-a456-426614174100",
            vec![snapshot(10, vec![])],
            vec![],
        );
        let newer = workspace(
            "123e4567-e89b-12d3-a456-426614174200",
            vec![snapshot(20, vec![])],
            vec![],
        );

        let workspaces = [older.clone(), newer.clone()];
        let selected = select_display_workspace(&workspaces).unwrap();
        assert_eq!(selected.id(), newer.id());
    }

    #[test]
    fn render_resume_html_contains_primary_mvp_surface_copy() {
        let workspace = workspace(
            "123e4567-e89b-12d3-a456-426614174300",
            vec![snapshot(30, vec![attachment("artifact-gamma", 0.9)])],
            vec![attachment("artifact-gamma", 0.9)],
        );

        let rendered = render_resume_html(&[workspace]);
        assert!(rendered.contains("Evo remembers this body of work"));
        assert!(rendered.contains("Resume this work"));
        assert!(rendered.contains("Snapshot 1/1"));
    }
}
