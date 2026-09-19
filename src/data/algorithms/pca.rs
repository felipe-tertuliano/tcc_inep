use super::super::DataSource;
use anyhow::Result;
use nalgebra::{DMatrix, SymmetricEigen};
use tokio_stream::StreamExt;

impl DataSource {
    /// PCA algorithm. Works properly only on **standardized data** provided in `include`
    pub async fn pca(&mut self, k: usize, include: &Vec<&str>) -> Result<Vec<String>> {
        return Ok(vec![]);
        self.read(true, None)?; //! REMOVE LATTER (for tests only)
        let mut means = self
            .get_header()?
            .iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned(), 0.0))
            .collect::<Vec<_>>();
        let mut n: u32 = 0;
        self.foreach(|di| {
            n += 1;
            for (header, _, value) in &mut means {
                *value += di.get::<f64>(header).unwrap_or(0.0)
            }
            Ok(())
        })
        .await?;
        for (_, _, value) in &mut means {
            *value /= n as f64
        }
        let cm = self
            .parallel_foreach(2_097_152, move |di| {
                let data = di.to_vec().unwrap();
                let mut res = vec![0.0; means.len().pow(2)];
                for i in 0..means.len() {
                    let head_m = &means[i];
                    let tail = &means[i..];
                    let head_v = data
                        .get(head_m.1)
                        .map(|(_, d)| d.parse::<f64>().unwrap_or(0.0))
                        .unwrap_or(0.0);
                    for j in 0..tail.len() {
                        let pair_m = &tail[j];
                        let pair_v = data
                            .get(pair_m.1)
                            .map(|(_, d)| d.parse::<f64>().unwrap_or(0.0))
                            .unwrap_or(0.0);
                        let value = ((head_v - head_m.2) * (pair_v - pair_m.2)) / ((n - 1) as f64);
                        res[(i + j) * means.len() + i] = value;
                        res[i * means.len() + i + j] = value;
                    }
                }
                res
            })?
            .fold(vec![0.0; include.len().pow(2)], |mut acc, x| {
                for i in 0..acc.len() {
                    acc[i] += x[i];
                }
                acc
            })
            .await;
        let eigenvalues =
            SymmetricEigen::new(DMatrix::from_vec(include.len(), include.len(), cm)).eigenvalues;
        let mut principal = include
            .iter()
            .enumerate()
            .map(|(i, h)| (*h, eigenvalues[i]))
            .collect::<Vec<(&str, f64)>>();
        principal.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        println!("{:#?}", principal[0..k].to_vec());
        Ok(principal[0..k].iter().map(|p| p.0.to_owned()).collect())
    }
}
