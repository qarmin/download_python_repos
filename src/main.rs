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

fn main() {
    // Set rayon thread number to 16
    rayon::ThreadPoolBuilder::new().num_threads(16).build_global().unwrap();

    // download_links();
    download_packages();
}

const MAX_SIZE: usize = 5 * 1024 * 1024;

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

pub fn download_single_package(package: &str, url: &str, save_location: &str) -> Result<(), anyhow::Error> {
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

    let name = format!("{}{package}.tar.gz", save_location);
    fs::write(name, bytes).unwrap();
    Ok(())
}

pub fn download_packages() {
    let save_location = "packages/";
    let _ = fs::create_dir_all(save_location);

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
            match download_single_package(&package, &url, save_location) {
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

pub fn download_links() {
    let mut packages_to_check = fs::read_to_string("requirements.txt").unwrap().split('\n').map(str::trim).map(ToString::to_string).collect::<HashSet<_>>();
    let already_downloaded: BTreeSet<String> = fs::read_to_string("already_downloaded.txt").unwrap_or_default().split('\n').map(ToString::to_string).collect::<BTreeSet<_>>();
    let not_downloaded_links: BTreeSet<String> = fs::read_to_string("not_downloaded.txt").unwrap_or_default().split('\n').map(ToString::to_string).collect::<BTreeSet<_>>();

    packages_to_check.retain(|x| !already_downloaded.contains(x));
    packages_to_check.retain(|x| !not_downloaded_links.contains(x));

    let atomic_counter = AtomicUsize::new(0);
    let all_to_test = packages_to_check.len();

    // crossbeam channel
    let (tx, rx) = crossbeam_channel::unbounded();

    let thread_join = thread::spawn(move || {
        let tx = tx.clone();
        packages_to_check.into_par_iter().for_each(|package| {
            let i = atomic_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if i % 100 == 0 {
                println!("{} / {}", i, all_to_test);
            }

            let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(30)).build().unwrap();
            let Ok(res) = client.get(format!("https://pypi.org/pypi/{package}/json")).send() else {
                println!("Error in fetching {package}");
                tx.send((package, None, None)).unwrap();
                return;
            };

            let Ok(js) = serde_json::from_str::<serde_json::Value>(&res.text().unwrap()) else {
                println!("Error in converting to string {package}");
                tx.send((package, None, None)).unwrap();
                return;
            };

            if js["urls"].is_null() {
                println!("Ignoring empty url {package}");
                tx.send((package, None, None)).unwrap();
                return;
            }

            let Some(urls) = js["urls"].as_array() else {
                println!("Error in setting as array {package}");
                tx.send((package, None, None)).unwrap();
                return;
            };
            let Some(source_url) = urls.iter().find(|x| x["packagetype"] == "sdist") else {
                println!("No urls found for {package}");
                tx.send((package, None, None)).unwrap();
                return;
            };

            tx.send((package, Some("".to_string()), Some(source_url["url"].as_str().unwrap().to_string()))).unwrap();
        });
    });

    let mut links_file = OpenOptions::new().append(true).create(true).open("links.txt").unwrap();
    let mut already_downloaded = OpenOptions::new().append(true).create(true).open("already_downloaded.txt").unwrap();
    let mut not_downloaded_links = OpenOptions::new().append(true).create(true).open("not_downloaded.txt").unwrap();
    while let Ok((package, version, url)) = rx.recv() {
        if (version.is_some() && url.is_none()) || (version.is_none() && url.is_some()) {
            panic!("Invalid data for {package}");
        }

        if url.is_some() && version.is_some() {
            #[allow(clippy::unnecessary_unwrap)]
            let url = url.unwrap();
            // let version = version.unwrap();
            writeln!(links_file, "{package} ||||| {url}").unwrap();
            writeln!(already_downloaded, "{package}").unwrap();
        } else {
            writeln!(not_downloaded_links, "{package}").unwrap();
        }
    }

    thread_join.join().unwrap();
}


