use super::DataItem;
use crate::types::{Source, UniRef};
use crate::utils::{get_csv_cols, unzip};
use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio_stream::{self, Stream, wrappers::ReceiverStream};
use uuid::Uuid;

pub type DataHeader = HashMap<String, usize>;

const BUFFER_SIZE: usize = 1024;

pub struct DataSource {
    _writer: Option<BufWriter<File>>,
    _reader: Option<BufReader<File>>,
    _header: Option<DataHeader>,
    _is_initialized: bool,
    _os_path: OsString,
    _source: Source,

    env_dsp: String,
    env_bs: usize,
    env_cs: usize,
}

impl DataSource {
    pub fn new(source: Source) -> Result<Self> {
        let env_bs = env::var("BUFFER_SIZE")?.parse()?;
        let env_cs = env::var("CHUNK_SIZE")?.parse()?;
        let env_dsp = env::var("DATA_SOURCE_PATH")?;
        let os_path = PathBuf::from(&env_dsp)
            .join(match &source {
                Source::Local(p) => p,
                Source::Remote(p, _) => p,
            })
            .as_os_str()
            .to_owned();
        Ok(Self {
            env_cs,
            env_bs,
            env_dsp,
            _header: None,
            _reader: None,
            _writer: None,
            _source: source,
            _os_path: os_path,
            _is_initialized: false,
        })
    }

    pub fn get_env_dsp(&self) -> &String {
        &self.env_dsp
    }

    pub fn get_env_bs(&self) -> &usize {
        &self.env_bs
    }

    pub fn get_env_cs(&self) -> &usize {
        &self.env_cs
    }

    pub fn metadata(&self) -> Result<fs::Metadata> {
        Ok(fs::metadata(&self._os_path)?)
    }

    pub fn child(&self, name: Option<&str>) -> Result<Self> {
        if self._is_initialized {
            Self::new(Source::Local(format!(
                "{}.csv",
                name.map(|s| s.to_string())
                    .unwrap_or(Uuid::new_v4().to_string())
            )))
        } else {
            msg_error!("DataSource is not initialized")
        }
    }

    pub fn delete(self) -> Result<()> {
        Ok(fs::remove_file(&self._os_path)?)
    }

    /* #region Helpers */
    pub fn exists(&self) -> bool {
        Path::new(&self._os_path).exists()
    }
    /* #endregion */

    /* #region Initializers */
    async fn _local_init(&mut self, _path: &str) -> Result<()> {
        File::create(&self._os_path)?;
        Ok(())
    }

    async fn _remote_init(&mut self, path: &str, url: &str) -> Result<()> {
        let zip_file = format!(
            "{}.zip",
            regex::Regex::new(r"[^a-z]")?.replace_all(&url.to_lowercase(), "")
        );
        let zip_path = PathBuf::from(&self.env_dsp).join(&zip_file);
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
            let parent_path = PathBuf::from(&self.env_dsp).join(parent);
            unzip(&zip_path, &parent_path)?;
        }
        fs::remove_file(&zip_path)?;
        if let Err(err) = res {
            return msg_error!(err);
        }
        Ok(())
    }

    pub async fn init(&mut self) -> Result<&mut Self> {
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
    fn _move_reader(
        reader: &mut BufReader<File>,
        seek: Option<SeekFrom>,
        line: Option<usize>,
    ) -> Result<usize> {
        let mut b = 1;
        if let Some(s) = seek {
            reader.seek(s)?;
        }
        if let Some(l) = line {
            let mut i = 0;
            while b > 0 && i < l {
                b = reader.skip_until(b'\n')?;
                i += 1;
            }
        }
        Ok(b)
    }

    pub fn move_reader(
        &mut self,
        seek: Option<SeekFrom>,
        line: Option<usize>,
    ) -> Result<usize> {
        if let Some(reader) = self._reader.as_mut() {
            Self::_move_reader(reader, seek, line)
        } else {
            msg_error!("Read mode is not activated")
        }
    }

    fn _new_reader(
        &self,
        seek: Option<SeekFrom>,
        line: Option<usize>,
    ) -> Result<(BufReader<File>, usize)> {
        let mut reader = BufReader::new(OpenOptions::new().read(true).open(&self._os_path)?);
        let b = Self::_move_reader(&mut reader, seek, line).unwrap_or(1);
        Ok((reader, b))
    }

    pub fn read(&mut self, on: bool, line: Option<usize>) -> Result<()> {
        if on {
            if self._reader.is_none() {
                self._reader = Some(self._new_reader(None, line)?.0);
            }
        } else {
            self._reader = None;
        }
        Ok(())
    }

    fn _read_line(
        reader: &mut BufReader<File>,
        buf: &mut Vec<u8>,
        seek: Option<SeekFrom>,
        rewind: bool,
    ) -> Result<(Option<Vec<String>>, usize)> {
        let mut res = Ok((None, 0));
        let old = reader.stream_position()?;
        buf.clear();
        if let Some(pos) = seek {
            reader.seek(pos)?;
        }
        let b = reader.read_until(b'\n', buf)?;
        if b > 0 {
            let line = get_csv_cols(String::from_utf8_lossy(buf).trim(), ';')?;
            res = Ok((Some(line), b));
        }
        if rewind {
            reader.seek(SeekFrom::Start(old))?;
        }
        res
    }

    pub fn read_item(&mut self) -> Result<Option<DataItem>> {
        if self._reader.is_some() {
            let mut buf = vec![0; BUFFER_SIZE];
            let header = self.get_header()?.clone();
            let reader = self._reader.as_mut().expect("Error while obtaining read permission");
            Ok(if let Some(value) = Self::_read_line(reader, &mut buf, None, false)?.0 {
                Some(DataItem::new(UniRef::Loc(header), value))
            } else {
                None
            })
        } else {
            msg_error!("Read mode is not activated")
        }
    }

    pub fn get_header(&mut self) -> Result<&DataHeader> {
        if self._reader.is_none() {
            msg_error!("Read mode is not activated")
        } else {
            if self._header.is_none() && let Some(reader) = self._reader.as_mut() {
                let mut buf = vec![0; BUFFER_SIZE];
                self._header = Some(
                    Self::_read_line(reader, &mut buf, Some(SeekFrom::Start(0)), true)?.0
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
                msg_error!("Unable to fetch DataSource's header")
            }
        }
    }
    /* #endregion */

    /* #region Writers */
    pub fn write(&mut self, on: bool) -> Result<()> {
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

    pub fn write_item(&mut self, item: DataItem) -> Result<()> {
        if self._header.is_none() {
            self.set_header(item.get_header().unwrap())?;
        }
        self.write_line(item.to_string())
    }

    pub fn write_line(&mut self, line: String) -> Result<()> {
        if let Some(writer) = self._writer.as_mut() {
            writeln!(writer, "{}", line)?;
            Ok(())
        } else {
            msg_error!("Write mode is not activated")
        }
    }

    pub fn set_header(&mut self, header: &DataHeader) -> Result<&DataHeader> {
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
                msg_error!("Unable to fetch DataSource's header")
            }
        } else {
            msg_error!("Write mode is not activated")
        }
    }
    /* #endregion */

    pub async fn foreach<F>(&mut self, mut f: F) -> Result<()>
    where
        F: FnMut(DataItem) -> Result<()>,
    {
        if self._is_initialized {
            let mut buf = vec![0; BUFFER_SIZE];
            self.read(true, Some(1))?;
            let header = self.get_header()?.clone();
            let reader = self._reader.as_mut().expect("Error while obtaining read permission");
            while let Some(value) = Self::_read_line(reader, &mut buf, None, false)?.0 {
                f(DataItem::new(UniRef::Ref(&header), value))?;
            }
            self.read(false, None)?;
            Ok(())
        } else {
            msg_error!("DataSource is not initialized")
        }
    }

    // TODO: Validate chunk division (i may jump lines)
    pub fn parallel_foreach<R, F>(
        &mut self,
        chunk_size: u64,
        func: F,
    ) -> Result<impl Stream<Item = R>>
    where
        R: Send + 'static,
        F: FnMut(DataItem) -> R + Clone + Send + 'static,
    {
        if BUFFER_SIZE > (chunk_size as usize) {
            msg_error!(format!("`chunk_size` min. value is {}", BUFFER_SIZE))
        } else if self._is_initialized {
            let mut eof = false;
            let mut pos = 0;
            let (tx, rx) = mpsc::channel((chunk_size as usize) / BUFFER_SIZE);
            self.read(true, None)?;
            while !eof {
                let (mut t_reader, b) = self._new_reader(Some(SeekFrom::Start(pos)), Some(1))?;
                if b == 0 {
                    eof = true;
                } else {
                    pos += chunk_size;

                    let t_header = self.get_header()?.clone();
                    let t_tx = tx.clone();
                    let mut t_func = func.clone();
                    let mut t_buf = vec![0; BUFFER_SIZE];
                    let mut t_eoc = false;
                    let mut t_pos = 0;
                    tokio::spawn(async move {
                        while !t_eoc {
                            match Self::_read_line(&mut t_reader, &mut t_buf, None, false) {
                                Ok((Some(t_value), t_b)) => {
                                    t_pos += t_b as u64;
                                    if t_pos >= chunk_size || t_b == 0 {
                                        t_eoc = true;
                                    } else {
                                        let _ = t_tx
                                            .send(t_func(DataItem::new(
                                                UniRef::Ref(&t_header),
                                                t_value,
                                            )))
                                            .await;
                                    }
                                }
                                Ok((None, _)) => {
                                    t_eoc = true;
                                }
                                Err(_) => {
                                    t_eoc = true;
                                }
                            }
                        }
                    });
                }
            }
            self.read(false, None)?;
            drop(tx);
            Ok(ReceiverStream::new(rx))
        } else {
            msg_error!("DataSource is not initialized")
        }
    }
}
