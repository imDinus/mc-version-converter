use std::fs;
use std::path::{Path, PathBuf};

use super::copy_dir_recursive;
use crate::error::{Error, Result};
use crate::logging::log;
use crate::nbt::data_files::{
    convert_raids_file, copy_dat_rewrite_dataversion, copy_player_dat, DIM_MERGE_FILES,
    DIM_SKIP_FILES, SHARED_MERGE_FILES, SHARED_SKIP_FILES,
};
use crate::report::Report;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    New26,
    Legacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimKind {
    Overworld,
    Nether,
    End,
    Custom,
}

pub const CHUNK_SUBDIRS: &[&str] = &["region", "entities", "poi"];

#[derive(Debug)]
pub struct DimMapping {
    pub input_dir: PathBuf,
    pub output_rel: PathBuf,
    pub kind: DimKind,
}

pub fn detect_layout(world: &Path) -> Result<Layout> {
    if !world.join("level.dat").is_file() {
        return Err(Error::InvalidWorld(format!(
            "level.dat not found: {}",
            world.display()
        )));
    }
    if world
        .join("dimensions")
        .join("minecraft")
        .join("overworld")
        .is_dir()
    {
        Ok(Layout::New26)
    } else {
        Ok(Layout::Legacy)
    }
}

pub fn dimension_mappings(input: &Path) -> Result<Vec<DimMapping>> {
    let mut mappings = Vec::new();
    let dims_root = input.join("dimensions");
    if !dims_root.is_dir() {
        return Ok(mappings);
    }
    for ns_entry in fs::read_dir(&dims_root).map_err(|e| Error::io(&dims_root, e))? {
        let ns_entry = ns_entry.map_err(|e| Error::io(&dims_root, e))?;
        if !ns_entry.path().is_dir() {
            continue;
        }
        let ns_name = ns_entry.file_name().to_string_lossy().into_owned();
        for dim_entry in fs::read_dir(ns_entry.path()).map_err(|e| Error::io(ns_entry.path(), e))? {
            let dim_entry = dim_entry.map_err(|e| Error::io(ns_entry.path(), e))?;
            if !dim_entry.path().is_dir() {
                continue;
            }
            let dim_name = dim_entry.file_name().to_string_lossy().into_owned();
            let (output_rel, kind) = match (ns_name.as_str(), dim_name.as_str()) {
                ("minecraft", "overworld") => (PathBuf::new(), DimKind::Overworld),
                ("minecraft", "the_nether") => (PathBuf::from("DIM-1"), DimKind::Nether),
                ("minecraft", "the_end") => (PathBuf::from("DIM1"), DimKind::End),
                _ => (
                    PathBuf::from("dimensions").join(&ns_name).join(&dim_name),
                    DimKind::Custom,
                ),
            };
            mappings.push(DimMapping {
                input_dir: dim_entry.path(),
                output_rel,
                kind,
            });
        }
    }
    Ok(mappings)
}

pub fn convert_layout(
    input: &Path,
    output: &Path,
    target_data_version: i32,
    report: &mut Report,
) -> Result<()> {
    fs::create_dir_all(output).map_err(|e| Error::io(output, e))?;

    for mapping in dimension_mappings(input)? {
        log(format!(
            "Converting dimension: {} → {}",
            mapping.input_dir.display(),
            if mapping.output_rel.as_os_str().is_empty() {
                "(world root)".to_string()
            } else {
                mapping.output_rel.display().to_string()
            }
        ));
        let dim_out = output.join(&mapping.output_rel);
        for entry in
            fs::read_dir(&mapping.input_dir).map_err(|e| Error::io(&mapping.input_dir, e))?
        {
            let entry = entry.map_err(|e| Error::io(&mapping.input_dir, e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if CHUNK_SUBDIRS.contains(&name.as_str()) {
                continue;
            }
            let src = entry.path();
            if name == "data" && src.is_dir() {
                convert_dim_data(
                    &src,
                    &dim_out.join("data"),
                    mapping.kind,
                    target_data_version,
                    report,
                )?;
            } else if src.is_dir() {
                copy_dir_recursive(&src, &dim_out.join(&name))?;
            } else {
                fs::create_dir_all(&dim_out).map_err(|e| Error::io(&dim_out, e))?;
                fs::copy(&src, dim_out.join(&name)).map_err(|e| Error::io(&src, e))?;
            }
        }
        if mapping.kind == DimKind::Custom {
            report.warn(format!(
                "Custom dimension '{}' was copied without structural changes.",
                mapping.output_rel.display()
            ));
        }
    }

    convert_shared_data(input, output, target_data_version, report)?;
    convert_players(input, output, target_data_version, report)?;

    for entry in fs::read_dir(input).map_err(|e| Error::io(input, e))? {
        let entry = entry.map_err(|e| Error::io(input, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        match name.as_str() {
            "level.dat" | "level.dat_old" | "session.lock" | "dimensions" | "players" | "data" => {
                continue
            }
            _ => {
                let src = entry.path();
                let dst = output.join(&name);
                if src.is_dir() {
                    copy_dir_recursive(&src, &dst)?;
                } else {
                    fs::copy(&src, &dst).map_err(|e| Error::io(&src, e))?;
                }
            }
        }
    }
    Ok(())
}

fn convert_dim_data(
    src_data: &Path,
    out_data: &Path,
    kind: DimKind,
    target_data_version: i32,
    report: &mut Report,
) -> Result<()> {
    for ns_entry in fs::read_dir(src_data).map_err(|e| Error::io(src_data, e))? {
        let ns_entry = ns_entry.map_err(|e| Error::io(src_data, e))?;
        let ns_name = ns_entry.file_name().to_string_lossy().into_owned();
        let ns_path = ns_entry.path();

        if ns_name != "minecraft" || !ns_path.is_dir() {
            let dst = out_data.join(&ns_name);
            if ns_path.is_dir() {
                copy_dir_recursive(&ns_path, &dst)?;
            } else {
                fs::create_dir_all(out_data).map_err(|e| Error::io(out_data, e))?;
                fs::copy(&ns_path, &dst).map_err(|e| Error::io(&ns_path, e))?;
            }
            report.warn(format!(
                "Non-vanilla dimension data entry '{ns_name}' was copied preserving its path."
            ));
            continue;
        }

        for file in fs::read_dir(&ns_path).map_err(|e| Error::io(&ns_path, e))? {
            let file = file.map_err(|e| Error::io(&ns_path, e))?;
            let file_name = file.file_name().to_string_lossy().into_owned();
            let file_src = file.path();

            if DIM_MERGE_FILES.contains(&file_name.as_str()) {
                let merged = matches!(
                    (file_name.as_str(), kind),
                    ("ender_dragon_fight.dat", DimKind::End)
                        | ("world_border.dat", DimKind::Overworld)
                );
                if !merged {
                    report.warn(format!(
                        "Older versions do not support per-dimension {file_name}; the file from this dimension was dropped."
                    ));
                }
                continue;
            }
            if DIM_SKIP_FILES.contains(&file_name.as_str()) {
                continue;
            }

            fs::create_dir_all(out_data).map_err(|e| Error::io(out_data, e))?;
            if file_name == "raids.dat" {
                let out_name = if kind == DimKind::End {
                    "raids_end.dat"
                } else {
                    "raids.dat"
                };
                convert_raids_file(
                    &file_src,
                    &out_data.join(out_name),
                    target_data_version,
                    report,
                )?;
            } else if file_src.extension().is_some_and(|e| e == "dat") {
                copy_dat_rewrite_dataversion(
                    &file_src,
                    &out_data.join(&file_name),
                    target_data_version,
                    report,
                )?;
                report.warn(format!(
                    "Unknown dimension data file {file_name} was copied with only its DataVersion rewritten."
                ));
            } else if file_src.is_dir() {
                copy_dir_recursive(&file_src, &out_data.join(&file_name))?;
            } else {
                fs::copy(&file_src, out_data.join(&file_name))
                    .map_err(|e| Error::io(&file_src, e))?;
            }
        }
    }
    Ok(())
}

fn convert_shared_data(
    input: &Path,
    output: &Path,
    target_data_version: i32,
    report: &mut Report,
) -> Result<()> {
    let shared_in = input.join("data");
    if !shared_in.is_dir() {
        return Ok(());
    }
    log("Converting shared data/ folder");
    let data_out = output.join("data");
    for entry in fs::read_dir(&shared_in).map_err(|e| Error::io(&shared_in, e))? {
        let entry = entry.map_err(|e| Error::io(&shared_in, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let src = entry.path();
        if name == "minecraft" && src.is_dir() {
            for file in fs::read_dir(&src).map_err(|e| Error::io(&src, e))? {
                let file = file.map_err(|e| Error::io(&src, e))?;
                let file_name = file.file_name().to_string_lossy().into_owned();
                if SHARED_MERGE_FILES.contains(&file_name.as_str())
                    || SHARED_SKIP_FILES.contains(&file_name.as_str())
                {
                    continue;
                }
                let file_src = file.path();
                fs::create_dir_all(&data_out).map_err(|e| Error::io(&data_out, e))?;
                if file_name == "maps" && file_src.is_dir() {
                    convert_maps_dir(&file_src, &data_out, target_data_version, report)?;
                    continue;
                }
                let file_dst = data_out.join(&file_name);
                if file_src.is_dir() {
                    copy_dir_recursive(&file_src, &file_dst)?;
                } else if file_src.extension().is_some_and(|e| e == "dat") {
                    copy_dat_rewrite_dataversion(
                        &file_src,
                        &file_dst,
                        target_data_version,
                        report,
                    )?;
                } else {
                    fs::copy(&file_src, &file_dst).map_err(|e| Error::io(&file_src, e))?;
                }
            }
        } else {
            let dst = data_out.join(&name);
            if src.is_dir() {
                copy_dir_recursive(&src, &dst)?;
            } else {
                fs::create_dir_all(&data_out).map_err(|e| Error::io(&data_out, e))?;
                fs::copy(&src, &dst).map_err(|e| Error::io(&src, e))?;
            }
            report.warn(format!("data/{name} entry was copied preserving its path."));
        }
    }
    Ok(())
}

fn convert_maps_dir(
    src: &Path,
    data_out: &Path,
    target_data_version: i32,
    report: &mut Report,
) -> Result<()> {
    let mut count = 0;
    for file in fs::read_dir(src).map_err(|e| Error::io(src, e))? {
        let file = file.map_err(|e| Error::io(src, e))?;
        let file_src = file.path();
        let file_name = file.file_name().to_string_lossy().into_owned();
        if file_name == "last_id.dat" {
            copy_dat_rewrite_dataversion(
                &file_src,
                &data_out.join("idcounts.dat"),
                target_data_version,
                report,
            )?;
            continue;
        }
        let stem = file_src
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if stem.chars().all(|c| c.is_ascii_digit())
            && !stem.is_empty()
            && file_src.extension().is_some_and(|e| e == "dat")
        {
            copy_dat_rewrite_dataversion(
                &file_src,
                &data_out.join(format!("map_{stem}.dat")),
                target_data_version,
                report,
            )?;
            count += 1;
        } else {
            fs::copy(&file_src, data_out.join(&file_name)).map_err(|e| Error::io(&file_src, e))?;
            report.warn(format!(
                "Unexpected file in maps folder was copied as-is: {file_name}"
            ));
        }
    }
    log(format!(
        "Converted {count} map file(s) to the legacy data/map_N.dat layout"
    ));
    Ok(())
}

const PLAYERS_SUBDIR_MAP: &[(&str, &str)] = &[
    ("data", "playerdata"),
    ("advancements", "advancements"),
    ("stats", "stats"),
];

fn convert_players(
    input: &Path,
    output: &Path,
    target_data_version: i32,
    report: &mut Report,
) -> Result<()> {
    let players_in = input.join("players");
    if !players_in.is_dir() {
        return Ok(());
    }
    log("Converting players/ folder");
    for entry in fs::read_dir(&players_in).map_err(|e| Error::io(&players_in, e))? {
        let entry = entry.map_err(|e| Error::io(&players_in, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let src = entry.path();

        let out_dir = match PLAYERS_SUBDIR_MAP.iter().find(|(new, _)| *new == name) {
            Some((_, legacy)) if src.is_dir() => output.join(legacy),
            _ => {
                let dst = output.join(&name);
                if src.is_dir() {
                    copy_dir_recursive(&src, &dst)?;
                } else {
                    fs::copy(&src, &dst).map_err(|e| Error::io(&src, e))?;
                }
                report.warn(format!(
                    "Unexpected entry '{name}' in players/ was copied to the output root as-is."
                ));
                continue;
            }
        };

        fs::create_dir_all(&out_dir).map_err(|e| Error::io(&out_dir, e))?;
        for file in fs::read_dir(&src).map_err(|e| Error::io(&src, e))? {
            let file = file.map_err(|e| Error::io(&src, e))?;
            let file_src = file.path();
            let dst = out_dir.join(file.file_name());
            if file_src.is_dir() {
                copy_dir_recursive(&file_src, &dst)?;
            } else if file_src
                .extension()
                .is_some_and(|e| e == "dat" || e == "dat_old")
            {
                copy_player_dat(&file_src, &dst, target_data_version, report)?;
            } else {
                fs::copy(&file_src, &dst).map_err(|e| Error::io(&file_src, e))?;
            }
        }
    }
    Ok(())
}
