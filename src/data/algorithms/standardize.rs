use super::super::DataSource;
use crate::types::GlobalRes;

impl DataSource {
    pub async fn standardize(&mut self, to: Option<&str>, include: &Vec<&str>) -> GlobalRes<Self> {
        let mut standardized = self.child(to)?;
        if !standardized.exists() {
            let mut variances: Vec<(&str, f64)> =
                include.clone().iter().map(|x| (*x, 0.0)).collect();
            let mut means = variances.clone();
            let mut n: u32 = 0;
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
                *value = value.sqrt();
            }
            standardized.init().await?;
            standardized.write(true)?;
            self.foreach(|mut di| {
                if let Some(all_headers) = di.get_header() {
                    standardized.set_header(all_headers)?;
                    for i in 0..include.len() {
                        let (header, variance) = &variances[i];
                        let (_, mean) = &means[i];
                        di.set(header, (di.get::<f64>(header).unwrap() - mean) / variance);
                    }
                    standardized.write_line(di.to_string())?;
                }
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
