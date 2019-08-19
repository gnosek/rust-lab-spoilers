use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind};

fn main() -> Result<(), Error> {
    let url_path = std::env::args().nth(1);
    let url_path = url_path.ok_or(Error::new(ErrorKind::NotFound, "File name missing"))?;

    println!("Loading urls from {}", url_path);

    let url_file = BufReader::new(File::open(url_path)?);
    let urls: Result<Vec<_>, _> = url_file.lines().collect();

    println!("{:?}", urls?);
    Ok(())
}
