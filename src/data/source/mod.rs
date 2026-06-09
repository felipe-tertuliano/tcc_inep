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
    fn _get_reader(&self, line: Option<u64>) -> GlobalRes<BufReader<File>> {
        let mut reader = BufReader::new(OpenOptions::new().read(true).open(&self._os_path)?);
        if let Some(l) = line {
            let mut i = 0;
            let mut b = 1;
            while b > 0 && i < l {
                b = reader.skip_until(b'\n')?;
                i += 1;
            }
        }
        Ok(reader)
    }

    fn _read_line(
        &self,
        reader: &mut BufReader<File>,
        buf: &mut Vec<u8>,
    ) -> GlobalRes<Option<Vec<String>>> {
        let mut res = Ok(None);
        buf.clear();
        if reader.read_until(b'\n', buf)? > 0 {
            let line = get_csv_cols(String::from_utf8_lossy(buf).trim(), ';')?;
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
            Ok(header)
        } else {
            other_error!("Unable to fetch DataSource's header")
        }
    }
    /* #endregion */

    /* #region Writers */
    fn _get_writer(&self) -> GlobalRes<BufWriter<File>> {
        Ok(BufWriter::new(OpenOptions::new().write(true).open(&self._os_path)?))
    }

    fn _write_line(&self, writer: &mut BufWriter<File>, line: String) -> GlobalRes<()> {
        writeln!(writer, "{}", line)?;
        Ok(())
    }

    fn _set_header(
        &mut self,
        writer: &mut BufWriter<File>,
        header: &DataHeader,
    ) -> GlobalRes<&DataHeader> {
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
            self._write_line(writer, value)?;
        }

        if let Some(header) = &self._header {
            Ok(header)
        } else {
            other_error!("Unable to fetch DataSource's header")
        }
    }
    /* #endregion */

    pub async fn filter<F>(&mut self, to: Option<&str>, f: F) -> GlobalRes<Self>
    where
        F: Fn(DataItem) -> Option<DataItem>,
    {
        if self._is_initialized {
            let mut filtered = Self::new(Source::Local(
                to.map(|s| s.to_string())
                    .unwrap_or(format!("{}.csv", Uuid::new_v4())),
            ))?;
            if !filtered.exists() {
                filtered.init().await?;
                let mut buf = vec![0; 1024];
                let mut w = filtered._get_writer()?;
                let mut r = self._get_reader(Some(1))?;
                let header = self._get_header(&mut r)?.clone();
                while let Some(value) = self._read_line(&mut r, &mut buf)? {
                    if let Some(output) = f(DataItem::new(UniRef::Ref(&header), value))
                        && let Some(output_h) = output.get_header()
                    {
                        filtered._set_header(&mut w, output_h)?;
                        filtered._write_line(&mut w, output.to_string())?;
                    }
                }
            } else {
                filtered.init().await?;
            }
            Ok(filtered)
        } else {
            other_error!("DataSource is not initialized")
        }
    }
}
