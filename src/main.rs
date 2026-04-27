use jwalk::WalkDir;
use std::collections::HashMap;
use std::fs;
use xxhash_rust::xxh3::xxh3_64;

fn main() {
    let dirname = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());

    let mut files: HashMap<u64, Vec<std::path::PathBuf>> = HashMap::new();
    let mut file_counter = 0;
    for entry in WalkDir::new(&dirname).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let fsize = entry.metadata().unwrap().len();

            files.entry(fsize).or_default().push(entry.path());
            file_counter += 1;
        }
    }
    //dbg!(files);
    // 1. Convert to a Vec so we can sort by size
    let mut sorted_groups: Vec<_> = files.into_iter().collect();

    // 2. Sort: Biggest file size first
    sorted_groups.sort_by(|a, b| b.0.cmp(&a.0));

    let mut dup_db: HashMap<u64, &std::path::PathBuf> = HashMap::new();
    let mut dup_counter = 0;
    for (size, paths) in &sorted_groups {
        if paths.len() > 1 && *size > 0 {
            for path in paths {
                let hash = fs::read(&path).map(|bytes| xxh3_64(&bytes)).unwrap_or(0);
                if let Some(old_path) = dup_db.insert(hash, &path) {
                    println!(
                        "{} => {}, size: {}",
                        path.display(),
                        old_path.display(),
                        size
                    );
                    dup_counter += 1;
                }
            }
        }
    }
    println!("files: {}, duplicates: {}", file_counter, dup_counter);
}
