fn main() {
    let root = std::path::PathBuf::from(
        std::env::var("HOME").unwrap() + "/Library/Application Support/evo/storage",
    );
    // observation.log lives in the storage root directory itself
    let engine = evo_daemon::threads::engine_from_storage(&root).expect("engine built");
    let threads = engine.threads();
    println!("threads minted: {}", threads.len());
    for t in &threads {
        let b = engine.resume_bundle(t.id);
        println!("thread {:>3}: episodes={} anchors={}", t.id, t.episodes.len(), t.anchors.len());
        println!("   resume: {}", b.resume_point);
        println!("   reason: {}", b.resume_reason);
        println!("   restore_set: {:?}", b.restore_set);
    }
}
