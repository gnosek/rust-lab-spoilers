use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let url_path = std::env::args().nth(1).unwrap();

    println!("Loading urls from {}", url_path);

    let url_file = BufReader::new(File::open(url_path).unwrap());
    let mut urls = Vec::new();

    for url in url_file.lines() {
        urls.push(url.unwrap());
    }

    println!("{:?}", urls);
}
