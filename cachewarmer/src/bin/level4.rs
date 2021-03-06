use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct Stats {
    elapsed_time: Duration,
    content_length: usize,
}

fn get(client: &reqwest::blocking::Client, url: &str) -> Result<Stats, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let resp = client.get(url).send()?;

    // can't rely on .content_length()
    let body = resp.text()?;
    let elapsed_time = start.elapsed();

    Ok(Stats {
        elapsed_time,
        content_length: body.len(),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url_path = std::env::args().nth(1);
    let url_path = url_path.ok_or(Error::new(ErrorKind::NotFound, "File name missing"))?;

    println!("Loading urls from {}", url_path);

    let url_file = BufReader::new(File::open(url_path)?);

    let client = reqwest::blocking::Client::new();
    for url in url_file.lines() {
        let url = url?;
        let stats = get(&client, &url)?;
        println!("{} -> {:?}", url, stats);
    }

    Ok(())
}
