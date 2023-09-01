use std::{fs, thread};
use std::collections::{BTreeSet, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use anyhow::Context;
use rayon::prelude::*;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_LENGTH;

use crate::{DWN_PACKAGES, MAX_SIZE};

pub fn download_header_check_size(client: &Client, package: &str, url: &str) -> anyhow::Result<(), anyhow::Error> {
    let response = client.head(url).send().context("Error in fetching")?;
    let Some(content_length_header) = response.headers().get(CONTENT_LENGTH) else {
        return Ok(());
    };
    let content_length = content_length_header.to_str().context("Failed to parse content length header to str")?.parse::<usize>().context("Failed to parse content length header to usize")?;

    if content_length > MAX_SIZE {
        return Err(anyhow::Error::msg(format!("Ignoring too big source files {package} - {url} - {} MB", content_length / 1024 / 1024)));
    }
    Ok(())
}


pub fn download_single_package(package: &str, url: &str) -> Result<(), anyhow::Error> {
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();

    download_header_check_size(&client, package, url)?;

    let res = client.get(url).send().context("Error in fetching")?;

    let bytes = match res.bytes() {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err(anyhow::Error::msg(format!("Failed to convert to bytes {e}")));
        }
    };

    if bytes.len() > MAX_SIZE {
        return Err(anyhow::Error::msg(format!("Too big file {}", bytes.len())));
    }

    let name = format!("{}{package}.tar.gz", DWN_PACKAGES);
    fs::write(name, bytes).unwrap();
    Ok(())
}

pub fn download_packages() {
    let _ = fs::create_dir_all(DWN_PACKAGES);

    let mut packages_to_check = fs::read_to_string("links.txt").unwrap().split('\n').filter(|e| !e.trim().is_empty()).map(|idd| {
        let mut split = idd.split(" ||||| ");
        let package = split.next().unwrap().to_string();
        let url = split.next().unwrap().to_string();

        (package, url)
    }).collect::<HashSet<_>>();
    let already_downloaded: BTreeSet<String> = fs::read_to_string("already_downloaded_zip.txt").unwrap_or_default().split('\n').map(ToString::to_string).collect::<BTreeSet<_>>();

    packages_to_check.retain(|(package, _url)| !already_downloaded.contains(package));

    let atomic_counter = AtomicUsize::new(0);
    let all_to_test = packages_to_check.len();

    // crossbeam channel
    let (tx, rx) = crossbeam_channel::unbounded();

    let thread_join = thread::spawn(move || {
        let tx = tx.clone();
        packages_to_check.into_par_iter().for_each(|(package, url)| {
            let i = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if i % 100 == 0 {
                println!("{} / {}", i, all_to_test);
            }
            match download_single_package(&package, &url) {
                Ok(_) => tx.send(package).unwrap(),
                Err(e) => println!("Error in downloading {package} - {url} - {e}")
            }
        });
    });

    let mut already_downloaded = OpenOptions::new().append(true).create(true).open("already_downloaded_zip.txt").unwrap();
    while let Ok(package) = rx.recv() {
        writeln!(already_downloaded, "{package}").unwrap();
    }

    thread_join.join().unwrap();
}