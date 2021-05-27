use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Eq, PartialEq)]
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
    let start = Instant::now();
    let mut totals = Stats::new();
    let client = reqwest::blocking::Client::new();
    for url in url_file.lines() {
        let url = url?;
        let stats = get(&client, &url)?;
        println!(
            "{} -> {:?} ({:.2} bytes/sec)",
            url,
            stats,
            stats.bytes_per_sec().unwrap_or_default()
        );

        totals.aggregate(&stats);
    }

    println!(
        "total {:?} ({:.2} bytes/sec)",
        totals,
        totals.bytes_per_sec().unwrap_or_default()
    );

    println!("wall clock time: {:?}", start.elapsed());

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::Stats;
    use std::time::Duration;

    #[test]
    fn test_stats_aggregate() {
        let mut stats = Stats {
            elapsed_time: Duration::from_millis(500),
            content_length: 1000,
        };

        stats.aggregate(&Stats {
            elapsed_time: Duration::from_millis(1500),
            content_length: 3000,
        });

        assert_eq!(
            stats,
            Stats {
                elapsed_time: Duration::from_secs(2),
                content_length: 4000,
            }
        );
    }

    #[test]
    fn test_stats_aggregate_empty() {
        let mut stats = Stats {
            elapsed_time: Duration::from_millis(500),
            content_length: 1000,
        };

        let stats2 = stats.clone();
        stats.aggregate(&Stats::new());

        assert_eq!(stats, stats2);
    }

    #[test]
    fn test_stats_bytes_per_sec() {
        let stats = Stats {
            elapsed_time: Duration::from_millis(500),
            content_length: 1000,
        };

        assert_eq!(stats.bytes_per_sec(), Some(2000f64))
    }

    #[test]
    fn test_stats_bytes_per_sec_empty() {
        let stats = Stats::new();

        assert_eq!(stats.bytes_per_sec(), None)
    }
}
