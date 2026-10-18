//! The evo-threads shadow evaluator, over the real observation log.
//!
//! This is the parity instrument of the read-path unification: the engine is
//! replayed over the same evidence the ledger ingests and evaluated by the
//! same behavioral gates, under two membership readings (identity and
//! participation — see `evo_ledger::thread_gate`). On machines with Evo's
//! captured history every `cargo test` re-proves it; elsewhere it skips
//! loudly — the fixture is machine-local by nature, exactly like
//! `regression_gate.rs` beside it.
//!
//! What is asserted vs. measured:
//! * **Asserted (mechanics):** the log reads, the engine reconstructs more
//!   than zero threads, every gate is evaluated, and nothing fails on BOTH
//!   readings (that would be a real safety regression the ledger gate also
//!   reports).
//! * **Measured (parity):** divergences — where the engine's identity
//!   semantics and the ledger's co-occurrence semantics disagree — are
//!   printed. Retirement of the ledger hypothesis layer requires that list
//!   to empty on the owning machine across a real working week; Phase 1's
//!   naming/input capture is expected to shrink it by design.

use evo_ledger::thread_gate::{run_thread_gates, Verdict};

#[test]
fn thread_gate_on_the_real_log() {
    let Ok(home) = std::env::var("HOME") else {
        eprintln!("SKIPPED: no HOME; the observation log is machine-local");
        return;
    };
    let log_path = std::path::PathBuf::from(home)
        .join("Library/Application Support/evo/storage/observation.log");
    let Some(report) = run_thread_gates(&log_path) else {
        eprintln!(
            "SKIPPED: no observation log at {log_path:?} (this machine has no captured history)"
        );
        return;
    };

    println!(
        "thread gate: {} attention events, {} saves, {} declarations -> {} threads",
        report.attention_events, report.saves, report.declarations, report.thread_count
    );
    let mut identity_failures = 0;
    for (name, identity, participation) in &report.gates {
        println!(
            "  gate {name}: identity={identity:?} participation={:?}",
            participation.as_ref().map(|v| v.to_string_lossy())
        );
        if matches!(identity, Verdict::Fail(_)) {
            identity_failures += 1;
        }
    }
    let divergences = report.divergences();
    println!(
        "  divergences (identity vs participation): {}",
        if divergences.is_empty() {
            "none".to_string()
        } else {
            divergences.join("; ")
        }
    );

    assert!(
        report.thread_count > 0,
        "the engine minted no threads from a non-empty log"
    );
    assert_eq!(
        report.gates.len(),
        7,
        "every ledger gate must be evaluated over the engine"
    );
    assert!(
        report.failures().is_empty(),
        "evo-threads failed gates on BOTH membership readings (a real safety \
         regression, not a semantic divergence): {:?}",
        report.failures()
    );
    let _ = identity_failures; // reported above, not asserted: Phase-0 state
}

/// Shorthand so the print above stays readable.
trait ToStringLossy {
    fn to_string_lossy(&self) -> String;
}

impl ToStringLossy for Verdict {
    fn to_string_lossy(&self) -> String {
        match self {
            Verdict::Pass => "Pass".into(),
            Verdict::Divergence(s) => format!("Divergence({s})"),
            Verdict::Fail(s) => format!("Fail({s})"),
        }
    }
}
