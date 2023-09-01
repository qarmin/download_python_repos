use jwalk::WalkDir;
use crate::DWN_PY_FILES;

pub fn remove_non_parsable_files() {
    let files_to_check = WalkDir::new(DWN_PY_FILES).into_iter().flatten().filter(|e| e.path().is_file() && e.path().extension().unwrap_or_default() == "py").map(|e| e.path().to_path_buf()).collect::<Vec<_>>();
    // dbg!(&files_to_check);
    let atomic_counter = std::sync::atomic::AtomicUsize::new(0);
    let all_to_test = files_to_check.len();
    files_to_check.into_iter().for_each(|file| {
        let i = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if i % 100 == 0 {
            println!("{} / {}", i, all_to_test);
        }
        // execute ruff with RUF001 rule to check if file is parsable
        let output = std::process::Command::new("ruff").arg("--select").arg("RUF001").arg(file.to_str().unwrap()).output().unwrap();
        // dbg!(output);
        let string_output = format!("{file:?} {} {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        if string_output.contains("Failed to parse") {
            dbg!(string_output);
        }
    })
}