use crate::utils::unzip;
use chrono::Local;
use reqwest;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs};

pub trait DataSource: Sized {
    fn get_web_source(&self) -> String;

    fn get_path(&self) -> String;

    fn new() -> Self;

    fn get_ref(&self, path: String) -> PathBuf {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;
        return PathBuf::from(env_dsp).join(path);
    }

    fn get_ref(&self) -> PathBuf {
        return self.get_ref(self.get_path());
    }

    async fn init() -> Self {
        let env_dsp = env::var("DATA_SOURCE_PATH")?;

        let inst = Self::new();
        let path = inst.get_path();
        let full_path = PathBuf::from(env_dsp).join(path);

        if !full_path.exists() {
            let zip_path = PathBuf::from(env_dsp).join(Local::now().format("%Y%m%d_%H%M%S.zip"));
            let mut zip_file = File::create(zip_path)?;

            let content = reqwest::get(inst.get_web_source()).await?.bytes().await?;
            zip_file.write_all(&content)?;

            let parent = path.split('/').next().unwrap_or(path.as_str()).to_string();
            let parent_path = PathBuf::from(env_dsp).join(parent);
            unzip(zip_path, parent_path)?;
            fs::remove_file(zip_path)?;
        }

        return inst;
    }
}
