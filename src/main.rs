use dotenv::dotenv;

mod data;
mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();
    println!("Hello, world!");
}
