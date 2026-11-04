use super::super::{DataItem, DataSource};
use crate::utils;
use anyhow::Result;

impl DataSource {
    pub async fn filter<F>(&mut self, to: Option<String>, f: F) -> Result<Self>
    where
        F: Fn(DataItem) -> Option<DataItem>,
    {
        let mut dt = utils::DebugTimer::new();
        println!("Filter Start");

        let mut filtered = self.child(to)?;
        if !filtered.exists() {
            filtered.init().await?;
            filtered.write(true)?;
            self.foreach(|di| {
                if let Some(output) = f(di) {
                    filtered.write_item(output)?;
                }
                Ok(true)
            })
            .await?;
            filtered.write(false)?;
        } else {
            filtered.init().await?;
        }

        println!("Filter End - {:.2}s", dt.lap().as_secs_f32());
        Ok(filtered)
    }
}
