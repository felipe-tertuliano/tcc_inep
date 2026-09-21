use super::super::DataSource;
use crate::{data::DataItem, types::Source, utils};
use anyhow::Result;
use tokio_stream::StreamExt;

impl DataSource {
    fn _choose_rand(
        &mut self,
        limit: &mut Option<u32>,
    ) -> Result<DataItem> {
        self.read(
            true,
            Some(utils::rand(if limit.is_none() {
                let value = (self.metadata()?.len() / (*self.get_env_bs() as u64)) as u32;
                *limit = Some(value);
                value
            } else {
                limit.unwrap()
            }) as usize),
        )?;
        Ok(self.read_item()?.unwrap())
    }

    async fn _get_centroids(&mut self, k: usize, include: &Vec<&str>) -> Result<Vec<DataItem>> {
        if k == 0 {
            msg_error!("k must be bigger than zero!")
        } else {
            let mut res = Vec::with_capacity(k);
            let (mut ds_r, mut ds_w) = (self.clone(), self.clone());
            let mut limit = None;

            res[0] = ds_r._choose_rand(&mut limit)?;

            for i in 1..k {
                let centroid = &res[i - 1];
                let pruning = 1.0 - (1.0 / (k - i) as f32);
                ds_w.parallel_foreach(move |_di| 0)?
                .fold(0, |mut acc, x| {
                    acc += x;
                    acc
                })
                .await;
                let mut ds = self.child(None)?;
                // TODO
            }
            Ok(vec![])
        }
    }

    // TODO
    pub async fn kmeanspp(
        &mut self,
        to: Option<&str>,
        k: usize,
        include: &Vec<&str>,
    ) -> Result<Self> {
        let mut dt = utils::DebugTimer::new();
        let mut kmeanspp = self.child(to)?;
        if !kmeanspp.exists() {
            if k == 0 {
                return msg_error!("k must be bigger than zero!");
            }
            let mut limit = None;
            let centroid = self._choose_rand(&mut limit)?;

            let mut i = 0;
            let pruning = 1.0 - (1.0 / (k - i) as f32);

            self.read(false, None)?;
            // kmeanspp.init().await?;
            println!("K-means++: Build Step - {:.2}s", dt.lap().as_secs_f32());
        }
        Ok(kmeanspp)
    }
}
