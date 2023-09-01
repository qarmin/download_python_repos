use std::collections::HashMap;
use std::fs;

use blake3::Hasher;
use jwalk::WalkDir;
use rayon::prelude::*;

use crate::DWN_PY_FILES;

const MAX_PY_FILE_SIZE: u64 = 50 * 1024;

pub fn deduplicate_packages() {
    let mut hashmap: HashMap<u64, Vec<Vec<u8>>> = HashMap::new();
    let mut files_to_remove = vec![];
    let mut path_to_check = vec![];
    println!("Collecting files");
    for i in WalkDir::new(DWN_PY_FILES)
        .skip_hidden(false)
        .into_iter()
        .flatten()
    {
        let path = i.path();
        if !path.is_file() {
            continue;
        }
        path_to_check.push(path.to_path_buf());
    }
    println!("Collected files - {}", path_to_check.len());
    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);
    let all_path = path_to_check.len();
    let hashes_to_check: Vec<_> = path_to_check
        .into_par_iter()
        .filter_map(|path| {
            let i = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if i % 10000 == 0 {
                println!("{} / {}", i, all_path);
            }
            if !path.to_string_lossy().ends_with(".py") {
                return Some((path, 0, None));
            }
            let Ok(metadata) = path.metadata() else {
                return None;
            };
            if !(1..MAX_PY_FILE_SIZE).contains(&metadata.len()) {
                return Some((path, 0, None));
            }
            let Ok(buf) = fs::read(&path) else {
                return None;
            };

            let mut hasher = Hasher::new();
            hasher.update(&buf);
            let hash_result = hasher.finalize().as_bytes().to_vec();
            Some((path, metadata.len(), Some(hash_result)))
        })
        .collect();

    println!("Got {} hashes to compare", hashes_to_check.len());
    for (path, length, hash) in hashes_to_check {
        if let Some(hash) = hash {
            if hashmap.get(&length).is_none() {
                hashmap.insert(length, vec![hash]);
                continue;
            }

            let hashes = hashmap.get_mut(&length).unwrap();
            if hashes.contains(&hash) {
                files_to_remove.push(path);
                continue;
            }
            hashes.push(hash);
        } else {
            files_to_remove.push(path);
        }
    }

    println!("Files to remove - {}", files_to_remove.len());
    files_to_remove.into_par_iter().for_each(|e| {
        let _ = fs::remove_file(e);
    });
}
