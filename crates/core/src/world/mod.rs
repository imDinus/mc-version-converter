pub mod layout;

use std::fs;
use std::path::Path;

use crate::error::{Error, Result};

pub(crate) fn copy_dir_recursive(from: &Path, to: &Path) -> Result<u64> {
    let mut count = 0;
    fs::create_dir_all(to).map_err(|e| Error::io(to, e))?;
    for entry in fs::read_dir(from).map_err(|e| Error::io(from, e))? {
        let entry = entry.map_err(|e| Error::io(from, e))?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| Error::io(&src, e))?;
        if file_type.is_dir() {
            count += copy_dir_recursive(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).map_err(|e| Error::io(&src, e))?;
            count += 1;
        }
    }
    Ok(count)
}
