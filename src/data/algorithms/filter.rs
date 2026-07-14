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
            let mut w = filtered._get_writer()?;
            self.foreach(|di| {
                if let Some(output) = f(di)
                    && let Some(output_h) = output.get_header()
                {
                    filtered._set_header(&mut w, output_h)?;
                    filtered._write_line(&mut w, output.to_string())?;
                }
                Ok(())
            })
            .await?;
        } else {
            filtered.init().await?;
        }
        Ok(filtered)
    }
}
