use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
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

fn get(client: &reqwest::blocking::Client, url: &str) -> Result<Stats, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let resp = client.get(url).send()?;
    let body = resp.text()?;
    let elapsed_time = start.elapsed();

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
    let start = Instant::now();
    let mut totals = Stats::new();
    let mut threads = Vec::new();

    let receiver = {
        let (sender, receiver) = std::sync::mpsc::channel();
        for url in url_file.lines() {
            let url = url?;
            let sender = sender.clone();
            threads.push(std::thread::spawn(move || {
                let client = reqwest::blocking::Client::new();
                let stats = get(&client, &url).unwrap();
                sender.send(stats).unwrap();
            }));
        }

        receiver
    };

    while let Ok(stats) = receiver.recv() {
        totals.aggregate(&stats);
    }

    for thread in threads.into_iter() {
        thread.join().unwrap();
    }

    println!(
        "total {:?} ({:.2} bytes/sec)",
        totals,
        totals.bytes_per_sec().unwrap_or_default()
    );

    println!("wall clock time: {:?}", start.elapsed());

    Ok(())
}