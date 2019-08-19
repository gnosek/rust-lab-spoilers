use failure::Fail;
use futures::stream::Stream;
use futures::{future, stream, Future};
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

fn get<T, F, FR>(
    client: &reqwest::r#async::Client,
    url: &str,
    stats_callback: F,
) -> impl Future<Item = T, Error = CacheWarmerError>
    where
        FR: Future<Item = T, Error = CacheWarmerError>,
        F: FnOnce(Stats) -> FR + 'static,
{
    let start = Instant::now();

    client
        .get(url)
        .send()
        .map_err(CacheWarmerError::from)
        .and_then(move |resp| {
            resp.into_body()
                .fold(0, |total, chunk| {
                    future::ok::<_, reqwest::Error>(total + chunk.len())
                })
                .map_err(CacheWarmerError::from)
                .and_then(move |content_length| {
                    let elapsed_time = start.elapsed();

                    let stats = Stats {
                        elapsed_time,
                        content_length,
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
        let urls: Vec<_> = url_file
            .lines()
            .map(|line| line.map_err(CacheWarmerError::from))
            .collect();

        let client = reqwest::r#async::Client::new();
        let requests = stream::iter_result(urls.into_iter()).map(move |url| {
            get(&client, &url, |req_stats| {
                future::ok::<_, CacheWarmerError>(req_stats)
            })
        });
        let totals_fut = requests.fold(Stats::new(), |mut totals, req_stats| {
            req_stats.map(move |s| {
                totals.aggregate(&s);
                totals
            })
        });

        let mut runtime = tokio::runtime::Runtime::new().expect("Unable to create a runtime");
        let totals = runtime.block_on(totals_fut)?;

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
