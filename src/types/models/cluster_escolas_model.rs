use serde::Serialize;
use crate::types::models::EscolaModel;

#[derive(Serialize, Debug)]
pub struct ClusterEscolaModel {
    pub id: usize,
    pub avg_grade: u32,
    pub outliers: Vec<EscolaModel>,
}
