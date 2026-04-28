use jwalk::WalkDir;
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

    let mut dup_db: HashMap<u64, &PathBuf> = HashMap::new();
    let mut dup_counter = 0;
    let mut dup_size = 0;
    for (size, paths) in &sorted_groups {
        if paths.len() > 1 && *size > 0 {
            for path in paths {
                //let hash = fs::read(&path).map(|bytes| xxh3_64(&bytes)).unwrap_or(0);
                let hash = get_partial_hash(&path).unwrap_or(0);
                if let Some(old_path) = dup_db.insert(hash, &path) {
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
                        if let Err(e) = std::fs::rename(path, &dest_file) {
                            println!(
                                "Error moving {} to {}: {}",
                                path.display(),
                                dest_file.display(),
                                e
                            )
                        }
                    }
                }
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
    let metadata = file.metadata().ok()?;
    let len = metadata.len();

    // Buffer for our samples
    let mut buffer = [0u8; 16384]; // 16KB

    // 1. Read the beginning
    file.read_exact(&mut buffer).ok()?;
    let mut hash = xxh3_64(&buffer);

    // 2. Read the end (if the file is big enough)
    if len > 32768 {
        file.seek(std::io::SeekFrom::End(-16384)).ok()?;
        file.read_exact(&mut buffer).ok()?;
        // Mix the end hash into the beginning hash
        hash ^= xxh3_64(&buffer);
    }

    Some(hash)
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
