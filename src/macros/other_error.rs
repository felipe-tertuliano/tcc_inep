#[macro_export]
macro_rules! other_error {
    ($err:expr) => {
        core::result::Result::Err(Box::new(std::io::Error::other(format!(
            "{:?}",
            $err
        ))))
    };
}
