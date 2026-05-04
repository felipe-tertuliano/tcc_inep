use crate::types::GlobalRes;
use crate::utils::unzip;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, Write};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::{env, fs};

pub trait DataSource<'a>: Sized {
    fn get_web_source(&self) -> String;

    fn get_source_path(&self) -> String;

    fn get_struct_path(&self) -> String;

    fn new() -> Self;

    fn get_ref(&self) -> GlobalRes<PathBuf> {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;
        Ok(PathBuf::from(&env_dsp).join(self.get_source_path()))
    }

    fn get_header(&self) -> GlobalRes<Vec<String>> {
        let mut reader = BufReader::new(File::open(
            self.get_ref()
                .expect("Error while fetching data source path reference"),
        )?);
        reader.seek(std::io::SeekFrom::Start(0))?;
        let mut header = String::new();
        reader.read_line(&mut header)?;
        Ok(header.split(';').map(|s| s.trim().to_string()).collect())
    }

    async fn init() -> GlobalRes<Self> {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;

        let inst = Self::new();
        let path = inst.get_source_path();
        let full_path = PathBuf::from(&env_dsp).join(&path);

        if !full_path.exists() {
            let zip_file = format!(
                "{}.zip",
                regex::Regex::new(r"[^a-z]")?
                    .replace_all(&inst.get_web_source().trim().to_lowercase(), "")
            );
            let zip_path = PathBuf::from(&env_dsp).join(&zip_file);
            let mut zip_file = File::create(&zip_path)?;

            let web_source = inst.get_web_source();
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
}
