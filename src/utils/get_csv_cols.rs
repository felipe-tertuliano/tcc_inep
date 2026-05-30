use crate::types::GlobalRes;

pub fn get_csv_cols(line: &str, pat: char) -> GlobalRes<Vec<String>> {
    Ok(line.split(pat).map(|s| s.trim().to_string()).collect())
}
