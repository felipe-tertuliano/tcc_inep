use super::DataItem;
use crate::types::{GlobalRes, MaybeMut, Source};
use crate::utils::{get_csv_cols, unzip};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Error, ErrorKind, Seek, SeekFrom, Write};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};

pub type DataHeader = HashMap<String, usize>;

pub struct DataSource {
    _header: Option<DataHeader>,
    _is_initialized: bool,
    _os_path: OsString,
    _source: Source,
    _dsp: String,
}

impl DataSource {
    pub fn new(source: Source) -> GlobalRes<Self> {
        let dsp = env::var("DATA_SOURCE_PATH")?;
        let os_path = PathBuf::from(&dsp)
            .join(match &source {
                Source::Local(p) => p,
                Source::Remote(p, _) => p,
            })
            .as_os_str()
            .to_owned();
        Ok(Self {
            _dsp: dsp,
            _header: None,
            _source: source,
            _os_path: os_path,
            _is_initialized: false,
        })
    }

    async fn _local_init(&mut self, _path: &str) -> GlobalRes<()> {
        Ok(())
    }

    async fn _remote_init(&mut self, path: &str, url: &str) -> GlobalRes<()> {
        let zip_file = format!(
            "{}.zip",
            regex::Regex::new(r"[^a-z]")?.replace_all(&url.to_lowercase(), "")
        );
        let zip_path = PathBuf::from(&self._dsp).join(&zip_file);
        let mut zip_file = File::create(&zip_path)?;

        let url_s = url.to_string();
        let res = tokio::spawn(AssertUnwindSafe(async move {
            let content = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .expect("Error while building the reqwest client")
                .get(url_s)
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
            let parent_path = PathBuf::from(&self._dsp).join(parent);
            unzip(&zip_path, &parent_path)?;
        }
        fs::remove_file(&zip_path)?;
        if let Err(err) = res {
            return other_error!(err);
        }
        Ok(())
    }

    pub async fn init(&mut self) -> GlobalRes<&mut Self> {
        if !Path::new(&self._os_path).exists() {
            match &self._source.clone() {
                Source::Local(path) => self._local_init(path).await,
                Source::Remote(path, url) => self._remote_init(path, url).await,
            }?;
        }
        self._is_initialized = true;
        Ok(self)
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

        if let Some(header) = &mut self._header {
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
        let mut reader = BufReader::new(File::open(&self._os_path)?);
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

    pub fn filter<F>(&mut self, f: F) -> GlobalRes<()>
    where
        F: for<'a> Fn(&'a DataItem) -> Option<DataItem<'a>>,
    {
        if self._is_initialized {
            let mut buf = vec![0; 1024];
            let mut r = self._get_reader(Some(1))?;
            let header = self._get_header(&mut r)?.clone();
            while let Some(value) = self._read_line(&mut r, &mut buf)? {
                let input = DataItem::new(MaybeMut::Immutable(&header), value);
                if let Some(output) = f(&input) {
                    println!("OK!");
                }
            }
            return Ok(());
        } else {
            return Err(
                Error::new(ErrorKind::NotConnected, "DataSource is not initialized").into(),
            );
        }
    }
}
