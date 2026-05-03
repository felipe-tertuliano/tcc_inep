mod types;
#[macro_use]
mod macros;

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
    println!(
        "ENEM DS number of columns: {:?}",
        enem_ds
            .get_header()
            .expect("Error while fetching headers from ENEM data source")
            .len()
    );
    println!(
        "Escolas DS number of columns: {:?}",
        escolas_ds
            .get_header()
            .expect("Error while fetching headers from Escolas data source")
            .len()
    );
}
