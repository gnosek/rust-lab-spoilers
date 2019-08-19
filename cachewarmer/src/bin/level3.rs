use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url_path = std::env::args().nth(1);
    let url_path =
        url_path.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "File name missing"))?;

    println!("Loading urls from {}", url_path);

    let url_file = BufReader::new(File::open(url_path)?);

    let client = reqwest::Client::new();
    for url in url_file.lines() {
        let url = url?;
        let resp = client.get(&url).send()?;
        println!("GET {} -> {}", url, resp.status());
    }

    Ok(())
}
