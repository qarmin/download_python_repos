use std::fs;
use std::path::Path;

use jwalk::WalkDir;
use rayon::prelude::*;

use crate::DWN_PY_FILES;

pub fn remove_non_parsable_files() {
    let folders_to_check = WalkDir::new(DWN_PY_FILES)
        .skip_hidden(false)
        .max_depth(1)
        .into_iter()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<_>>();
    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);
    let all_to_test = folders_to_check.len();

    folders_to_check.into_par_iter().for_each(|folder| {
        let i = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if i % 1000 == 0 {
            println!("{} / {}", i, all_to_test);
        }
        // execute ruff with RUF001 rule to check if file is parsable
        let output = std::process::Command::new("ruff")
            .arg("--select")
            .arg("E999")
            .arg(folder.to_str().unwrap())
            .output()
            .unwrap();
        let str_err = String::from_utf8_lossy(&output.stderr);
        let mut files_to_remove = vec![];
        if str_err.contains("Failed to parse") {
            files_to_remove = str_err
                .split('\n')
                .filter_map(extract_file)
                .collect::<Vec<_>>();
        }
        if files_to_remove.is_empty() {
            return;
        }
        println!("Removing {} files", files_to_remove.len());
        for i in files_to_remove {
            let _ = fs::remove_file(i);
        }
    })
}

// Extract file from line
// error: Failed to parse /home/rafal/test/DOWNLOADED/py_files/sshstdlib-1.4/src/sshstdlib/sshtempfile.py:10:26: Unexpected token 'async'
fn extract_file(line: &str) -> Option<String> {
    let Some(new_line) = line.strip_prefix("error: Failed to parse ") else {
        return None;
    };
    let mut split = new_line.split(':');
    let file = split.next().unwrap().to_string();
    if Path::new(&file).is_file() {
        return Some(file);
    }
    None
}
