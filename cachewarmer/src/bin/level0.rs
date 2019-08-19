fn main() {
    let url_path = std::env::args().nth(1).unwrap();

    println!("Loading urls from {}", url_path);
}
