use jwalk::WalkDir;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek};
use std::path::PathBuf;
use xxhash_rust::xxh3::xxh3_64;

fn main() {
    let mut args = std::env::args().skip(1); // skip the executable name
    let mut dirname = ".".to_string();
    let mut dup_dir: Option<PathBuf> = None;
    let help_msg = r#"
    Duplicate Finder 1.0
    Usage: dupfinder [DIRECTORY] [OPTIONS]

    Arguments:
      [DIRECTORY]            Directory to search (default: ".")

    Options:
      -d, --dup_dir <PATH>   Move duplicates to this directory
      -h, --help             Print help information
    "#;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{}", help_msg);
                std::process::exit(0);
            }
            "-d" | "--dup_dir" => {
                dup_dir = args.next().map(PathBuf::from);
                if dup_dir.is_none() {
                    eprintln!("Error: -d/--dup_dir requires a path argument.");
                    std::process::exit(1);
                }
                if !dup_dir.as_ref().unwrap().is_dir() {
                    eprintln!("Error: -d/--dup_dir requires a valid directory.");
                    std::process::exit(1);
                }
            }
            // If it starts with '-', it's an unknown flag
            ref opt if opt.starts_with('-') => {
                eprintln!("Unknown option: {}\n{}", opt, help_msg);
                std::process::exit(1);
            }
            // Otherwise, it's our unnamed directory parameter
            path => {
                dirname = path.to_string();
            }
        }
    }

    let mut files: HashMap<u64, Vec<PathBuf>> = HashMap::new();
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

    let mut dup_counter = 0;
    let mut dup_size = 0;
    for (size, paths) in &sorted_groups {
        if paths.len() <= 1 || *size == 0 {
            continue;
        }

        // Key: Hash, Value: The original file we want to keep
        let mut hash_to_original: HashMap<u64, &PathBuf> = HashMap::new();

        for path in paths {
            if !path.exists() {
                continue;
            } // Skip if already moved

            let partial_hash = get_partial_hash(&path).unwrap_or(0);
            if partial_hash == 0 {
                println!("{} == 0", &path.display());
                std::process::exit(1);
            }

            if let Some(old_path) = hash_to_original.get(&partial_hash) {
                let full_hash_new = fs::read(&path).map(|b| xxh3_64(&b)).ok();
                let full_hash_old = fs::read(old_path).map(|b| xxh3_64(&b)).ok();
                if full_hash_new.is_some() && full_hash_new == full_hash_old {
                    println!(
                        "{} => {}, size: {}",
                        path.display(),
                        old_path.display(),
                        size
                    );

                    dup_counter += 1;
                    dup_size += size;

                    if let Some(ref dest_dir) = dup_dir {
                        let filename = path.file_name().unwrap();
                        let dest_file = dest_dir.join(filename);
                        let _ = std::fs::rename(path, dest_file);
                    }
                } else {
                    println!(
                        "❗❗❗Partial hash match, but full hash don't: {} => {}",
                        &path.display(),
                        &old_path.display()
                    );
                }
            } else {
                // This is the first time we've seen this hash for THIS size
                hash_to_original.insert(partial_hash, path);
            }
        }
    }
    println!(
        "files: {}, duplicates: {}, dup size: {}",
        file_counter,
        dup_counter,
        format_number(dup_size)
    );
}
fn get_partial_hash(path: &std::path::Path) -> Option<u64> {
    let mut file = fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let mut hash_accumulator = 0u64;

    // If file is smaller than 32KB, just hash the whole thing
    if len <= 32768 {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;
        return Some(xxh3_64(&buf));
    }

    // For big files, sample the start and end
    let mut buffer = [0u8; 16384];

    // Read start
    file.read_exact(&mut buffer).ok()?;
    hash_accumulator ^= xxh3_64(&buffer);

    // Read end
    file.seek(std::io::SeekFrom::End(-16384)).ok()?;
    file.read_exact(&mut buffer).ok()?;
    hash_accumulator ^= xxh3_64(&buffer);

    Some(hash_accumulator)
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    s.as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",")
}
