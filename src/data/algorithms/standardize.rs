use super::super::DataSource;
use crate::types::{GlobalRes, Number};

impl DataSource {
    pub async fn standardize(
        &mut self,
        to: Option<&str>,
        include: &Vec<&str>,
    ) -> GlobalRes<Self> {
        let mut standarized = self.child(to)?;
        if !standarized.exists() {
            let mut n: u32 = 0;
            let mut means: Vec<(&str, f64)> = include.clone().iter().map(|x| (*x, 0.0)).collect();
            let mut variances = means.clone();
            self.foreach(|di| {
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
            self.foreach(|di| {
                n += 1;
                for i in 0..include.len() {
                    let (header, mean) = &means[i];
                    let (_, variance) = &mut variances[i];
                    *variance += (di.get::<f64>(header).unwrap() - *mean).powi(2) / (n as f64);
                }
                Ok(())
            })
            .await?;
            for (_, value) in &mut variances {
                *value = value.sqrt()
            }
            standarized.init().await?;
        }
        Ok(standarized)
    }
}
