use super::super::DataSource;
use crate::utils;
use anyhow::Result;
use std::collections::HashSet;

impl DataSource {
    pub async fn hash<S>(&mut self, key: S) -> Result<HashSet<String>>
    where
        S: AsRef<str> + Clone + ToString,
    {
        let mut dt = utils::DebugTimer::new();
        println!("Hash Start");

        let mut hash = HashSet::new();
        self.foreach(|di| {
            if let Some(value) = di.get(key.clone()) {
                hash.insert(value);
            }
            Ok(true)
        })
        .await?;

        println!("Hash End - {:.2}s", dt.lap().as_secs_f32());
        Ok(hash)
    }
}
