use super::super::DataSource;
use crate::types::GlobalRes;

impl DataSource {
    //TODO: remove underscore
    pub async fn _pca<FE>(
        &mut self,
        to: Option<&str>,
        _include: Option<&Vec<&str>>,
    ) -> GlobalRes<Self> {
        let reduced = self.child(to)?;
        if !reduced.exists() {
            self.read(true, None)?;
        }
        Ok(reduced)
    }
}
