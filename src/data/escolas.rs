use super::base::DataSource;
use std::marker::PhantomData;

const WEB_SOURCE: &str =
    "https://download.inep.gov.br/dados_abertos/microdados_censo_escolar_2024.zip";
const SOURCE_PATH: &str = "microdados_censo_escolar_2024/microdados_censo_escolar_2024/dados/microdados_ed_basica_2024.csv";
const STRUCT_PATH: &str = "escolas.json";

pub struct EscolasDataSource<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> DataSource<'a> for EscolasDataSource<'a> {
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
