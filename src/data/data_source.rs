use super::DataItem;
use crate::types::GlobalRes;
use crate::utils::{get_csv_cols, unzip};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Error, ErrorKind, Seek, SeekFrom, Write};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;

pub type DataHeader = HashMap<String, usize>;

pub struct DataSource {
    _header: Option<DataHeader>,
    _is_initialized: bool,
    _source_path: String,
    _web_source: String,
    _path: OsString,
}

impl DataSource {
    pub fn new(web_source: &str, source_path: &str) -> GlobalRes<Self> {
        let path = PathBuf::from(&env::var("DATA_SOURCE_PATH")?)
            .join(source_path)
            .as_os_str()
            .to_owned();
        Ok(Self {
            _path: path,
            _header: None,
            _is_initialized: false,
            _web_source: web_source.to_string(),
            _source_path: source_path.to_string(),
        })
    }

    fn _read_line(
        &self,
        reader: &mut BufReader<File>,
        buf: &mut Vec<u8>,
    ) -> GlobalRes<Option<Vec<String>>> {
        let mut res = Ok(None);
        buf.clear();
        if reader.read_until(b'\n', buf)? > 0 {
            let line = get_csv_cols(String::from_utf8_lossy(&buf).trim(), ';')?;
            res = Ok(Some(line));
        }
        res
    }

    fn _get_header(&mut self, reader: &mut BufReader<File>) -> GlobalRes<&DataHeader> {
        if self._header.is_none() {
            let mut buf = vec![0; 1024];
            let p = reader.stream_position()?;
            reader.rewind()?;
            self._header = Some(
                self._read_line(reader, &mut buf)?
                    .expect("No header found for the DataSource")
                    .iter()
                    .enumerate()
                    .fold(HashMap::new(), |mut acc, (v, k)| {
                        acc.insert(k.to_owned(), v);
                        acc
                    }),
            );
            reader.seek(SeekFrom::Start(p))?;
        }

        if let Some(header) = &self._header {
            return Ok(header);
        } else {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "Unable to fetch DataSource's header",
            )
            .into());
        }
    }

    fn _get_reader(&self, at: Option<u64>) -> GlobalRes<BufReader<File>> {
        let mut reader = BufReader::new(File::open(&self._path)?);
        if let Some(end) = at {
            let mut i = 0;
            let mut b = 1;
            while b > 0 && i < end {
                b = reader.skip_until(b'\n')?;
                i += 1;
            }
        }
        Ok(reader)
    }

    pub async fn init(&mut self) -> GlobalRes<&mut Self> {
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

        if self._header.is_none() {}

        self._is_initialized = true;
        Ok(self)
    }

    pub fn filter<F>(&mut self, mut f: F) -> GlobalRes<Vec<DataItem<'_>>>
    where
        F: FnMut(&mut DataItem) -> bool,
    {
        let mut res =
            Err(Error::new(ErrorKind::NotConnected, "DataSource is not initialized").into());
        if self._is_initialized {
            let mut r = self._get_reader(Some(1))?;
            let mut filtered = vec![];
            let mut buf = vec![0; 1024];
            while let Some(line) = self._read_line(&mut r, &mut buf)? {
                let mut hash = HashMap::new();
                for (k, v) in self._get_header(&mut r)? {
                    hash.insert(k.clone(), line[*v].clone());
                }
                let mut di = DataItem::new(hash);
                if f(&mut di) {
                    filtered.push(di);
                }
                buf.clear();
            }
            res = Ok(filtered);
        }
        res
    }
}
