use core::hash;
use std::io::{BufRead, BufReader, Seek, Write};
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

    fn _header(&self, r: &mut BufReader<File>) -> GlobalRes<Vec<String>> {
        r.rewind();
        let mut header = String::new();
        r.read_line(&mut header)?;
        Ok(header.split(';').map(|s| s.trim().to_string()).collect())
    }

    fn filter<F>(&self, mut f: F) -> GlobalRes<Vec<DataItem>>
    where
        F: FnMut(&mut DataItem) -> bool,
    {
        let mut r = BufReader::new(File::open(
            self._ref()
                .expect("Error while fetching data source path reference"),
        )?);
        let mut res = vec![];
        let mut lines = r.lines();
        if let Some(h_line) = lines.next() {
            let header = get_csv_cols(h_line?, ';')?;
            for r_line in lines {
                let line = get_csv_cols(r_line?, ';')?;
                let mut hash = HashMap::new();
                for (i, col) in line.into_iter().enumerate() {
                    hash.insert(header[i].clone(), col);
                }
                let mut di = DataItem::new(hash);
                if f(&mut di) {
                    res.push(di);
                }
            }
        }
        Ok(res)
    }
}
