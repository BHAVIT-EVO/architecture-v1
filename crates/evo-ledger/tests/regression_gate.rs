//! The regression gate over the real observation log.
//!
//! The fixture is machine-local by nature (Evo's own captured history):
//! on machines without it the test skips — loudly, because a silent skip
//! is how a gate rots. On the machine that owns the data, every
//! `cargo test` re-proves the grouping against what the person actually
//! did.

#[test]
fn regression_gate_on_the_real_log() {
    let Ok(home) = std::env::var("HOME") else {
        eprintln!("SKIPPED: no HOME; the observation log is machine-local");
        return;
    };
    let log_path = std::path::PathBuf::from(home)
        .join("Library/Application Support/evo/storage/observation.log");
    let Some(report) = evo_ledger::gate::run_gates(&log_path) else {
        eprintln!(
            "SKIPPED: no observation log at {log_path:?} (this machine has no captured history)"
        );
        return;
    };

    println!(
        "gate: {} events -> {} intervals -> {} works, {} proposals",
        report.attention_events,
        report.intervals,
        report.works.len(),
        report.merge_proposals
    );
    for work in &report.works {
        println!("  work: {}", work.identity);
    }
    assert!(
        report.failures.is_empty(),
        "regression gate failures:\n{}",
        report.failures.join("\n")
    );
}
