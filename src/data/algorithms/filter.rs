use super::super::{DataItem, DataSource};
use crate::types::GlobalRes;

impl DataSource {
    pub async fn filter<F>(&mut self, to: Option<&str>, f: F) -> GlobalRes<Self>
    where
        F: Fn(DataItem) -> Option<DataItem>,
    {
        let mut filtered = self.child(to)?;
        if !filtered.exists() {
            filtered.init().await?;
            filtered.write(true)?;
            self.foreach(|di| {
                if let Some(output) = f(di) {
                    filtered.write_item(output)?;
                }
                Ok(())
            })
            .await?;
            filtered.write(false)?;
        } else {
            filtered.init().await?;
        }
        Ok(filtered)
    }
}
