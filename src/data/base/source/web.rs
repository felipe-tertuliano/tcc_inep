use super::DataSource;

pub trait WebDataSource: DataSource {
    fn web_init() -> Self;
    fn init() -> Self {
        Self::web_init()
    }
}
