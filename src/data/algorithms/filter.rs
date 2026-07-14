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
                if let Some(output) = f(di)
                    && let Some(output_h) = output.get_header()
                {
                    filtered.set_header(output_h)?;
                    filtered.write_line(output.to_string())?;
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
