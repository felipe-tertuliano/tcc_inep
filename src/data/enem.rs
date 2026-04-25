use super::base::DataSource;

const SOURCE: &str = "https://download.inep.gov.br/microdados/microdados_enem_2024.zip";

struct EnemDataSource {}

impl DataSource for EnemDataSource {
    fn get_web_source(&self) -> String {
        return String::new();
    }

    fn get_path(&self) -> String {
        return String::new();
    }

    fn new() -> Self {
        return Self {};
    }
}
