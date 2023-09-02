use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Error;
use jwalk::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

use crate::{DWN_PACKED_FILES, DWN_PY_FILES};

const PACKED_FILE_IDX: &str = "FILES_PPP.zip";

const MAX_CHUNKS: usize = 1_000_000;
const SMALL_LIMIT: usize = 50_000; // Files that can be easily used to generate

pub fn pack_files() {
    let files = WalkDir::new(DWN_PY_FILES).skip_hidden(false).into_iter().flatten().filter_map(|e| {
        let Ok(metadata) = e.metadata() else {
            return None;
        };
        if metadata.is_file() {
            return Some(e.path().to_path_buf());
        }
        return None;
    }).collect::<Vec<_>>();

    println!("Collected files");
    for (idx, chunk) in files.chunks(MAX_CHUNKS).enumerate() {
        println!("Packing chunk {}/{}", idx + 1 , files.len() / MAX_CHUNKS + 1);
        let res = pack_simple_archive(chunk, idx + 1);
        if res.is_err() {
            dbg!(&res.unwrap_err());
        }
    }
    println!("Saving last chunk");
    let res = pack_simple_archive(&files[..min(SMALL_LIMIT, files.len())], 999);
    if res.is_err() {
        dbg!(&res.unwrap_err());
    }
}

fn pack_simple_archive(files_to_check: &[PathBuf], idx: usize) -> Result<(), Error> {
    let zip_filename = format!("{}{}", DWN_PACKED_FILES, PACKED_FILE_IDX.replace("PPP", &idx.to_string()));
    let zip_file = File::create(&zip_filename)?;
    let mut zip_writer = ZipWriter::new(zip_file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for (idx, file_to_zip) in files_to_check.into_iter().enumerate() {
        let Ok(file_content) = std::fs::read(&file_to_zip) else {
            continue;
        };
        let file_name = format!("{}/{}.py", idx / 1000, idx % 1000);

        let _ = zip_writer.start_file(file_name, options);
        let _ = zip_writer.write_all(&file_content);
    }

    zip_writer.finish()?;

    println!("Pliki zostały spakowane do {zip_filename}");
    Ok(())
}
