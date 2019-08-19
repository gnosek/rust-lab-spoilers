// https://github.com/rust-lang/rust-clippy/issues/3988
#![allow(clippy::needless_lifetimes)]
#![feature(async_await)]
#![feature(async_closure)]
use failure::Fail;
use futures::compat::{Future01CompatExt, Stream01CompatExt};
use futures::stream::{FuturesOrdered, StreamExt};
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
        let elapsed_sec = self.elapsed_time.as_secs_f64();
        if elapsed_sec == 0.0 {
            return None;
        }

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

async fn get<
    T,
    FR: std::future::Future<Output = Result<T, CacheWarmerError>>,
    F: FnOnce(Duration, usize) -> FR,
>(
    client: &reqwest::r#async::Client,
    url: String,
    stats_callback: F,
) -> Result<T, CacheWarmerError> {
    let start = Instant::now();
    let resp = client.get(&url).send().compat().await?;
    let mut body = resp.into_body().compat();

    let mut length = 0;
    while let Some(chunk) = body.next().await {
        length += chunk?.len();
    }

    let elapsed = start.elapsed();

    stats_callback(elapsed, length).await
}

async fn calc_speedup<
    FR: std::future::Future<Output = Result<Duration, CacheWarmerError>>,
    F: FnOnce() -> FR,
>(
    f: F,
) -> Result<(Duration, f64), CacheWarmerError> {
    let start = Instant::now();
    let elapsed_inner = f().await?;
    let elapsed = start.elapsed();

    Ok((
        elapsed,
        (elapsed_inner.as_millis() as f64) / (elapsed.as_millis() as f64),
    ))
}

#[runtime::main(runtime_tokio::Tokio)]
async fn main() -> Result<(), CacheWarmerError> {
    let url_path = std::env::args().nth(1);
    let url_path = url_path.ok_or(CacheWarmerError::FilenameMissing)?;

    println!("Loading urls from {}", url_path);

    let url_file = BufReader::new(File::open(url_path)?);
    let urls: Result<Vec<_>, CacheWarmerError> = url_file
        .lines()
        .map(|line| line.map_err(CacheWarmerError::from))
        .collect();
    let urls = urls?;
    let client = reqwest::r#async::Client::new();

    let (elapsed, speedup) = calc_speedup(async move || {
        let mut futs = FuturesOrdered::new();

        for url in urls.into_iter() {
            futs.push(get(&client, url, async move |elapsed, content_length| {
                Ok(Stats {
                    elapsed_time: elapsed,
                    content_length,
                })
            }))
        }

        let mut totals = Stats::new();
        while let Some(req_stats) = futs.next().await {
            let req_stats = req_stats?;
            totals.aggregate(&req_stats);
        }

        println!(
            "total {:?} ({:.2} bytes/sec)",
            totals,
            totals.bytes_per_sec().unwrap_or_default()
        );
        Ok(totals.elapsed_time)
    })
    .await?;

    println!(
        "wall clock time: {} msec, speedup {:.2}x",
        elapsed.as_millis(),
        speedup
    );
    Ok(())
}
