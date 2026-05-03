use crate::utils::unzip;
use chrono::Local;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::{env, fs};

pub trait DataSource<'a>: Sized {
    fn get_web_source(&self) -> String;

    fn get_path(&self) -> String;

    fn new() -> Self;

    fn get_ref(&self) -> Result<PathBuf, Box<dyn Error>> {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;
        Ok(PathBuf::from(&env_dsp).join(self.get_path()))
    }

    async fn init() -> Result<Self, Box<dyn Error>> {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;

        let inst = Self::new();
        let path = inst.get_path();
        let full_path = PathBuf::from(&env_dsp).join(&path);

        if !full_path.exists() {
            let zip_file = format!("{}", Local::now().format("%Y%m%d_%H%M%S.zip"));
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
                return Err(Box::new(std::io::Error::other(format!("{:?}", err))));
            }
        }

        Ok(inst)
    }
}
