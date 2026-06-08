pub enum Source {
    Local(String),
    Remote(String, String),
}

impl Clone for Source {
    fn clone(&self) -> Self {
        match self {
            Source::Local(path) => Source::Local(path.clone()),
            Source::Remote(path, url) => Source::Remote(path.clone(), url.clone()),
        }
    }
}
