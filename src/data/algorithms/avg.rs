use std::{ops::AddAssign, ops::DivAssign, str::FromStr};
use super::super::DataSource;
use anyhow::Result;

impl DataSource {
    pub async fn avg<S, T>(&mut self, include: &[S]) -> Result<(Vec<(String, T)>, T)>
    where
        T: Default + AddAssign + DivAssign + FromStr + From<u8> + Copy,
        S: AsRef<str> + ToString
    {
        let mut avg = include.iter().map(|x| (x.to_string(), T::default())).collect::<Vec<_>>();
        let mut n = T::default();
        let one = T::from(1u8); 
        self.foreach(|di| {
            n += one;
            for (header, value) in &mut avg {
                *value += di.get::<_, T>(header).unwrap_or(T::default());
            }
            Ok(true)
        })
        .await?;
        for (_, value) in &mut avg {
            *value /= n
        }
        Ok((avg, n))
    }
}
