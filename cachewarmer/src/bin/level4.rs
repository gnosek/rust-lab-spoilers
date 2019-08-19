use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct Stats {
    elapsed_time: Duration,
    content_length: usize,
}

fn get(client: &reqwest::Client, url: &str) -> Result<Stats, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let mut resp = client.get(url).send()?;
    let elapsed_time = start.elapsed();

    // can't rely on .content_length()
    let body = resp.text()?;

    Ok(Stats {
        elapsed_time,
        content_length: body.len(),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url_path = std::env::args().nth(1);
    let url_path =
        url_path.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "File name missing"))?;

    println!("Loading urls from {}", url_path);

    let url_file = BufReader::new(File::open(url_path)?);

    let client = reqwest::Client::new();
    for url in url_file.lines() {
        let url = url?;
        let stats = get(&client, &url)?;
        println!("{} -> {:?}", url, stats);
    }

    Ok(())
}
