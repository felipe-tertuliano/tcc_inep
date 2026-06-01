use std::io::{BufRead, BufReader, Write};
use crate::utils::{get_csv_cols, unzip};
use std::panic::AssertUnwindSafe;
use std::collections::HashMap;
use crate::types::GlobalRes;
use std::fs::{self, File};
use std::path::PathBuf;
use super::DataItem;
use std::env;

pub trait DataSource<'a>: Sized {
    fn _web_source(&self) -> String;

    fn _source_path(&self) -> String;

    fn _struct_path(&self) -> String;

    fn new() -> Self;

    fn _ref(&self) -> GlobalRes<PathBuf> {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;
        Ok(PathBuf::from(&env_dsp).join(self._source_path()))
    }

    async fn init() -> GlobalRes<Self> {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;

        let inst = Self::new();
        let path = inst._source_path();
        let full_path = PathBuf::from(&env_dsp).join(&path);

        if !full_path.exists() {
            let zip_file = format!(
                "{}.zip",
                regex::Regex::new(r"[^a-z]")?
                    .replace_all(&inst._web_source().trim().to_lowercase(), "")
            );
            let zip_path = PathBuf::from(&env_dsp).join(&zip_file);
            let mut zip_file = File::create(&zip_path)?;

            let web_source = inst._web_source();
            let res = tokio::spawn(AssertUnwindSafe(async move {
                let content = reqwest::Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .expect("Error while building the reqwest client")
                    .get(web_source)
                    .send()
                    .await
                    .expect("Error while fetching the data source from the web")
                    .bytes()
                    .await
                    .expect("Error while casting the data source into bytes");
                zip_file
                    .write_all(&content)
                    .expect("Error while saving the data source into a .zip file");
            }))
            .await;

            if res.is_ok() {
                let parent = path.split('/').next().unwrap().to_string();
                let parent_path = PathBuf::from(&env_dsp).join(parent);
                unzip(&zip_path, &parent_path)?;
            }

            fs::remove_file(&zip_path)?;
            if let Err(err) = res {
                return other_error!(err);
            }
        }

        Ok(inst)
    }

    fn filter<F>(&self, mut f: F) -> GlobalRes<Vec<DataItem<'_>>>
    where
        F: FnMut(&mut DataItem) -> bool,
    {
        let mut r = BufReader::new(File::open(
            self._ref()
                .expect("Error while fetching data source path reference"),
        )?);
        let mut res = vec![];
        let mut buf = vec![0; 1024];
        r.read_until(b'\n', &mut buf)?;
        let mut line = String::from_utf8_lossy(&buf).trim().to_string();
        buf.clear();
        if !line.is_empty() {
            let header = get_csv_cols(&line, ';')?;
            while r.read_until(b'\n', &mut buf)? > 0 {
                line = String::from_utf8_lossy(&buf).trim().to_string();
                let cols = get_csv_cols(&line, ';')?;
                let mut hash = HashMap::new();
                for (i, col) in cols.into_iter().enumerate() {
                    hash.insert(header[i].clone(), col);
                }
                let mut di = DataItem::new(hash);
                if f(&mut di) {
                    res.push(di);
                }
                buf.clear();
            }
        }
        Ok(res)
    }
}
