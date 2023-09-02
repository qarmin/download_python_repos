use crate::deduplicate_py_files::deduplicate_packages;
use crate::download_lnk::download_links;
use crate::download_pck::download_packages;
use crate::remove_non_parsable_python_files::remove_non_parsable_files;
use crate::save_zip::pack_files;
use crate::unpack_pck::unpack_packages;

mod deduplicate_py_files;
mod download_lnk;
mod download_pck;
mod remove_non_parsable_python_files;
mod unpack_pck;
mod save_zip;

const MAX_SIZE: usize = 5 * 1024 * 1024;

const DWN_PACKAGES: &str = "/home/rafal/test/DOWNLOADED/packages/";
const DWN_PY_FILES: &str = "/home/rafal/test/DOWNLOADED/py_files/";
const DWN_PACKED_FILES: &str = "/home/rafal/test/DOWNLOADED/packed_files/";
// const DWN_LINKS: &str = "/home/rafal/test/DOWNLOADED/links.txt";
// const DWN_ALREADY_DOWNLOADED_ZIP: &str = "/home/rafal/test/DOWNLOADED/already_downloaded_zip.txt";


fn main() {
    // Set rayon thread number to 16
    rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build_global()
        .unwrap();

    let _ = pack_files();
    if false {
        download_links();
        download_packages();

        unpack_packages();
        deduplicate_packages();
        remove_non_parsable_files();
        let _ = pack_files();
    }
}
