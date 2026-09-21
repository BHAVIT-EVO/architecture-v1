fn main() {
    let root = std::path::PathBuf::from(
        std::env::var("HOME").unwrap() + "/Library/Application Support/evo",
    );
    let engine = evo_daemon::threads::engine_from_live_data(&root).expect("engine built");
    let display = evo_daemon::threads::display_threads(&engine);
    println!("═{}═", "═".repeat(78));
    for (i, d) in display.iter().enumerate() {
        let u = &d.understanding;
        println!("WORK {}: {} — {}", i + 1, u.phase.label(), u.title);
        println!("─{}─", "─".repeat(78));
        println!("  NARRATIVE: {}", u.narrative);
        println!();
        if !u.trail.is_empty() {
            println!("  TRAIL (what you were doing):");
            for entry in &u.trail {
                println!("    • {} {}", entry.action.phrase(), entry.name);
            }
        }
        if !u.completed.is_empty() {
            println!();
            println!("  COMPLETED:");
            for item in &u.completed {
                println!("    ✓ {}", item);
            }
        }
        if !u.pending.is_empty() {
            println!();
            println!("  PENDING:");
            for item in &u.pending {
                println!("    ○ {}", item);
            }
        }
        println!();
        println!("  NEXT STEP: {}", u.next_step);
        println!("  RESTORE: {:?}", d.bundle.restore_set);
        println!("═{}═", "═".repeat(78));
    }
}
