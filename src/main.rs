#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod types;
#[macro_use]
mod macros;
mod consts;
mod data;
mod utils;

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use consts::{ENEM_ESCOLA, ENEM_NOTA, ESCOLAS_ID, ESCOLAS_QTS};
use data::DataSource;
use dotenv::dotenv;
use tokio::sync::mpsc as tokio_mpsc;

use crate::{data::DataItem, types::Source};

slint::include_modules!();

async fn update_progress(
    p_tx: &tokio_mpsc::Sender<f32>,
    steps: usize,
    step: usize,
    tasks: usize,
    task: usize,
) -> Result<()> {
    let b = 1.0 / steps as f32;
    let total = (step as f32 / steps as f32 - b) + (task as f32 / tasks as f32) * b;
    Ok(p_tx.send(total).await?)
}

async fn step_one_enem(enem: &mut DataSource) -> Result<DataSource> {
    // Initialize DataSource
    enem.init().await?;

    // Filter ENEM data
    enem.filter(Some("s1_enem_filter".to_string()), |di| {
        if di
            .get::<_, String>("CO_ESCOLA")
            .is_some_and(|v| !v.is_empty())
            && di.get::<_, i8>("TP_PRESENCA_MT").is_some_and(|v| v == 1)
            && di.get::<_, i8>("TP_PRESENCA_LC").is_some_and(|v| v == 1)
        {
            Some(di)
        } else {
            None
        }
    })
    .await
}

async fn step_one_escola(
    escolas: &mut DataSource,
    pca_k: usize,
    kmpp: (usize, usize),
    p_tx: &tokio_mpsc::Sender<f32>,
) -> Result<Vec<HashSet<String>>> {
    // Initialize DataSource
    escolas.init().await?;
    update_progress(p_tx, 2, 1, 5, 1).await?;

    // Standardize ESCOLAS `QTS` data
    let mut escolas = escolas
        .standardize(
            Some("s1_escolas_standardized".to_string()),
            ESCOLAS_QTS,
            ESCOLAS_ID,
        )
        .await?;
    update_progress(p_tx, 2, 1, 5, 2).await?;

    // Get most influential columns in ESCOLAS data
    let escolas_pca = escolas.pca(pca_k, &[ESCOLAS_ID]).await?;
    update_progress(p_tx, 2, 1, 5, 3).await?;

    // Identify clusters in ESCOLAS
    let mut escolas = escolas
        .kmeanspp(
            Some("s1_escolas_kmeanspp".to_string()),
            kmpp.0,
            kmpp.1,
            &escolas_pca.iter().map(|s| s.as_str()).collect(),
        )
        .await?;
    update_progress(p_tx, 2, 1, 5, 4).await?;

    // Hash ESCOLAS clusters by ID
    let mut hash = vec![];
    for e in escolas.iter_mut() {
        hash.push(e.hash(ESCOLAS_ID).await?);
    }
    update_progress(p_tx, 2, 1, 5, 5).await?;

    Result::Ok(hash)
}

fn get_grade(di: &DataItem) -> u32 {
    ENEM_NOTA.iter().fold(0, |mut acc, f| {
        acc += di.get::<_, u32>(f).unwrap_or_default();
        acc
    })
}

async fn step_two(
    enem: &mut DataSource,
    clusters: Vec<HashSet<String>>,
    p_tx: &tokio_mpsc::Sender<f32>,
) -> Result<Vec<(u32, Vec<(String, u32)>)>> {
    let mut clusters_v = vec![(0, 0, HashMap::new()); clusters.len()];

    enem.foreach(|di| {
        let escola = di.get::<_, String>(ENEM_ESCOLA).unwrap();

        // Sum of grades per cluster
        if let Some((cluster, _)) = clusters
            .iter()
            .enumerate()
            .find(|(_, h)| h.contains(&escola))
            && let Some((n, v, e)) = clusters_v.get_mut(cluster)
        {
            let grade = get_grade(&di);
            *n += 1;
            *v += grade;

            // Sum of grades per escolas per cluster
            if let Some((n, v)) = e.get_mut(&escola) {
                *n += 1;
                *v += grade;
            } else {
                e.insert(escola.to_owned(), (1, grade));
            }
        }
        Ok(true)
    })
    .await?;
    update_progress(p_tx, 2, 2, 4, 1).await?;

    // Average of grades
    let clusters_v = clusters_v
        .iter()
        .map(|(n, v, e)| {
            (
                *n,
                v.checked_div(*n).unwrap_or(*v),
                e.iter()
                    .map(|(k, (n, v))| (k.to_owned(), (*n, v.checked_div(*n).unwrap_or(*v))))
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect::<Vec<_>>();
    update_progress(p_tx, 2, 2, 4, 2).await?;

    // Standard deviation per cluster
    let clusters_v = clusters_v
        .iter()
        .map(|(_, v, e)| {
            (
                (if e.is_empty() {
                    0.0
                } else {
                    e.values()
                        .map(|(_, v_i)| v_i.abs_diff(*v).pow(2))
                        .sum::<u32>() as f64
                        / e.len() as f64
                })
                .sqrt()
                .round() as u32,
                *v,
                e,
            )
        })
        .collect::<Vec<_>>();
    update_progress(p_tx, 2, 2, 4, 3).await?;

    let clusters_v = clusters_v
        .iter()
        .map(|(d, v, e)| {
            (
                *v,
                e.iter()
                    .filter(|(_, (_, v_i))| v_i.abs_diff(*v) > 2 * d)
                    .map(|(k, (_, v_i))| (k.to_owned(), *v_i))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    update_progress(p_tx, 2, 2, 4, 4).await?;

    Result::Ok(clusters_v)
}

async fn exe_data_mining(pca_k: usize, kmpp: (usize, usize), p_tx: tokio_mpsc::Sender<f32>) {
    let mut enem = DataSource::new(Source::Remote(
        "microdados_enem_2024/DADOS/RESULTADOS_2024.csv".to_owned(),
        "https://download.inep.gov.br/microdados/microdados_enem_2024.zip".to_owned(),
    ))
    .expect("Error while creating ENEM's DataSource");

    let mut escolas = DataSource::new(Source::Remote(
        "microdados_censo_escolar_2024/microdados_censo_escolar_2024/dados/microdados_ed_basica_2024.csv".to_owned(),
        "https://download.inep.gov.br/dados_abertos/microdados_censo_escolar_2024.zip".to_owned(),
	)).expect("Error while creating Escolas's DataSource");

    match tokio::join!(
        step_one_enem(&mut enem),
        step_one_escola(&mut escolas, pca_k, kmpp, &p_tx)
    ) {
        (Ok(mut enem), Ok(clusters)) => match step_two(&mut enem, clusters, &p_tx).await {
            Ok(outliers) => {
                println!("{:?}", outliers);
            }
            Err(err) => println!("{}", err),
        },
        (enem, escolas) => {
            if let Err(err_enem) = enem {
                println!("{}", err_enem);
            }
            if let Err(err_escolas) = escolas {
                println!("{}", err_escolas);
            }
        }
    }

    /* match tokio::join!(enem.init(), escolas.init()) {
        (Ok(enem), Ok(escolas)) => {
            progress_tx
                .send(0.32)
                .await
                .expect("Error trying to update the execution progress");

            match tokio::join!(
                enem.filter(Some("s1_enem_filter".to_string()), |di| {
                    if di
                        .get::<_, String>("CO_ESCOLA")
                        .is_some_and(|v| !v.is_empty())
                        && di.get::<_, i8>("TP_PRESENCA_MT").is_some_and(|v| v == 1)
                        && di.get::<_, i8>("TP_PRESENCA_LC").is_some_and(|v| v == 1)
                    {
                        Some(di)
                    } else {
                        None
                    }
                }),
                {
                    let progress_tx_c = progress_tx.clone();
                    async move {
                        match escolas
                            .standardize(
                                Some("s1_escolas_standardized".to_string()),
                                ESCOLAS_QTS,
                                ESCOLAS_ID,
                            )
                            .await
                        {
                            Ok(mut escolas_std) => {
                                progress_tx_c
                                    .send(0.48)
                                    .await
                                    .expect("Error trying to update the execution progress");
                                match escolas_std.pca(pca_k, &[ESCOLAS_ID]).await {
                                    Ok(escolas_pca) => {
                                        progress_tx_c.send(0.8).await.expect(
                                            "Error trying to update the execution progress",
                                        );
                                        match escolas_std
                                            .kmeanspp(
                                                Some("s1_escolas_kmeanspp".to_string()),
                                                kmpp.0,
                                                kmpp.1,
                                                &escolas_pca.iter().map(|s| s.as_str()).collect(),
                                            )
                                            .await
                                        {
                                            Ok(mut escolas_kmpp) => {
                                                progress_tx_c.send(0.96).await.expect(
                                                    "Error trying to update the execution progress",
                                                );
                                                let mut hash = vec![];
                                                for e in escolas_kmpp.iter_mut() {
                                                    hash.push(e.hash(ESCOLAS_ID).await?);
                                                }
                                                Result::Ok(hash)
                                            }
                                            Err(err) => Result::Err(err),
                                        }
                                    }
                                    Err(err) => Result::Err(err),
                                }
                            }
                            Err(err) => Result::Err(err),
                        }
                    }
                }
            ) {
                (Ok(mut enem), Ok(escolas)) => {
                    progress_tx
                        .send(1.0)
                        .await
                        .expect("Error trying to update the execution progress");

                    // ---
                    // Step 2: Join ENEM data with ESCOLAS data
                    // ---

                    // 2.1 - Divide ENEM data by the discovered clusters
                    let switch = enem
                        .switch(
                            Some("s2_enem_cluster".to_string()),
                            &escolas
                                .iter()
                                .map(|h| {
                                    Box::new(|di: &DataItem| {
                                        h.contains(&di.get::<_, String>(ENEM_ESCOLA).unwrap())
                                    })
                                })
                                .collect::<Vec<_>>(),
                        )
                        .await;
                    if let Ok(enem_escolas) = switch {
                    } else if let Err(err) = switch {
                        println!("{}", err);
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
        (enem, escolas) => {
            if let Err(err_enem) = enem {
                println!("{}", err_enem);
            }
            if let Err(err_escolas) = escolas {
                println!("{}", err_escolas);
            }
        }
    } */
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
            let pca_k = ui.get_pca_k().try_into().unwrap();
            let kmpp_k = ui.get_kmpp_k().try_into().unwrap();
            let kmpp_i = ui.get_kmpp_i().try_into().unwrap();
            match rt
                .spawn(async move { exe_data_mining(pca_k, (kmpp_k, kmpp_i), progress_tx).await })
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
