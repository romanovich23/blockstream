use wevscan::config::load_config;

#[tokio::main]
async fn main() {
    let config = load_config(Option::None);
    println!("{:?}", config);
}
