//! Dumps each work's containment surface (apps/urls/documents/titles).
fn main() {
    let home = std::env::var("HOME").unwrap();
    let path =
        std::path::PathBuf::from(home).join("Library/Application Support/evo/storage/ledger.db");
    let Ok(ledger) = evo_ledger::Ledger::open_readonly(&path) else {
        eprintln!("no ledger");
        return;
    };
    for work in ledger.works(10).unwrap_or_default() {
        println!(
            "WORK: {}",
            work.identity.chars().take(50).collect::<String>()
        );
        println!("  apps: {:?}", work.apps);
        println!(
            "  urls: {} docs: {} titles: {}",
            work.urls.len(),
            work.documents.len(),
            work.titles.len()
        );
    }
}
