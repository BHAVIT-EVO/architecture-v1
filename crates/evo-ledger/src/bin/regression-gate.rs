//! The regression gate as a thin human printer. The gate logic lives in
//! `evo_ledger::gate` so `cargo test` runs it too (see
//! crates/evo-ledger/tests/regression_gate.rs).

use evo_ledger::gate;
use std::path::PathBuf;

fn main() {
    let home = std::env::var("HOME").expect("HOME");
    let log_path =
        PathBuf::from(&home).join("Library/Application Support/evo/storage/observation.log");
    let Some(report) = gate::run_gates(&log_path) else {
        eprintln!("REGRESSION-GATE: no observation log at {log_path:?}");
        std::process::exit(2);
    };

    println!(
        "REGRESSION-GATE: {} attention events, {} saves, {} declared pairs",
        report.attention_events, report.saves, report.declared_pairs
    );
    println!(
        "REGRESSION-GATE: {} intervals -> {} works, {} merge proposals",
        report.intervals,
        report.works.len(),
        report.merge_proposals
    );
    for work in report.works.iter().take(25) {
        println!(
            "  [work] {} — {}s across {} sessions, {} urls, {} docs",
            work.identity.chars().take(58).collect::<String>(),
            work.total_time_ms / 1000,
            work.sessions,
            work.urls,
            work.documents
        );
    }
    println!();
    if report.failures.is_empty() {
        println!("REGRESSION-GATE: ALL GATES PASS");
    } else {
        for failure in &report.failures {
            println!("REGRESSION-GATE FAIL: {failure}");
        }
        std::process::exit(1);
    }
}
