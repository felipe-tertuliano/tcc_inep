#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod types;
#[macro_use]
mod macros;
mod consts;
mod data;
mod utils;

use anyhow::Result;
use consts::{ESCOLAS_PCA_K, ESCOLAS_QTS};
use data::DataSource;
use dotenv::dotenv;
use tokio::sync::mpsc as tokio_mpsc;

use crate::types::Source;

slint::include_modules!();

async fn exe_data_mining(progress_tx: tokio_mpsc::Sender<f32>) {
    let mut enem = DataSource::new(Source::Remote(
        "microdados_enem_2024/DADOS/RESULTADOS_2024.csv".to_owned(),
        "https://download.inep.gov.br/microdados/microdados_enem_2024.zip".to_owned(),
    ))
    .expect("Error while creating ENEM's DataSource");

    let mut escolas = DataSource::new(Source::Remote(
        "microdados_censo_escolar_2024/microdados_censo_escolar_2024/dados/microdados_ed_basica_2024.csv".to_owned(),
        "https://download.inep.gov.br/dados_abertos/microdados_censo_escolar_2024.zip".to_owned(),
	)).expect("Error while creating Escolas's DataSource");

    progress_tx
        .send(0.33)
        .await
        .expect("Error trying to update the execution progress");

    match tokio::join!(enem.init(), escolas.init()) {
        (Ok(enem), Ok(escolas)) => {
            progress_tx
                .send(0.66)
                .await
                .expect("Error trying to update the execution progress");

            let inc_escolas = ESCOLAS_QTS.to_vec();
            match tokio::join!(
                enem.filter(Some("s1_enem_filter"), |di| {
                    if di.get::<String>("CO_ESCOLA").is_some_and(|v| !v.is_empty())
                        && di.get::<i8>("TP_PRESENCA_MT").is_some_and(|v| v == 1)
                        && di.get::<i8>("TP_PRESENCA_LC").is_some_and(|v| v == 1)
                    {
                        Some(di)
                    } else {
                        None
                    }
                }),
                async move {
                    match escolas
                        .standardize(Some("s1_escolas_standardized"), &inc_escolas)
                        .await
                    {
                        Ok(mut escolas_std) => {
                            match escolas_std.pca(ESCOLAS_PCA_K, &ESCOLAS_QTS.to_vec()).await {
                                Ok(escolas_pca) => {
                                    match escolas_std
                                        .kmeanspp(
                                            Some("s1_escolas_kmeanspp"),
                                            &escolas_pca.iter().map(|s| s.as_str()).collect(),
                                        )
                                        .await
                                    {
                                        Ok(value) => Result::Ok(value),
                                        Err(err) => Result::Err(err),
                                    }
                                }
                                Err(err) => Result::Err(err),
                            }
                        }
                        Err(err) => Result::Err(err),
                    }
                }
            ) {
                (Ok(enem), Ok(escolas)) => {
                    progress_tx
                        .send(1.0)
                        .await
                        .expect("Error trying to update the execution progress");
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
    let (ui_progress_tx, mut ui_progress_rx) = tokio_mpsc::channel::<f32>(1);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let ui = AppWindow::new()?;

    let rt_handle = rt.handle().clone();
    let ui_handle = ui.as_weak();
    ui.on_exe_data_mining(move || {
        let rt = rt_handle.clone();
        let ui = ui_handle.unwrap();
        let progress_tx = ui_progress_tx.clone();

        ui.set_state(UIState::Running);
        slint::spawn_local(async move {
            match rt
                .spawn(async move { exe_data_mining(progress_tx).await })
                .await
            {
                Ok(_) => ui.set_state(UIState::Success),
                Err(_) => ui.set_state(UIState::Error),
            }
        })
        .unwrap();
    });

    let ui_handle = ui.as_weak();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(1),
        move || {
            if let Some(ui) = ui_handle.upgrade()
                && let Ok(v) = ui_progress_rx.try_recv()
            {
                ui.set_progress(v);
            }
        },
    );

    ui.run()?;

    Ok(())
}
