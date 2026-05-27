use crate::types::GlobalRes;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;
use zip::ZipArchive;

pub fn unzip<P: AsRef<Path>>(zip_path: P, extract_to: P) -> GlobalRes<()> {
    let file = File::open(zip_path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let out_path = extract_to.as_ref().join(file.mangled_name());

        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent)?;
            }

            let mut out_file = File::create(&out_path)?;
            io::copy(&mut file, &mut out_file)?;
        }
    }
    Ok(())
}
