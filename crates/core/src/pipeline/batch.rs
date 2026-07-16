use std::fs;
use std::path::{Path, PathBuf};

use super::{convert, ConvertOptions};
use crate::error::{Error, Result};
use crate::logging::log;
use crate::report::Report;

pub struct BatchOutcome {
    pub world_name: String,
    pub output: PathBuf,
    pub result: Result<Report>,
}

pub fn discover_worlds(input_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut worlds = Vec::new();
    for entry in fs::read_dir(input_dir).map_err(|e| Error::io(input_dir, e))? {
        let entry = entry.map_err(|e| Error::io(input_dir, e))?;
        let path = entry.path();
        if path.is_dir() && path.join("level.dat").is_file() {
            worlds.push(path);
        }
    }
    worlds.sort();
    Ok(worlds)
}

pub fn convert_batch(
    input_dir: &Path,
    output_dir: &Path,
    target: &str,
    block_table: Option<&Path>,
) -> Result<Vec<BatchOutcome>> {
    if input_dir.join("level.dat").is_file() {
        return Err(Error::InvalidWorld(format!(
            "level.dat found directly inside '{}'. Put each world in as its own folder \
             (e.g. {}\\my_world\\level.dat).",
            input_dir.display(),
            input_dir.display()
        )));
    }

    let worlds = discover_worlds(input_dir)?;
    log(format!(
        "Found {} world(s) in {}",
        worlds.len(),
        input_dir.display()
    ));
    let mut outcomes = Vec::new();
    for world in worlds {
        let world_name = world
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "world".to_string());
        let output = output_dir.join(format!("{world_name} ({target})"));
        let result = convert(&ConvertOptions {
            input: world.clone(),
            output: output.clone(),
            target: target.to_string(),
            block_table: block_table.map(Path::to_path_buf),
        });
        outcomes.push(BatchOutcome {
            world_name,
            output,
            result,
        });
    }
    Ok(outcomes)
}
