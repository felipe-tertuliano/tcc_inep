use anyhow::Result;
use std::{fs, path::Path};

pub fn dir_ls<P>(dir: P, ext: Option<&str>) -> Result<Vec<(String, String)>>
where
    P: AsRef<Path>,
{
    let mut res = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let buf = entry.path();

        if buf.is_file()
            && let Some(name) = buf.file_name()
        {
            let matches_ext = match ext {
                Some(e) => buf.extension().and_then(|x| x.to_str()) == Some(e),
                None => true,
            };

            if matches_ext {
                let file_name = name.to_string_lossy().into_owned();
                let path_str = buf.to_string_lossy().into_owned();
                res.push((file_name, path_str));
            }
        }
    }

    Ok(res)
}
