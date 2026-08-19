use super::super::DataSource;
use crate::{data::DataItem, types::{GlobalRes, SymmetricKey, UniRef}};
use nalgebra::{DMatrix, SymmetricEigen};
use std::collections::HashMap;

impl DataSource {
    pub async fn pca(
        &mut self,
        to: Option<&str>,
        k: usize,
        include: &Vec<&str>,
    ) -> GlobalRes<Self> {
        let mut pca = self.child(to)?;
        if !pca.exists() {
            let mut standardized = self.standardize(None, include).await?;

            let mut means: Vec<(&str, f64)> = include.clone().iter().map(|x| (*x, 0.0)).collect();
            let mut cov_matrix: HashMap<SymmetricKey<&str>, f64> = HashMap::new();
            let mut n: u32 = 0;
            standardized
                .foreach(|di| {
                    n += 1;
                    for (header, value) in &mut means {
                        *value += di.get::<f64>(header).unwrap_or(0.0)
                    }
                    Ok(())
                })
                .await?;
            for (_, value) in &mut means {
                *value /= n as f64
            }
            standardized
                .foreach(|di| {
                    for i in 0..means.len() {
                        let head = means[i];
                        let tail = &means[i..];
                        let head_v = di.get::<f64>(head.0).unwrap_or(0.0);
                        for pair in tail {
                            let pair_v = di.get::<f64>(pair.0).unwrap_or(0.0);
                            let key = SymmetricKey(head.0, pair.0);
                            let value = cov_matrix.get(&key).unwrap_or(&0.0);
                            cov_matrix.insert(
                                key,
                                value
                                    + (((head_v - head.1) * (pair_v - pair.1)) / ((n - 1) as f64)),
                            );
                        }
                    }
                    Ok(())
                })
                .await?;
            standardized.delete()?;

            let mut raw_cm = Vec::with_capacity(include.len().pow(2));
            for x in include {
                for y in include {
                    raw_cm.push(*cov_matrix.get(&SymmetricKey(x, y)).unwrap());
                }
            }
            let eigenvalues =
                SymmetricEigen::new(DMatrix::from_vec(include.len(), include.len(), raw_cm))
                    .eigenvalues;

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
            }).await?;
            pca.write(false)?;
        } else {
            pca.init().await?;
        }
        Ok(pca)
    }
}
