use super::super::DataSource;
use crate::{data::DataItem, types::UniRef, utils::DebugTimer};
use std::{fmt::Display, hash::Hash};
use anyhow::Result;

impl DataSource {
    pub async fn standardize<S>(
        &mut self,
        to: Option<&str>,
        include: &[S],
        id: S,
    ) -> Result<Self> 
    where 
        S: AsRef<str> + Copy + ToString + Display + Hash + Eq
    {
        let mut dt = DebugTimer::new();
        let mut standardized = self.child(to)?;
        if !standardized.exists() {
            let mut variances = include.iter().map(|x| (x.to_string(), 0.0)).collect::<Vec<_>>();
            let (avg, n) = self.avg::<_, f64>(include).await?;
            println!("Standardize: Mean Step - {:.2}s", dt.lap().as_secs_f32());
            self.foreach(|di| {
                for i in 0..include.len() {
                    let (header, mean) = &avg[i];
                    let (_, variance) = &mut variances[i];
                    *variance += (di.get::<_, f64>(header).unwrap_or(0.0) - *mean).powi(2);
                }
                Ok(true)
            })
            .await?;
            for (_, value) in &mut variances {
                *value = (*value / n).sqrt();
            }
            println!(
                "Standardize: Variance Step - {:.2}s",
                dt.lap().as_secs_f32()
            );
            standardized.init().await?;
            standardized.write(true)?;
            self.foreach(|di| {
                let mut new_di = DataItem::new(UniRef::Int, vec![]);
                for i in 0..include.len() {
                    let (header, variance) = &variances[i];
                    let (_, mean) = &avg[i];
                    new_di.set(id, di.get::<_, String>(id).unwrap_or("".to_owned()));
                    new_di.set(
                        header,
                        (di.get::<_, f64>(header).unwrap_or(0.0) - mean) / variance,
                    );
                }
                standardized.write_item(new_di)?;
                Ok(true)
            })
            .await?;
            standardized.write(false)?;
            println!(
                "Standardize: Standardize Step - {:.2}s",
                dt.lap().as_secs_f32()
            );
        } else {
            standardized.init().await?;
        }
        Ok(standardized)
    }
}
