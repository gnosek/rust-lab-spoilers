use futures::stream::StreamExt;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct Stats {
    elapsed_time: Duration,
    content_length: usize,
}

impl Stats {
    fn new() -> Self {
        Stats {
            elapsed_time: Duration::default(),
            content_length: 0,
        }
    }

    fn aggregate(&mut self, other: &Stats) {
        self.elapsed_time += other.elapsed_time;
        self.content_length += other.content_length;
    }

    fn bytes_per_sec(&self) -> Option<f64> {
        let elapsed_sec = self.elapsed_time.as_secs_f64();
        if elapsed_sec < 0.001 {
            return None;
        }

        let bytes = self.content_length as f64;

        Some(bytes / elapsed_sec)
    }
}

async fn get(
    client: &reqwest::Client,
    url: String,
) -> Result<Stats, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let resp = client.get(&url).send().await?;

    // can't rely on .content_length()
    let body = resp.text().await?;
    let elapsed_time = start.elapsed();

    Ok(Stats {
        elapsed_time,
        content_length: body.len(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url_path = std::env::args().nth(1);
    let url_path = url_path.ok_or(Error::new(ErrorKind::NotFound, "File name missing"))?;

    println!("Loading urls from {}", url_path);

    let url_file = BufReader::new(File::open(url_path)?);
    let start = Instant::now();
    let mut totals = Stats::new();
    let client = reqwest::Client::new();
    let mut requests = futures::stream::FuturesUnordered::new();

    for url in url_file.lines() {
        let url = url?;
        requests.push(get(&client, url));
    }

    while let Some(stats) = requests.next().await {
        totals.aggregate(&stats?);
    }

    println!(
        "total {:?} ({:.2} bytes/sec)",
        totals,
        totals.bytes_per_sec().unwrap_or_default()
    );

    println!("wall clock time: {:?}", start.elapsed());

    Ok(())
}
