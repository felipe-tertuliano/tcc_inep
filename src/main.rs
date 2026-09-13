#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod types;
#[macro_use]
mod macros;
mod consts;
mod data;
mod utils;

use anyhow::Result;
use consts::ESCOLAS_QTS;
use data::DataSource;
use dotenv::dotenv;

use crate::types::Source;

slint::include_modules!();

async fn exe_data_mining() {

    let mut enem = DataSource::new(Source::Remote(
        "microdados_enem_2024/DADOS/RESULTADOS_2024.csv".to_owned(),
        "https://download.inep.gov.br/microdados/microdados_enem_2024.zip".to_owned(),
    ))
    .expect("Error while creating ENEM's DataSource");

    let mut escolas = DataSource::new(Source::Remote(
        "microdados_censo_escolar_2024/microdados_censo_escolar_2024/dados/microdados_ed_basica_2024.csv".to_owned(),
        "https://download.inep.gov.br/dados_abertos/microdados_censo_escolar_2024.zip".to_owned(),
	)).expect("Error while creating Escolas's DataSource");

    match tokio::join!(enem.init(), escolas.init()) {
        (Ok(enem), Ok(escolas)) => {
            let inc_escolas = ESCOLAS_QTS.to_vec();
            match tokio::join!(
                enem.filter(Some("enem_v1"), |di| {
                    if di.get::<String>("CO_ESCOLA").is_some_and(|v| !v.is_empty())
                        && di.get::<i8>("TP_PRESENCA_MT").is_some_and(|v| v == 1)
                        && di.get::<i8>("TP_PRESENCA_LC").is_some_and(|v| v == 1)
                    {
                        Some(di)
                    } else {
                        None
                    }
                }),
                escolas.pca(Some("escolas_v1"), 10, &inc_escolas)
            ) {
                (Ok(_enem), Ok(_escolas)) => {
                    // TODO: K-means++
                }
                (enem, escolas) => {
                    if let Err(err_enem) = enem {
                        println!("{}", err_enem);
                    }
                    if let Err(err_escolas) = escolas {
                        println!("{}", err_escolas);
                    }
                }
            }
        }
        (enem, escolas) => {
            if let Err(err_enem) = enem {
                println!("{}", err_enem);
            }
            if let Err(err_escolas) = escolas {
                println!("{}", err_escolas);
            }
        }
    }
}

fn main() -> Result<()> {
    dotenv().ok();
    let ui = AppWindow::new()?;

    let ui_handle = ui.as_weak();
    ui.on_exe_data_mining(move || {
        let ui = ui_handle.unwrap();
        ui.set_progress(0.5);
    });

    ui.run()?;

    Ok(())
}
