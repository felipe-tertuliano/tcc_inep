use super::super::DataSource;
use crate::types::GlobalRes;

impl DataSource {
    //TODO: remove underscore
    pub async fn _pca<FE>(&mut self, to: Option<&str>, exclude: Option<FE>) -> GlobalRes<Self>
    where
        FE: Fn(&str) -> bool,
    {
        let mut reduced = self.child(to)?;
        if !reduced.exists() {
            self.read(true, None)?;
            //TODO: remove underscore
            let _headers = if let Some(exclude_fn) = exclude {
                self.get_header()?.iter().fold(vec![], |mut acc, (h, _)| {
                    if !exclude_fn(h) {
                        acc.push(h);
                    }
                    acc
                })
            } else {
                self.get_header()?.keys().collect()
            };
            self.read(false, None)?;
            reduced.init().await?;
            reduced.write(true)?;
            self.foreach(|_di| {
                //TODO: implement PCA
                Ok(())
            })
            .await?;
            reduced.write(false)?;
        } else {
            reduced.init().await?;
        }
        Ok(reduced)
    }
}
