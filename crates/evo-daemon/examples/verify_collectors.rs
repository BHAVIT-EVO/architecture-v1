//! Real-machine verification of the three new collectors.
//!
//! Starts the real daemon runtime against an isolated storage root and then
//! triggers genuine platform events:
//!
//! 1. a real file write under a non-hidden home path (OBS-FILE-SAVED);
//! 2. a real `git commit` in a scratch repository under the home directory
//!    (OBS-COMMIT-MADE);
//! 3. a real browser navigation in the default browser (OBS-URL-NAVIGATED).
//!
//! After a capture window the example prints every persisted Observation from
//! the canonical log, grouped by schema, so the run can be verified by hand.
//!
//! The scratch paths must sit under a non-hidden home path, because that is the
//! scope the FSEvents collector watches ([`evo_capture`]'s
//! `is_user_visible_path`) — a `/tmp` write is deliberately not witnessed, so
//! writing there would verify nothing. They default to `target/` inside the
//! working tree: under the home directory, not hidden, already ignored by git,
//! and conventionally scratch, so a verification run leaves nothing behind in
//! the person's own folders. Override with `EVO_VERIFY_SCRATCH`.
//!
//! Usage:
//!   cargo run -p evo-daemon --example verify_collectors
//!   EVO_VERIFY_SCRATCH=/some/other/non-hidden/home/path cargo run -p evo-daemon --example verify_collectors

use evo_daemon::persistence::load_persisted_observations;
use evo_daemon::runtime::Runtime;
use evo_observation::evidence::FactValue;

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

fn main() {
    let home = std::env::var("HOME").expect("HOME is set");
    let stamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs();
    let root = std::env::temp_dir().join(format!("evo-collector-verify-{stamp}"));

    // Where the genuine writes go. Must be under the home directory and
    // non-hidden or the FSEvents collector will not witness them at all, which
    // would make a green run meaningless.
    let scratch = std::env::var_os("EVO_VERIFY_SCRATCH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("target").join(format!("evo-collector-verify-{stamp}"))
        });
    let scratch = std::path::absolute(&scratch).unwrap_or(scratch);
    if !evo_capture::macos_fsevents::is_user_visible_path(&scratch.to_string_lossy(), &home) {
        println!(
            "scratch path {} is outside the collector's capture scope (it must be \
             under {home} and contain no hidden or Library component), so a file \
             save there would not be witnessed and this run would prove nothing.",
            scratch.display()
        );
        std::process::exit(2);
    }

    println!("== storage root: {}", root.display());
    println!("== scratch root: {}", scratch.display());

    let runtime = Runtime::new();
    let result = runtime.start_vertical_runtime_with_storage_root(root.clone());
    let (_handle, boundary) = match result {
        Ok(pair) => pair,
        Err(err) => {
            println!("runtime could not start: {err}");
            return;
        }
    };

    // ── Stage 1: real file save ────────────────────────────────────────────
    let save_dir = scratch.join("saved");
    let save_file = save_dir.join(format!("evo-collector-verify-{stamp}.txt"));
    std::fs::create_dir_all(&save_dir).expect("create scratch save dir");
    std::fs::write(&save_file, "evo collector verification\n").expect("write file");
    println!("== triggered file save: {}", save_file.display());

    // ── Stage 2: real git commit ───────────────────────────────────────────
    //
    // Every git invocation is pinned to the scratch repository. Without this,
    // a failed `git init` leaves no `.git` in the scratch directory and git's
    // ordinary discovery walks *up* the tree until it finds one — which, for a
    // scratch path inside a checkout, is the real repository. A verification
    // tool that stages and commits the user's actual working tree, and then
    // reports that repository's HEAD as the commit it "triggered", is both a
    // destructive action and fabricated evidence. `GIT_CEILING_DIRECTORIES`
    // stops the upward walk, and the explicit git dir and work tree mean no
    // command can resolve anywhere else even if the ceiling is honoured
    // differently across git versions.
    let repo_dir = scratch.join("repo");
    std::fs::create_dir_all(&repo_dir).expect("create repo dir");
    let git_dir = repo_dir.join(".git");
    let run = |args: &[&str], pinned: bool| -> std::process::Output {
        let mut command = Command::new("git");
        command
            .args(args)
            .current_dir(&repo_dir)
            .env("GIT_CEILING_DIRECTORIES", &scratch)
            // A verification run must not read the developer's own git identity
            // or hooks, so it cannot depend on machine-specific configuration.
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", &scratch);
        if pinned {
            command.env("GIT_DIR", &git_dir).env("GIT_WORK_TREE", &repo_dir);
        }
        let output = command.output().expect("git runs");
        if !output.status.success() {
            println!(
                "  git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        output
    };
    // `init` is the one command that must not be given GIT_DIR, which would
    // make it initialise the pinned path rather than the repository directory.
    // An empty template directory keeps the run independent of whatever hooks
    // and samples this machine's git installation would otherwise copy in —
    // the same reason the system config is disabled above.
    let template = scratch.join("empty-template");
    std::fs::create_dir_all(&template).expect("create empty git template dir");
    let init = run(
        &[
            "init",
            "-b",
            "main",
            &format!("--template={}", template.display()),
        ],
        false,
    );
    if !init.status.success() || !git_dir.exists() {
        // Continuing here is what produced the hazard described above.
        println!(
            "could not create the scratch repository at {} — refusing to run any \
             further git command, because with no scratch .git present git would \
             resolve to whichever repository encloses this path.",
            repo_dir.display()
        );
        println!("\nVERIFY-COLLECTORS PARTIAL (commit stage unavailable)");
        std::fs::remove_dir_all(&scratch).ok();
        std::process::exit(2);
    }
    run(&["config", "user.email", "evo-verify@example.com"], true);
    run(&["config", "user.name", "Evo Verify"], true);
    std::fs::write(repo_dir.join("work.txt"), "first line\n").ok();
    run(&["add", "."], true);
    run(&["commit", "-m", "evo collector verification commit"], true);
    let head = run(&["rev-parse", "HEAD"], true);
    let head_hash = String::from_utf8_lossy(&head.stdout).trim().to_string();
    // The hash has to have come from the scratch repository. Reporting a hash
    // read from anywhere else would be claiming a commit this run did not make.
    let scratch_head = std::fs::read_to_string(git_dir.join("logs/HEAD")).unwrap_or_default();
    if head_hash.is_empty() || !scratch_head.contains(&head_hash) {
        println!(
            "the commit stage produced no verifiable hash in {} — not reporting one",
            git_dir.join("logs/HEAD").display()
        );
        println!("\nVERIFY-COLLECTORS PARTIAL (commit stage unavailable)");
        std::fs::remove_dir_all(&scratch).ok();
        std::process::exit(2);
    }
    println!("== triggered commit: {head_hash} in {}", repo_dir.display());

    // ── Stage 3: real browser navigation ───────────────────────────────────
    // Two navigations: the first establishes the browser's current URL, the
    // second is the witnessed change the poller emits.
    let _ = Command::new("open").arg("https://example.com/").status();
    std::thread::sleep(Duration::from_secs(4));
    let _ = Command::new("open").arg("https://example.com/evo-collector").status();
    println!("== triggered browser navigation to https://example.com/evo-collector");

    // ── Capture window ─────────────────────────────────────────────────────
    println!("== capturing for 10 seconds…");
    std::thread::sleep(Duration::from_secs(10));

    // Drain the boundary for error visibility.
    while let Ok(result) = boundary.try_recv() {
        if let Err(err) = result {
            println!("boundary error: {err}");
        }
    }

    // ── Inspect the persisted canonical Observation log ────────────────────
    let observations = match load_persisted_observations(&root) {
        Ok(observations) => observations,
        Err(err) => {
            println!("could not load observations: {err}");
            return;
        }
    };

    println!("\n== persisted observations: {}", observations.len());
    let mut file_seen = false;
    let mut commit_seen = false;
    let mut url_seen = false;
    let mut file_membership_seen = false;
    let mut commit_membership_seen = false;
    for observation in &observations {
        let schema = observation.schema().name().to_string();
        let subject = observation
            .evidence()
            .facts()
            .iter()
            .find_map(|fact| match fact.value() {
                FactValue::Text(text) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let source = observation.provenance().source().as_str().to_string();
        println!("  {schema} source={source} subject={subject:?}");
        match schema.as_str() {
            "OBS-FILE-SAVED" => {
                if subject == save_file.to_string_lossy() {
                    file_seen = true;
                }
            }
            "OBS-COMMIT-MADE" => {
                if subject == head_hash {
                    commit_seen = true;
                }
            }
            "OBS-URL-NAVIGATED" => {
                if subject == "https://example.com/evo-collector" {
                    url_seen = true;
                }
            }
            "OBS-REPOSITORY-MEMBERSHIP" => {
                // RFC-0012: the member is the first subject-valued fact
                // (RepositoryMembership); the second is the repository
                // identity (Repository).
                let facts = observation.evidence().facts();
                let member = facts
                    .iter()
                    .find(|fact| fact.name() == "RepositoryMembership")
                    .and_then(|fact| match fact.value() {
                        FactValue::Text(text) => Some(text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                if member == save_file.to_string_lossy() {
                    file_membership_seen = true;
                }
                if member == head_hash {
                    commit_membership_seen = true;
                }
            }
            _ => {}
        }
    }

    println!("\n== verification summary");
    println!("  file save witnessed:  {file_seen} (expected {})", save_file.display());
    println!("  commit witnessed:     {commit_seen} (expected {head_hash})");
    println!("  url navigation seen:  {url_seen}");
    println!("  file membership:      {file_membership_seen} (RFC-0012, same repo)");
    println!("  commit membership:    {commit_membership_seen} (RFC-0012, same repo)");

    let all_ok = file_seen && commit_seen && url_seen;
    println!("\nVERIFY-COLLECTORS {}", if all_ok { "OK" } else { "PARTIAL" });

    // Clean up the whole scratch root (deletions are not witnessed), so a
    // verification run leaves nothing behind on the machine it ran on.
    std::fs::remove_dir_all(&scratch).ok();
}
