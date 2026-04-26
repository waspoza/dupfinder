use jwalk::WalkDir;
use std::collections::HashMap;
use xxhash_rust::xxh3::xxh3_64;

fn main() {
    let dirname = std::env::args().nth(1).unwrap_or(".".to_owned());
    let mut files = HashMap::new();
    let mut dups = 0;
    for entry in WalkDir::new(&dirname).sort(false) {
        if !entry.as_ref().unwrap().file_type().is_file() {
            continue;
        }
        let path = entry.as_ref().unwrap().path();
        //println!("{}", path.display());
        let file = std::fs::File::open(&path).unwrap();
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file).unwrap() };
        let hash = xxh3_64(&mmap);
        match files.insert(hash, path.clone()) {
            Some(file) => {
                println!("{} => {} => {}", &path.display(), file.display(), hash);
                dups = dups + 1;
            }
            None => (),
        }
    }
    println!("len: {}, dups: {}", files.len(), dups);
    
}
