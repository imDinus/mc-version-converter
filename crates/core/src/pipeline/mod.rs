pub mod batch;

use std::fs;
use std::path::{Path, PathBuf};

use fastnbt::Value;

use crate::chunk::rewrite_region_dir;
use crate::error::{Error, Result};
use crate::logging::log;
use crate::mapping::BlockTable;
use crate::nbt::level_dat::downgrade_level_dat;
use crate::nbt::read_nbt_file;
use crate::report::Report;
use crate::version::{self, VersionInfo, DV_26_1};
use crate::world::layout::{self, Layout, CHUNK_SUBDIRS};

pub struct ConvertOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub target: String,
    pub block_table: Option<PathBuf>,
}

pub struct WorldInfo {
    pub layout: Layout,
    pub data_version: Option<i32>,
    pub version_name: Option<&'static str>,
    pub level_name: Option<String>,
}

pub fn inspect(world: &Path) -> Result<WorldInfo> {
    let layout = layout::detect_layout(world)?;
    let root = read_nbt_file(&world.join("level.dat"))?;

    let mut data_version = None;
    let mut level_name = None;
    if let Value::Compound(root_map) = &root {
        if let Some(Value::Compound(data)) = root_map.get("Data") {
            if let Some(Value::Int(dv)) = data.get("DataVersion") {
                data_version = Some(*dv);
            }
            if let Some(Value::String(name)) = data.get("LevelName") {
                level_name = Some(name.clone());
            }
        }
    }
    let version_name = data_version
        .and_then(version::from_data_version)
        .map(|v| v.name);
    Ok(WorldInfo {
        layout,
        data_version,
        version_name,
        level_name,
    })
}

pub fn convert(options: &ConvertOptions) -> Result<Report> {
    let target: &'static VersionInfo = version::find(&options.target).ok_or_else(|| {
        Error::UnsupportedTarget(format!(
            "'{}' — unknown version. Run `mcconvert versions` for the list.",
            options.target
        ))
    })?;
    if !version::is_supported_target(target) {
        return Err(Error::UnsupportedTarget(format!(
            "'{}' — 26.x versions are inputs, not conversion targets (supported: 1.18.2–1.21.11).",
            target.name
        )));
    }

    log(format!(
        "Validating input world: {}",
        options.input.display()
    ));
    let info = inspect(&options.input)?;
    if info.layout != Layout::New26 {
        return Err(Error::InvalidWorld(
            "input world does not use the 26.x layout (it already looks like the legacy structure)"
                .into(),
        ));
    }
    if let Some(dv) = info.data_version {
        log(format!(
            "Input version: {} (DataVersion {dv})",
            info.version_name.unwrap_or("unknown")
        ));
        if dv <= target.data_version {
            return Err(Error::InvalidWorld(format!(
                "input world (DataVersion {dv}) is not newer than the target ({}, DataVersion {})",
                target.name, target.data_version
            )));
        }
        if dv < DV_26_1 {
            return Err(Error::InvalidWorld(format!(
                "input world (DataVersion {dv}) is older than 26.1 (DataVersion {DV_26_1})"
            )));
        }
    }

    if options.output.exists() {
        let mut entries =
            fs::read_dir(&options.output).map_err(|e| Error::io(&options.output, e))?;
        if entries.next().is_some() {
            return Err(Error::Output(format!(
                "output folder already exists and is not empty: {}",
                options.output.display()
            )));
        }
    }

    let table = options
        .block_table
        .as_deref()
        .map(BlockTable::load_json)
        .transpose()?;
    if let Some(table) = &table {
        log(format!("Loaded block allowlist: {} entries", table.len()));
    }

    let mut report = Report::default();
    if table.is_none() {
        report.warn(
            "No block allowlist (--block-table) provided; skipping replacement of new 26.x \
             blocks with air. Unknown blocks may remain in the converted world.",
        );
    }

    if let Err(error) = run_steps(options, target, table.as_ref(), &mut report) {
        fs::remove_dir_all(&options.output).ok();
        log("Conversion failed; partial output folder was removed");
        return Err(error);
    }

    report.warn(
        "Detailed NBT for blocks/items/mobs added in 26.x may be ignored or lost in older \
         versions. Always test the converted world with a copy.",
    );
    report.dedup_warnings();
    log("Conversion finished");
    Ok(report)
}

fn run_steps(
    options: &ConvertOptions,
    target: &'static VersionInfo,
    table: Option<&BlockTable>,
    report: &mut Report,
) -> Result<()> {
    log("Step 1: remapping folder layout");
    layout::convert_layout(&options.input, &options.output, target.data_version, report)?;

    log("Step 2: rebuilding level.dat");
    downgrade_level_dat(&options.input, &options.output, target, report)?;

    log(format!(
        "Step 3: rewriting chunks (target DataVersion {})",
        target.data_version
    ));
    for mapping in layout::dimension_mappings(&options.input)? {
        for sub in CHUNK_SUBDIRS {
            let in_dir = mapping.input_dir.join(sub);
            let out_dir = options.output.join(&mapping.output_rel).join(sub);
            report.merge(rewrite_region_dir(
                &in_dir,
                &out_dir,
                target.data_version,
                table,
            )?);
        }
    }
    Ok(())
}
