use super::base::DataSource;
use std::marker::PhantomData;

const WEB_SOURCE: &str = "https://download.inep.gov.br/microdados/microdados_enem_2024.zip";
const PATH: &str = "microdados_enem_2024/DADOS/RESULTADOS_2024.csv";

pub struct EnemDataSource<'a> {
    _marker: PhantomData<&'a ()>,
}

impl<'a> DataSource<'a> for EnemDataSource<'a> {
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
