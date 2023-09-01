use std::collections::HashMap;
use std::fs;

use blake3::Hasher;
use jwalk::WalkDir;
use rayon::prelude::*;
use crate::DWN_PY_FILES;

pub fn deduplicate_packages() {
    let mut hashmap: HashMap<u64, Vec<Vec<u8>>> = HashMap::new();
    let mut files_to_remove = vec![];
    for i in WalkDir::new(DWN_PY_FILES).into_iter().flatten() {
        let path = i.path();
        if !path.is_file() || path.extension().unwrap_or_default() != "py" {
            continue;
        }
        dbg!(&path);

        let mut hasher = Hasher::new();
        let Ok(metadata) = path.metadata() else {
            continue;
        };
        if metadata.len() == 0 {
            files_to_remove.push(path.to_path_buf());
        }
        let Ok(buf) = fs::read(&path) else {
            continue;
        };

        hasher.update(&buf);
        let hash_result = hasher.finalize().as_bytes().to_vec();

        if hashmap.get(&metadata.len()).is_none() {
            hashmap.insert(metadata.len(), vec![hash_result]);
            continue;
        }

        let hashes = hashmap.get_mut(&metadata.len()).unwrap();
        if hashes.contains(&hash_result) {
            files_to_remove.push(path.to_path_buf());
            continue;
        }
        hashes.push(hash_result);
    }

    files_to_remove.iter().for_each(|e| {
        let _ = fs::remove_file(e);
    });
}
