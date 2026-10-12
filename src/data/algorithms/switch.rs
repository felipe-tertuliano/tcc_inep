use super::super::{DataItem, DataSource};
use anyhow::Result;

impl DataSource {
    pub async fn switch<F>(&mut self, to: Option<String>, f: &[F]) -> Result<Vec<Self>>
    where
        F: Fn(&DataItem) -> bool,
    {
        let f = f.iter().enumerate().collect::<Vec<_>>();
        let mut switch = f.iter()
            .map(|(i, _)| {
                self.child(to.clone().map(|v| format!("{}_{}", v, i)))
                    .unwrap()
            })
            .collect::<Vec<_>>();

        for ds in switch.iter_mut() {
            ds.init().await?;
            ds.write(true)?;
        }
        self.foreach(|di| {
            if let Some((i, _)) = f.iter().find(|(_, lf)| lf(&di)) {
                switch[*i].write_item(di)?;
            }
            Ok(true)
        })
        .await?;
        for ds in switch.iter_mut() {
            ds.write(false)?;
        }

        Ok(switch)
    }
}
