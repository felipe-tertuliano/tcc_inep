mod types;
#[macro_use]
mod macros;
mod clusters;
mod data;
mod utils;

use data::DataSource;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mut enem_off = DataSource::new(
        "https://download.inep.gov.br/microdados/microdados_enem_2024.zip",
        "microdados_enem_2024/DADOS/RESULTADOS_2024.csv",
    );
    let mut escolas_off = DataSource::new(
        "https://download.inep.gov.br/dados_abertos/microdados_censo_escolar_2024.zip",
        "microdados_censo_escolar_2024/microdados_censo_escolar_2024/dados/microdados_ed_basica_2024.csv",
    );
    let ds_inits = tokio::join!(enem_off.init(), escolas_off.init());
    if let (Ok(enem_on), Ok(_escolas_on)) = ds_inits {
        let items = enem_on
            .filter(|di| {
                di.get::<i8>("TP_PRESENCA_MT").is_some_and(|v| v == 1)
                    && di.get::<i8>("TP_PRESENCA_LC").is_some_and(|v| v == 1)
            })
            .expect("Error while filtering ENEM Data Source");
        println!("Lines: {}", items.len());
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
