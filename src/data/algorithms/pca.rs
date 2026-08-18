use std::collections::HashMap;

use super::super::DataSource;
use crate::types::{GlobalRes, SymmetricKey};

impl DataSource {
    pub async fn pca(&mut self, to: Option<&str>, include: &Vec<&str>) -> GlobalRes<Self> {
        let mut pca = self.child(to)?;
        if !pca.exists() {
            let mut cov_matrix: HashMap<SymmetricKey<&str>, f64> = HashMap::new();
            let mut standardized = self.standardize(None, include).await?;
            let mut means: Vec<(&str, f64)> = include.clone().iter().map(|x| (*x, 0.0)).collect();
            let mut n: u32 = 0;
            standardized
                .foreach(|di| {
                    n += 1;
                    for (header, value) in &mut means {
                        *value += di.get::<f64>(header).unwrap()
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
                        let tail = &means[i + 1..];
                        let head_v = di.get::<f64>(head.0).unwrap();
                        for pair in tail {
                            let pair_v = di.get::<f64>(pair.0).unwrap();
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
            pca.init().await?;
        } else {
            pca.init().await?;
        }
        Ok(pca)
    }
}
