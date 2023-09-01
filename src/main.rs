use std::io::Write;

use anyhow::Context;
use rayon::prelude::*;
use crate::download_lnk::download_links;
use crate::download_pck::download_packages;

use crate::unpack_pck::unpack_packages;

mod unpack_pck;
mod download_pck;
mod download_lnk;

const MAX_SIZE: usize = 5 * 1024 * 1024;
const START_URL: &str = "../output/";

fn main() {
    // Set rayon thread number to 16
    rayon::ThreadPoolBuilder::new().num_threads(16).build_global().unwrap();

    // download_links();
    // download_packages();
    unpack_packages();
}
