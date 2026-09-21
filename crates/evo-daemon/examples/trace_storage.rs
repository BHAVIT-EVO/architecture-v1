fn main() {
    let root = evo_storage::prepare_storage_root().expect("prepare");
    println!("root: {root:?}");
    
    // Exactly what CanonicalIndex::new does:
    // 1. any_log_shrank
    let log_path = root.join("observation.log");
    let len = std::fs::metadata(&log_path).map(|m| m.len());
    println!("observation.log exists: {}, len: {:?}", log_path.exists(), len);
    
    // 2. tail_observations: set guard, read_from offset 0
    let _guard = evo_storage::Storage::with_thread_root(root.clone());
    let storage = evo_storage::Storage::new();
    match storage.read_from(evo_storage::StorageObjectKind::Observation, 0) {
        Ok((records, offset)) => {
            println!("read_from(0): {} records, next offset {}", records.len(), offset);
            
            // 3. decode each record (what tail_observations does next)
            use evo_daemon::persistence::decode_observation_record_pub;
            let mut good = 0;
            let mut bad = 0;
            for (i, record) in records.iter().enumerate() {
                match decode_observation_record_pub(record) {
                    Ok(_) => good += 1,
                    Err(e) => {
                        bad += 1;
                        if bad <= 3 {
                            println!("  record {i} decode error: {e}");
                            // Show first 80 chars of the record
                            let preview = String::from_utf8_lossy(&record[..record.len().min(80)]);
                            println!("  preview: {preview}");
                        }
                    }
                }
            }
            println!("decode: {good} good, {bad} bad");
        }
        Err(e) => println!("read_from(0) ERROR: {e}"),
    }
}
