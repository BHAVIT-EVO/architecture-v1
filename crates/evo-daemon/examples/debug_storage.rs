fn main() {
    let root = evo_storage::prepare_storage_root().expect("prepare");
    println!("storage root: {root:?}");
    
    // Test 1: direct Storage read (with guard)
    {
        let _guard = evo_storage::Storage::with_thread_root(root.clone());
        let storage = evo_storage::Storage::new();
        println!("Storage::new(): OK");
        match storage.read_all(evo_storage::StorageObjectKind::Observation) {
            Ok(r) => println!("read_all: {} records", r.len()),
            Err(e) => println!("read_all ERROR: {e}"),
        }
    }
    
    // Test 2: CanonicalIndex::new
    match evo_daemon::cache::CanonicalIndex::new(root.clone()) {
        Ok(_) => println!("CanonicalIndex::new: OK"),
        Err(e) => println!("CanonicalIndex::new ERROR: {e}"),
    }
    
    // Test 3: engine_from_live_data  
    match evo_daemon::threads::engine_from_live_data(&root) {
        Ok(engine) => {
            let display = evo_daemon::threads::display_threads(&engine);
            println!("engine_from_live_data: OK ({} display threads)", display.len());
            for d in &display {
                println!("  [{}] {}", d.name, d.bundle.resume_reason);
            }
        }
        Err(e) => println!("engine_from_live_data ERROR: {e}"),
    }
}
