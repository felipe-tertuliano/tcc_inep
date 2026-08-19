mod types;
#[macro_use]
mod macros;
mod data;
mod utils;
mod consts;

use data::DataSource;
use dotenv::dotenv;
use consts::ESCOLAS_QTS;

use crate::types::Source;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mut enem_off = DataSource::new(Source::Remote(
        "microdados_enem_2024/DADOS/RESULTADOS_2024.csv".to_owned(),
        "https://download.inep.gov.br/microdados/microdados_enem_2024.zip".to_owned(),
    ))
    .expect("Error while creating ENEM's DataSource");
    // `(?<=^|;)(?!QT_)[^;]+;` to select and remove all non Qt. headers
    let mut escolas_off = DataSource::new(Source::Remote(
        "microdados_censo_escolar_2024/microdados_censo_escolar_2024/dados/microdados_ed_basica_2024.csv".to_owned(),
        "https://download.inep.gov.br/dados_abertos/microdados_censo_escolar_2024.zip".to_owned(),
	)).expect("Error while creating Escolas's DataSource");
    let ds_inits = tokio::join!(enem_off.init(), escolas_off.init());
    if let (Ok(enem_on), Ok(escolas_on)) = ds_inits {
        /* let _ = enem_on
            .filter(Some("enem_v1"), |di| {
                if di.get::<String>("CO_ESCOLA").is_some_and(|v| !v.is_empty())
                    && di.get::<i8>("TP_PRESENCA_MT").is_some_and(|v| v == 1)
                    && di.get::<i8>("TP_PRESENCA_LC").is_some_and(|v| v == 1)
                {
                    Some(di)
                } else {
                    None
                }
            })
            .await
            .inspect_err(|e| println!("{}", e)); */
        let _ = escolas_on.pca(Some("escolas_v1"), 10, &ESCOLAS_QTS.to_vec()).await.inspect_err(|e| println!("{}", e)); 
    } else {
        if let Err(err) = ds_inits.0 {
            panic!("{:?}", err);
        }
        if let Err(err) = ds_inits.1 {
            panic!("{:?}", err);
        }
    }
}
