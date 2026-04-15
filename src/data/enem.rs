use super::base::DataSource;

const SOURCE: &str = "https://download.inep.gov.br/microdados/microdados_enem_2024.zip";

struct EnemDataSource {}

impl EnemDataSource for DataSource {
    fn get_web_source(&self) -> String;

    fn get_path(&self) -> String;

    fn new() -> Self {
        return Self {};
    }
}
