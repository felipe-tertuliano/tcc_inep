use super::super::DataSource;
use crate::{data::DataItem, utils};
use anyhow::Result;
use tokio_stream::StreamExt;

impl DataSource {
    fn _sqr_dist(include: &Vec<String>, c: &DataItem, p: &DataItem) -> f64 {
        include.iter().fold(0.0, |mut acc, field| {
            acc += (p.get::<f64>(field).unwrap() - c.get::<f64>(field).unwrap()).powi(2);
            acc
        })
    }

    fn _choose_rand<'a>(&mut self, limit: &mut Option<u32>) -> Result<DataItem<'a>> {
        let l_value = if limit.is_none() {
            let value = (self.metadata()?.len() / (*self.get_env_cs() as u64)) as u32;
            *limit = Some(value);
            value
        } else {
            limit.unwrap()
        };
        self.read(
            true,
            Some(utils::rand(l_value) as usize),
        )?;
        Ok(self.read_item()?.unwrap())
    }

    async fn _get_centroids(&mut self, k: usize, include: &Vec<&str>) -> Result<Vec<DataItem>> {
        if k == 0 {
            msg_error!("k must be bigger than zero!")
        } else {
            let include = include.iter().map(|f| f.to_string()).collect::<Vec<_>>();
            let mut res = Vec::with_capacity(k);
            let mut ds = self.clone();
            let mut limit = None;

            res.push(ds._choose_rand(&mut limit)?);

            let k = 2; // !REMOVE
            for i in 1..k {
                let centroid = res[i - 1].clone();
                // let pruning = 1.0 - (1.0 / (k - i) as f32);

                let include_c = include.clone();
                let centroid_c = centroid.clone();
                let sum = ds
                    .parallel_foreach(move |p| Self::_sqr_dist(&include_c, &centroid_c, &p))?
                    .fold(0.0, |mut acc, x| {
                        acc += x;
                        acc
                    })
                    .await;
                println!("SUM: {:.2}", sum);
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
            let _ = self._get_centroids(k, include).await?;
            println!("K-means++: Build Step - {:.2}s", dt.lap().as_secs_f32());
        }
        Ok(kmeanspp)
    }
}
