use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::{fs, io};

use anyhow::{Context, Error};
use flate2::read::GzDecoder;
use jwalk::WalkDir;
use tar::Archive;

use crate::{DWN_PACKAGES, DWN_PY_FILES};

pub fn unpack_packages() {
    let mut collected_files = vec![];
    for entry in WalkDir::new(DWN_PACKAGES)
        .skip_hidden(false)
        .into_iter()
        .flatten()
    {
        let path = entry.path();
        if path.extension().unwrap_or_default() != "gz" {
            continue;
        }
        collected_files.push(path.to_path_buf());
    }
    let atomic_counter = AtomicUsize::new(0);
    let all_to_test = collected_files.len();
    collected_files.into_iter().for_each(|e| {
        let i = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if i % 100 == 0 {
            println!("{} / {}", i, all_to_test);
        }
        let file_name = e.to_str().unwrap();
        let _ = process_tar_files(file_name);
    });
}

fn copy_entry_to_output(
    entry: &mut tar::Entry<GzDecoder<BufReader<File>>>,
    base_output_path: &Path,
) -> Result<(), Error> {
    let path = entry.path().context("Failed to get path")?;
    let output_path = base_output_path
        .to_path_buf()
        .join(path.strip_prefix("/").unwrap_or(&path));
    if entry.header().entry_type().is_dir() {
        fs::create_dir_all(&output_path).context(format!("Failed to create {output_path:?}"))?;
    } else {
        let extension = output_path.extension().unwrap_or_default();
        let extension = extension.to_str().unwrap_or_default();
        if extension == "py" {
            let mut output_file = File::create(output_path)?;
            io::copy(entry, &mut output_file)?;
        }
    }

    Ok(())
}

fn process_tar_files(file_name: &str) -> Result<(), Error> {
    let file = File::open(file_name)?;
    let decompressed = GzDecoder::new(BufReader::new(file));
    let mut archive = Archive::new(decompressed);

    let base_output_path = Path::new(DWN_PY_FILES);

    for entry in archive.entries()? {
        let mut entry = entry.context("Failed to get entry")?;
        copy_entry_to_output(&mut entry, base_output_path)?;
    }
    Ok(())
}
