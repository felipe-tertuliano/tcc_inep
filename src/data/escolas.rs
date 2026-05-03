use super::base::DataSource;
use std::marker::PhantomData;

const WEB_SOURCE: &str = "https://download.inep.gov.br/dados_abertos/microdados_censo_escolar_2024.zip";
const PATH: &str = "microdados_censo_escolar_2024/dados/microdados_ed_basica_2024.csv";

pub struct EscolasDataSource<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> DataSource<'a> for EscolasDataSource<'a> {
    fn get_web_source(&self) -> String {
        WEB_SOURCE.to_owned()
    }

    fn get_path(&self) -> String {
        PATH.to_owned()
    }

    fn new() -> Self {
        Self { _marker: PhantomData }
    }
}
