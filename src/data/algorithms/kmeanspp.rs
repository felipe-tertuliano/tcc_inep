use super::super::DataSource;
use crate::utils::DebugTimer;
use anyhow::Result;

impl DataSource {
    // TODO
    pub async fn _kmeanspp(&mut self, to: Option<&str>, include: &Vec<&str>) -> Result<Self> {
        let mut dt = DebugTimer::new();
        let mut kmeanspp = self.child(to)?;
        if !kmeanspp.exists() {
            kmeanspp.init().await?;
            println!("K-means++: Build Step - {:.2}s", dt.lap().as_secs_f32());
        }
        Ok(kmeanspp)
    }
}
