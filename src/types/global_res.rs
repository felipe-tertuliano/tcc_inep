use std::error::Error;
pub type GlobalRes<T> = Result<T, Box<dyn Error>>;
