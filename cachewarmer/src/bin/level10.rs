use failure::Fail;
use std::fs::File;
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
        let elapsed_msec = self.elapsed_time.as_millis();
        if elapsed_msec == 0 {
            return None;
        }

        let elapsed_sec = (elapsed_msec as f64) / 1000.0;
        let bytes = self.content_length as f64;

        Some(bytes / elapsed_sec)
    }
}

#[derive(Debug, Fail)]
enum CacheWarmerError {
    #[fail(display = "HTTP client error: {}", _0)]
    Reqwest(reqwest::Error),

    #[fail(display = "I/O error: {}", _0)]
    Io(std::io::Error),

    #[fail(display = "File name missing")]
    FilenameMissing,
}

impl From<reqwest::Error> for CacheWarmerError {
    fn from(e: reqwest::Error) -> Self {
        CacheWarmerError::Reqwest(e)
    }
}

impl From<std::io::Error> for CacheWarmerError {
    fn from(e: std::io::Error) -> Self {
        CacheWarmerError::Io(e)
    }
}

fn get<T, F: FnOnce(Stats) -> Result<T, CacheWarmerError>>(
    client: &reqwest::Client,
    url: &str,
    stats_callback: F,
) -> Result<T, CacheWarmerError> {
    let start = Instant::now();

    client
        .get(url)
        .send()
        .map_err(CacheWarmerError::from)
        .and_then(|mut resp| {
            resp.text()
                .map_err(CacheWarmerError::from)
                .and_then(|body| {
                    let elapsed_time = start.elapsed();

                    let stats = Stats {
                        elapsed_time,
                        content_length: body.len(),
                    };

                    stats_callback(stats)
                })
        })
}

fn calc_speedup<F: FnOnce() -> Result<Duration, CacheWarmerError>>(
    f: F,
) -> Result<(Duration, f64), CacheWarmerError> {
    let start = Instant::now();
    let elapsed_inner = f()?;
    let elapsed = start.elapsed();

    Ok((
        elapsed,
        (elapsed_inner.as_millis() as f64) / (elapsed.as_millis() as f64),
    ))
}

fn main() -> Result<(), CacheWarmerError> {
    let url_path = std::env::args().nth(1);
    let url_path = url_path.ok_or(CacheWarmerError::FilenameMissing)?;

    println!("Loading urls from {}", url_path);

    let url_file = BufReader::new(File::open(url_path)?);

    let (elapsed, speedup) = calc_speedup(|| {
        let urls: Result<Vec<_>, _> = url_file.lines().collect();
        let client = reqwest::Client::new();

        let totals = urls?.iter().map(|url| get(&client, &url, Ok)).fold(
            Ok(Stats::new()),
            |a: Result<_, CacheWarmerError>, b| {
                let mut a = a?;
                a.aggregate(&b?);
                Ok(a)
            },
        )?;

        println!(
            "total {:?} ({:.2} bytes/sec)",
            totals,
            totals.bytes_per_sec().unwrap_or_default()
        );
        Ok(totals.elapsed_time)
    })?;

    println!(
        "wall clock time: {} msec, speedup {:.2}x",
        elapsed.as_millis(),
        speedup
    );
    Ok(())
}
