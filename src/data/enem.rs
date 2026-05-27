use super::base::DataSource;
use std::marker::PhantomData;

const WEB_SOURCE: &str = "https://download.inep.gov.br/microdados/microdados_enem_2024.zip";
const SOURCE_PATH: &str = "microdados_enem_2024/DADOS/RESULTADOS_2024.csv";
const STRUCT_PATH: &str = "enem.json";

pub struct EnemDataSource<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> DataSource<'a> for EnemDataSource<'a> {
    fn _web_source(&self) -> String {
        WEB_SOURCE.to_owned()
    }

    fn _source_path(&self) -> String {
        SOURCE_PATH.to_owned()
    }

    fn _struct_path(&self) -> String {
        STRUCT_PATH.to_owned()
    }

    fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
