use super::DataItem;
use crate::types::{GlobalRes, Source, UniRef};
use crate::utils::{get_csv_cols, unzip};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub type DataHeader = HashMap<String, usize>;

pub struct DataSource {
    _writer: Option<BufWriter<File>>,
    _reader: Option<BufReader<File>>,
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
            _reader: None,
            _writer: None,
            _source: source,
            _os_path: os_path,
            _is_initialized: false,
        })
    }

    pub fn child(&self, name: Option<&str>) -> GlobalRes<Self> {
        if self._is_initialized {
            Self::new(Source::Local(
                name.map(|s| s.to_string())
                    .unwrap_or(format!("{}.csv", Uuid::new_v4())),
            ))
        } else {
            other_error!("DataSource is not initialized")
        }
    }

    /* #region Helpers */
    pub fn exists(&self) -> bool {
        Path::new(&self._os_path).exists()
    }
    /* #endregion */

    /* #region Initializers */
    async fn _local_init(&mut self, _path: &str) -> GlobalRes<()> {
        File::create(&self._os_path)?;
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
        if !self.exists() {
            match &self._source.clone() {
                Source::Local(path) => self._local_init(path).await,
                Source::Remote(path, url) => self._remote_init(path, url).await,
            }?;
        }
        self._is_initialized = true;
        Ok(self)
    }
    /* #endregion */

    /* #region Readers */
    pub fn read(&mut self, on: bool, line: Option<u64>) -> GlobalRes<()> {
        if on {
            if self._reader.is_none() {
                let mut reader =
                    BufReader::new(OpenOptions::new().read(true).open(&self._os_path)?);
                if let Some(l) = line {
                    let mut i = 0;
                    let mut b = 1;
                    while b > 0 && i < l {
                        b = reader.skip_until(b'\n')?;
                        i += 1;
                    }
                }
                self._reader = Some(reader);
            }
        } else {
            self._reader = None;
        }
        Ok(())
    }

    pub fn read_line(&mut self, buf: &mut Vec<u8>, seek: Option<SeekFrom>, rewind: bool) -> GlobalRes<Option<Vec<String>>> {
        if let Some(reader) = self._reader.as_mut() {
            let mut res = Ok(None);
            let old = reader.stream_position()?;
            buf.clear();
            if let Some(pos) = seek {
                reader.seek(pos)?;
            }
            if reader.read_until(b'\n', buf)? > 0 {
                let line = get_csv_cols(String::from_utf8_lossy(buf).trim(), ';')?;
                res = Ok(Some(line));
            }
            if rewind {
                reader.seek(SeekFrom::Start(old))?;
            }
            res
        } else {
            other_error!("Read mode is not activated")
        }
    }

    pub fn get_header(&mut self) -> GlobalRes<&DataHeader> {
        if self._reader.is_none() {
            other_error!("Read mode is not activated")
        } else {
            if self._header.is_none() {
                let mut buf = vec![0; 1024];
                self._header = Some(
                    self.read_line(&mut buf, Some(SeekFrom::Start(0)), true)?
                        .expect("No header found for the DataSource")
                        .iter()
                        .enumerate()
                        .fold(HashMap::new(), |mut acc, (v, k)| {
                            acc.insert(k.to_owned(), v);
                            acc
                        }),
                );
            }
            if let Some(header) = &self._header {
                Ok(header)
            } else {
                other_error!("Unable to fetch DataSource's header")
            }
        }
    }
    /* #endregion */

    /* #region Writers */
    pub fn write(&mut self, on: bool) -> GlobalRes<()> {
        if on {
            if self._writer.is_none() {
                self._writer = Some(BufWriter::new(
                    OpenOptions::new().write(true).open(&self._os_path)?,
                ));
            }
        } else {
            self._writer = None;
        }
        Ok(())
    }

    pub fn write_line(&mut self, line: String) -> GlobalRes<()> {
        if let Some(writer) = self._writer.as_mut() {
            writeln!(writer, "{}", line)?;
            Ok(())
        } else {
            other_error!("Write mode is not activated")
        }
    }

    pub fn set_header(&mut self, header: &DataHeader) -> GlobalRes<&DataHeader> {
        if let Some(writer) = self._writer.as_mut() {
            if self._header.is_none() {
                self._header = Some(header.clone());
                let mut buf: Vec<(&String, &usize)> = header.iter().collect();
                buf.sort_by(|(_, a), (_, b)| a.cmp(b));
                let value = buf
                    .iter()
                    .map(|(v, _)| v.to_string())
                    .collect::<Vec<String>>()
                    .join(";");
                writer.rewind()?;
                self.write_line(value)?;
            }
            if let Some(header) = &self._header {
                Ok(header)
            } else {
                other_error!("Unable to fetch DataSource's header")
            }
        } else {
            other_error!("Write mode is not activated")
        }
    }
    /* #endregion */

    pub async fn foreach<F>(&mut self, mut f: F) -> GlobalRes<()>
    where
        F: FnMut(DataItem) -> GlobalRes<()>,
    {
        if self._is_initialized {
            let mut buf = vec![0; 1024];
            self.read(true, Some(1))?;
            let header = self.get_header()?.clone();
            while let Some(value) = self.read_line(&mut buf, None, false)? {
                f(DataItem::new(UniRef::Ref(&header), value))?;
            }
            self.read(false, None)?;
            Ok(())
        } else {
            other_error!("DataSource is not initialized")
        }
    }
}
