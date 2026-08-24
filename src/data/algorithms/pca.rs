use super::super::DataSource;
use crate::{
    data::DataItem,
    types::{GlobalRes, SymmetricKey, UniRef},
    utils::DebugTimer,
};
use nalgebra::{DMatrix, SymmetricEigen};
use std::collections::HashMap;

impl DataSource {
    pub async fn pca(
        &mut self,
        to: Option<&str>,
        k: usize,
        include: &Vec<&str>,
    ) -> GlobalRes<Self> {
        let mut dt = DebugTimer::new();
        let mut pca = self.child(to)?;
        if !pca.exists() {
            let mut standardized = self.standardize(None, include).await?;
            println!("PCA: Standardize Step - {:.2}s", dt.lap().as_secs_f32());
            standardized.read(true, None)?;
            let mut means = standardized
                .get_header()?
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned(), 0.0))
                .collect::<Vec<_>>();
            let mut n: u32 = 0;
            standardized
                .foreach(|di| {
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
            println!("PCA: Mean Step - {:.2}s", dt.lap().as_secs_f32());

            let mut cm = vec![0.0; include.len().pow(2)];
            let mut threads = vec![];
            standardized
                .foreach(|di| {
                    let data = di.to_vec().unwrap();
                    let m_clone = means.clone();
                    threads.push(tokio::spawn(async move {
                        let mut res = vec![0.0; m_clone.len().pow(2)];
                        for i in 0..m_clone.len() {
                            let head_m = &m_clone[i];
                            let tail = &m_clone[i..];
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
                                let value =
                                    ((head_v - head_m.2) * (pair_v - pair_m.2)) / ((n - 1) as f64);
                                res[(i + j) * m_clone.len() + i] = value;
                                res[i * m_clone.len() + i + j] = value;
                            }
                        }
                        res
                    }));
                    println!(
                        "PCA: Cov. Matrix Step - One Element - {:.3}s",
                        dt.lap().as_secs_f32()
                    );
                    Ok(())
                })
                .await?;
            standardized.delete()?;
            for thread in threads {
                let res = thread.await?;
                for i in 0..cm.len() {
                    cm[i] += res[i];
                }
            }
            println!("PCA: Cov. Matrix Step - {:.2}s", dt.lap().as_secs_f32());

            let eigenvalues =
                SymmetricEigen::new(DMatrix::from_vec(include.len(), include.len(), cm))
                    .eigenvalues;
            println!("PCA: Eigenvalues Step - {:.2}s", dt.lap().as_secs_f32());

            let mut remove = include
                .iter()
                .enumerate()
                .map(|(i, h)| (*h, eigenvalues[i]))
                .collect::<Vec<(&str, f64)>>();
            remove.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            remove = remove[k..remove.len()].to_vec();

            pca.read(true, None)?;
            let mut new_headers = self.get_header()?.clone();
            pca.read(false, None)?;
            for (r, _) in remove {
                new_headers.remove(r);
            }

            pca.init().await?;
            pca.write(true)?;
            self.foreach(|di| {
                let mut new_id = DataItem::new(UniRef::Int, vec![]);
                for h in new_headers.keys() {
                    new_id.set(h, di.get::<String>(h).unwrap_or("".to_string()));
                }
                pca.write_item(new_id)?;
                Ok(())
            })
            .await?;
            pca.write(false)?;
            println!("PCA: Build Step - {:.2}s", dt.lap().as_secs_f32());
        } else {
            pca.init().await?;
        }
        Ok(pca)
    }
}
