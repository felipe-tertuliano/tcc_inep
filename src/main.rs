mod data;
mod utils;
use data::DataSource;
use data::EnemDataSource;
use data::EscolasDataSource;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let enem_ds = EnemDataSource::init()
        .await
        .expect("Error while initiating the ENEM data source");
    let escolas_ds = EscolasDataSource::init()
        .await
        .expect("Error while initiating the Escolas data source");
    println!("Hello, world!");
}
