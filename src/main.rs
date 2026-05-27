mod types;
#[macro_use]
mod macros;
mod clusters;
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
        // let items = enem_ds.filter();
        // let value = item.get("NAME");
    } else {
        if let Err(err) = ds_inits.0 {
            panic!("{:?}", err);
        }
        if let Err(err) = ds_inits.1 {
            panic!("{:?}", err);
        }
    }
}
