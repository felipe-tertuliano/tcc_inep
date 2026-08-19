use super::super::DataSource;
use crate::{data::DataItem, types::{GlobalRes, UniRef}};

impl DataSource {
    pub async fn standardize(&mut self, to: Option<&str>, include: &Vec<&str>) -> GlobalRes<Self> {
        let mut standardized = self.child(to)?;
        if !standardized.exists() {
            let mut variances: Vec<(&str, f64)> = include.iter().map(|x| (*x, 0.0)).collect();
            let mut means = variances.clone();
            let mut n: u32 = 0;
            self.foreach(|di| {
                n += 1;
                for (header, value) in &mut means {
                    *value += di.get::<f64>(header).unwrap_or(0.0);
                }
                Ok(())
            })
            .await?;
            for (_, value) in &mut means {
                *value /= n as f64
            }
            self.foreach(|di| {
                for i in 0..include.len() {
                    let (header, mean) = &means[i];
                    let (_, variance) = &mut variances[i];
                    *variance += (di.get::<f64>(header).unwrap_or(0.0) - *mean).powi(2);
                }
                Ok(())
            })
            .await?;
            for (_, value) in &mut variances {
                *value = (*value / (n as f64)).sqrt();
            }
            standardized.init().await?;
            standardized.write(true)?;
            // TODO: Incluir todos os campos na normalização
            self.foreach(|mut di| {
                let mut new_di = DataItem::new(UniRef::Int, vec![]); 
                for i in 0..include.len() {
                    let (header, variance) = &variances[i];
                    let (_, mean) = &means[i];
                    new_di.set(
                        header,
                        (di.get::<f64>(header).unwrap_or(0.0) - mean) / variance,
                    );
                }
                standardized.write_item(new_di)?;
                Ok(())
            })
            .await?;
            standardized.write(false)?;
        } else {
            standardized.init().await?;
        }
        Ok(standardized)
    }
}
