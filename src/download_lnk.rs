use std::{fs, thread};
use std::collections::{BTreeSet, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use rayon::prelude::*;

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
