#[macro_export]
macro_rules! msg_error {
    ($err:expr) => {
        anyhow::Result::Err(anyhow::Error::msg(format!(
            "{}",
            $err
        )))
    };
}


