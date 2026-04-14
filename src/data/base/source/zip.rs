use super::DataSource;

pub trait ZipDataSource: DataSource {
    fn zip_init() -> Self;
    fn init() -> Self {
        Self::zip_init()
    }
}