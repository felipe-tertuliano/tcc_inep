use std::collections::HashSet;
use super::super::DataSource;
use anyhow::Result;

impl DataSource {
    pub async fn hash<S>(&mut self, key: S) -> Result<HashSet<String>>
    where
        S: AsRef<str> + Clone + ToString,
    {
        let mut hash = HashSet::new();
        self.foreach(|di| {
            if let Some(value) = di.get(key.clone()) {
                hash.insert(value);
            }
            Ok(true)
        })
        .await?;
        Ok(hash)
    }
}
