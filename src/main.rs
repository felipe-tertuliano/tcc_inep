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
    let ds_inits = tokio::join!(EnemDataSource::init(), EscolasDataSource::init());
    if let (Ok(enem_ds), Ok(escolas_ds)) = ds_inits {
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
    } else {
        if let Err(err) = ds_inits.0 {
            panic!("{:?}", err);
        }
        if let Err(err) = ds_inits.1 {
            panic!("{:?}", err);
        }
    }
}
