use super::DataItem;
use crate::types::GlobalRes;
use crate::utils::{get_csv_cols, unzip};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;

pub struct DataSource<'a> {
    _marker: PhantomData<&'a ()>,
    _is_initialized: bool,
    _source_path: String,
    _web_source: String,
}

impl<'a> DataSource<'a> {
    pub fn new(web_source: &str, source_path: &str) -> Self {
        Self {
            _marker: PhantomData,
            _is_initialized: false,
            _web_source: web_source.to_string(),
            _source_path: source_path.to_string(),
        }
    }

    fn _ref(&self) -> GlobalRes<PathBuf> {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;
        Ok(PathBuf::from(&env_dsp).join(&self._source_path))
    }

    pub async fn init(&mut self) -> GlobalRes<&Self> {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;
        let full_path = PathBuf::from(&env_dsp).join(&self._source_path);

        if !full_path.exists() {
            let zip_file = format!(
                "{}.zip",
                regex::Regex::new(r"[^a-z]")?.replace_all(&self._web_source.to_lowercase(), "")
            );
            let zip_path = PathBuf::from(&env_dsp).join(&zip_file);
            let mut zip_file = File::create(&zip_path)?;

            let ws = self._web_source.clone();
            let res = tokio::spawn(AssertUnwindSafe(async move {
                let content = reqwest::Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .expect("Error while building the reqwest client")
                    .get(ws)
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
                let parent = self._source_path.split('/').next().unwrap().to_string();
                let parent_path = PathBuf::from(&env_dsp).join(parent);
                unzip(&zip_path, &parent_path)?;
            }

            fs::remove_file(&zip_path)?;
            if let Err(err) = res {
                return other_error!(err);
            }
        }

        self._is_initialized = true;
        Ok(self)
    }

    pub fn filter<F>(&self, mut f: F) -> GlobalRes<Vec<DataItem<'_>>>
    where
        F: FnMut(&mut DataItem) -> bool,
    {
        let mut res: GlobalRes<Vec<DataItem<'_>>> =
            Err(Error::new(ErrorKind::NotConnected, "DataSource is not initialized").into());
        if self._is_initialized {
            let mut r = BufReader::new(File::open(
                self._ref()
                    .expect("Error while fetching data source path reference"),
            )?);
            let mut filtered = vec![];
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
                        filtered.push(di);
                    }
                    buf.clear();
                }
            }
            res = Ok(filtered);
        }
        res
    }
}
