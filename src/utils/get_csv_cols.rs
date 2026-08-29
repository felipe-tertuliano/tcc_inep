use anyhow::Result;

pub fn get_csv_cols(line: &str, pat: char) -> Result<Vec<String>> {
    Ok(line.split(pat).map(|s| s.trim().to_string()).collect())
}
