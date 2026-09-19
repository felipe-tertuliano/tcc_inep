use super::super::DataSource;
use crate::utils;
use anyhow::Result;

impl DataSource {
    // TODO
    pub async fn kmeanspp(&mut self, to: Option<&str>, k: usize, include: &Vec<&str>) -> Result<Self> {
        let mut dt = utils::DebugTimer::new();
        let mut kmeanspp = self.child(to)?;
        if !kmeanspp.exists() {
            let f_size = self.metadata()?.len();
            println!("f_size: {}", f_size);
            let rand_limit = (f_size / (*self.get_env_bs() as u64)) as u32;
            println!("rand_limit: {}", rand_limit);
            let rand_pos = utils::rand(rand_limit);
            println!("rand_pos: {}", rand_pos);
            self.read(true, Some(rand_pos as usize))?;
            let centroid = self.read_item()?.expect("Error while obtaining K-Means++ centroid");
            println!("{:?}", centroid.to_vec());
            self.read(false, None)?;
            // kmeanspp.init().await?;
            println!("K-means++: Build Step - {:.2}s", dt.lap().as_secs_f32());
        }
        Ok(kmeanspp)
    }
}
