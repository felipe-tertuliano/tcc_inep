use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct EscolaModel {
    pub id: String,
    pub avg_grade: u32,
}
