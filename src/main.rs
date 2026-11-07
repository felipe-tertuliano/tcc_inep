#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod types;
#[macro_use]
mod macros;
mod consts;
mod data;
mod utils;

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    rc::Rc,
};

use anyhow::Result;
use consts::{ENEM_ESCOLA, ENEM_NOTA, ESCOLAS_ID, ESCOLAS_QTS};
use data::DataSource;
use dotenv::dotenv;
use slint::Model;
use tokio::sync::mpsc as tokio_mpsc;

use crate::{data::DataItem, types::Source};

slint::include_modules!();

/* #region exe_data_mining */
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
            .is_some_and(|v| !v.is_empty()) // Students who were at a school
            && di.get::<_, i8>("TP_PRESENCA_MT").is_some_and(|v| v == 1) // Students who were at the Math exam
            && di.get::<_, i8>("TP_PRESENCA_LC").is_some_and(|v| v == 1)
        // Students who were at the Lang. & Codes exam
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
    update_progress(p_tx, 2, 1, 6, 1).await?;

    // Filter ESCOLAS data
    let mut escolas = escolas
        .filter(Some("s1_escolas_filter".to_string()), |di| {
            if di.get::<_, u8>("TP_DEPENDENCIA").is_some_and(|v| v != 4) {
                // Public schools
                Some(di)
            } else {
                None
            }
        })
        .await?;
    update_progress(p_tx, 2, 1, 6, 2).await?;

    // Standardize ESCOLAS `QTS` data
    let mut escolas = escolas
        .standardize(
            Some("s1_escolas_standardized".to_string()),
            ESCOLAS_QTS,
            ESCOLAS_ID,
        )
        .await?;
    update_progress(p_tx, 2, 1, 6, 3).await?;

    // Get the K most influential columns in ESCOLAS data
    let mut escolas_pca_ds = escolas
        .pca(Some("s1_escolas_pca".to_string()), &[ESCOLAS_ID])
        .await?;
    let mut escolas_pca = Vec::with_capacity(pca_k);
    escolas_pca_ds
        .foreach(|di| {
            if let Some(field) = di.get::<_, String>("FIELD") {
                escolas_pca.push(field);
            }
            Ok(escolas_pca.len() < pca_k)
        })
        .await?;
    drop(escolas_pca_ds);
    update_progress(p_tx, 2, 1, 6, 4).await?;

    // Identify clusters in ESCOLAS
    let mut escolas = escolas
        .kmeanspp(
            Some("s1_escolas_kmeanspp".to_string()),
            kmpp.0,
            kmpp.1,
            &escolas_pca.iter().map(|s| s.as_str()).collect(),
        )
        .await?;
    update_progress(p_tx, 2, 1, 6, 5).await?;

    // Hash ESCOLAS clusters by ID
    let mut hash = vec![];
    for e in escolas.iter_mut() {
        hash.push(e.hash(ESCOLAS_ID).await?);
    }
    update_progress(p_tx, 2, 1, 6, 6).await?;

    Ok(hash)
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
) -> Result<DataSource> {
    let mut response = DataSource::new(Source::Local("response".to_owned()))?;

    if !response.exists() {
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
        update_progress(p_tx, 2, 2, 5, 1).await?;

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
        update_progress(p_tx, 2, 2, 5, 2).await?;

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
        update_progress(p_tx, 2, 2, 5, 3).await?;

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
        update_progress(p_tx, 2, 2, 5, 4).await?;

        response.init().await?;
        response.write(true)?;
        let mut header = HashMap::new();
        header.insert("CLUSTER_CODE".to_owned(), 0);
        header.insert("CLUSTER_AVG_GRADE".to_owned(), 1);
        header.insert("ESCOLA_CODE".to_owned(), 2);
        header.insert("ESCOLA_AVG_GRADE".to_owned(), 3);
        response.set_header(&header)?;
        for (c_code, (c_avg, escolas_v)) in clusters_v.iter().enumerate() {
            for (e_code, e_avg) in escolas_v {
                response.write_item(DataItem::new(
                    types::UniRef::Ref(&header),
                    vec![
                        c_code.to_string(),
                        c_avg.to_string(),
                        e_code.to_string(),
                        e_avg.to_string(),
                    ],
                ))?;
            }
        }
        response.write(false)?;
    } else {
        response.init().await?;
    }

    update_progress(p_tx, 2, 2, 5, 5).await?;
    Ok(response)
}

async fn exe_data_mining(
    pca_k: usize,
    kmpp: (usize, usize),
    p_tx: tokio_mpsc::Sender<f32>,
    r_tx: tokio_mpsc::Sender<Vec<Vec<String>>>,
) {
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
            Ok(mut response) => {
                let mut data = vec![];
                match response
                    .foreach(|di| {
                        if let Some(vec) = di.to_vec() {
                            let row = vec
                                .iter()
                                .map(|(_, cell)| cell.to_owned())
                                .collect::<Vec<_>>();
                            data.push(row);
                        }
                        Ok(true)
                    })
                    .await
                {
                    Ok(_) => match r_tx.send(data).await {
                        Ok(_) => println!("Success"),
                        Err(err) => println!("{}", err),
                    },
                    Err(err) => println!("{}", err),
                };
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
}
/* #endregion */

/* #region sort_list */
fn sort_slvi(
    slvi: slint::ModelRc<slint::ModelRc<slint::StandardListViewItem>>,
    column: usize,
    ascending: bool,
) -> slint::ModelRc<slint::ModelRc<slint::StandardListViewItem>> {
    let mut vec_slvi = slvi.iter().collect::<Vec<_>>();
    vec_slvi.sort_by(|a, b| {
        let a_val = a
            .row_data(column)
            .map(|i| i.text.to_string())
            .unwrap_or_default();
        let b_val = b
            .row_data(column)
            .map(|i| i.text.to_string())
            .unwrap_or_default();
        let ord = match (a_val.parse::<f64>(), b_val.parse::<f64>()) {
            (Ok(na), Ok(nb)) => na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal),
            _ => a_val.cmp(&b_val),
        };
        if ascending { ord } else { ord.reverse() }
    });
    slint::ModelRc::new(slint::VecModel::from(vec_slvi))
}
/* #endregion */

fn main() -> Result<()> {
    dotenv().ok();
    let (ui_progress_tx, mut ui_progress_rx) = tokio_mpsc::channel(1);
    let (ui_response_tx, mut ui_response_rx) = tokio_mpsc::channel(1);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let ui = AppWindow::new()?;
    let dialog = CompClearDialog::new()?;

    /* #region UI interactions */
    let rt_handle = rt.handle().clone();
    let ui_handle = ui.as_weak();
    ui.on_exe_data_mining(move || {
        let rt = rt_handle.clone();
        let ui = ui_handle.unwrap();
        let progress_tx = ui_progress_tx.clone();
        let response_tx = ui_response_tx.clone();

        ui.set_state(UIState::Running);
        slint::spawn_local(async move {
            let pca_k = ui.get_pca_k().try_into().unwrap();
            let kmpp_k = ui.get_kmpp_k().try_into().unwrap();
            let kmpp_i = ui.get_kmpp_i().try_into().unwrap();
            match rt
                .spawn(async move {
                    exe_data_mining(pca_k, (kmpp_k, kmpp_i), progress_tx, response_tx).await
                })
                .await
            {
                Ok(_) => ui.set_state(UIState::Success),
                Err(_) => ui.set_state(UIState::Error),
            }
        })
        .unwrap();
    });

    ui.on_download_response(move || {
        if let Some(dest) = rfd::FileDialog::new()
            .set_file_name("response.csv")
            .add_filter("CSV Files", &["csv"])
            .save_file()
        {
            let source = PathBuf::from(std::env::var("DATA_SOURCE_PATH").unwrap()).join("response.csv");
            fs::copy(&source, &dest).unwrap();
        }
    });

    ui.on_sort_slvi(move |slvi, column, ascending| sort_slvi(slvi, column as usize, ascending));

    let ui_handle = ui.as_weak();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(1),
        move || {
            if let Some(ui) = ui_handle.upgrade() {
                if let Ok(v) = ui_progress_rx.try_recv() {
                    ui.set_progress(v);
                }
                if let Ok(rows) = ui_response_rx.try_recv() {
                    let model = slint::VecModel::default();
                    for row in rows {
                        let items: Vec<slint::StandardListViewItem> = row
                            .iter()
                            .map(|c| slint::StandardListViewItem::from(slint::format!("{}", c)))
                            .collect();
                        model.push(slint::ModelRc::from(Rc::new(slint::VecModel::from(items))));
                    }
                    ui.set_response(Rc::new(model).into());
                }
            }
        },
    );
    /* #endregion */

    let dialog_handle = dialog.as_weak();
    ui.on_open_clear_dialog(move || {
        if let Some(dialog) = dialog_handle.upgrade() {
            let os_path = PathBuf::from(std::env::var("DATA_SOURCE_PATH").unwrap());
            let model = slint::VecModel::default();
            for (f_name, _) in utils::dir_ls(os_path, Some("csv")).unwrap() {
                let item = ClearItemStruct {
                    name: f_name.into(),
                    checked: false,
                };
                model.push(item);
            }
            dialog.set_items(Rc::new(model).into());
            dialog.show().unwrap();
        }
    });

    /* #region Dialog interactions */
    let dialog_handle = dialog.as_weak();
    dialog.on_call_cancel(move || {
        if let Some(dialog) = dialog_handle.upgrade() {
            dialog.hide().unwrap();
        }
    });

    let ui_handle = ui.as_weak();
    let dialog_handle = dialog.as_weak();
    dialog.on_call_ok(move || {
        if let Some(dialog) = dialog_handle.upgrade() {
            let os_path = PathBuf::from(std::env::var("DATA_SOURCE_PATH").unwrap());
            for ci in dialog.get_items().iter() {
                if ci.checked {
                    fs::remove_file(os_path.join(&ci.name)).unwrap();
                }
            }
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_state(UIState::Ready);
                dialog.hide().unwrap();
            }
        }
    });
    /* #endregion */

    ui.run()?;

    Ok(())
}
