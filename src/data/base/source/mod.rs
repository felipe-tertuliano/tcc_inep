pub trait DataSource: Sized {
    fn init() -> Self;
}

mod web;
mod zip;
pub use web::WebDataSource;
pub use zip::ZipDataSource;