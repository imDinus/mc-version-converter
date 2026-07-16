use std::fs;
use std::path::Path;

use fastanvil::Region;
use fastnbt::Value;
use rayon::prelude::*;

use crate::error::{Error, Result};
use crate::logging::log;
use crate::mapping::{filter_chunk_blocks, BlockTable};
use crate::report::Report;

pub fn rewrite_region_dir(
    in_dir: &Path,
    out_dir: &Path,
    target_data_version: i32,
    table: Option<&BlockTable>,
) -> Result<Report> {
    let mut report = Report::default();
    if !in_dir.is_dir() {
        return Ok(report);
    }
    fs::create_dir_all(out_dir).map_err(|e| Error::io(out_dir, e))?;

    let mut mca_files = Vec::new();
    for entry in fs::read_dir(in_dir).map_err(|e| Error::io(in_dir, e))? {
        let entry = entry.map_err(|e| Error::io(in_dir, e))?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.extension().is_some_and(|e| e == "mca") {
            mca_files.push(path);
        } else {
            let dst = out_dir.join(entry.file_name());
            fs::copy(&path, &dst).map_err(|e| Error::io(&path, e))?;
            report.warn(format!(
                "Copied {} without conversion (.mcc oversized chunks are not supported yet).",
                entry.file_name().to_string_lossy()
            ));
        }
    }

    let results: Vec<Result<Report>> = mca_files
        .par_iter()
        .map(|path| {
            let out_path = out_dir.join(path.file_name().unwrap());
            rewrite_region_file(path, &out_path, target_data_version, table)
        })
        .collect();
    for result in results {
        report.merge(result?);
    }
    Ok(report)
}

fn rewrite_region_file(
    in_path: &Path,
    out_path: &Path,
    target_data_version: i32,
    table: Option<&BlockTable>,
) -> Result<Report> {
    let mut report = Report::default();

    let in_file = fs::File::open(in_path).map_err(|e| Error::io(in_path, e))?;
    let mut in_region = match Region::from_stream(in_file) {
        Ok(region) => region,
        Err(e) => {
            report.warn(format!(
                "{}: file is corrupt and was skipped entirely ({e}). The game may regenerate poi data; region/entity data in it is lost.",
                in_path.display()
            ));
            return Ok(report);
        }
    };

    let out_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(out_path)
        .map_err(|e| Error::io(out_path, e))?;
    let mut out_region = Region::create(out_file).map_err(|e| Error::nbt(out_path.display(), e))?;

    for chunk in in_region.iter() {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                report.chunks_skipped += 1;
                report.warn(format!(
                    "{}: skipped an unreadable chunk ({e}).",
                    in_path.display()
                ));
                continue;
            }
        };
        let mut value: Value = match fastnbt::from_bytes(&chunk.data) {
            Ok(value) => value,
            Err(e) => {
                report.chunks_skipped += 1;
                report.warn(format!(
                    "{} ({}, {}): skipped a chunk with invalid NBT ({e}).",
                    in_path.display(),
                    chunk.x,
                    chunk.z
                ));
                continue;
            }
        };

        if let Value::Compound(map) = &mut value {
            map.insert("DataVersion".into(), Value::Int(target_data_version));
            if let Some(table) = table {
                report.blocks_replaced += filter_chunk_blocks(map, table);
            }
            crate::downgrade::downgrade_compound(map, target_data_version, &mut report);
            strip_light_data(map);
            report.block_entities_added += ensure_block_entities(map);
        }

        let bytes = fastnbt::to_bytes(&value).map_err(|e| Error::nbt(out_path.display(), e))?;
        out_region
            .write_chunk(chunk.x, chunk.z, &bytes)
            .map_err(|e| Error::nbt(out_path.display(), e))?;
        report.chunks_rewritten += 1;
    }

    report.region_files += 1;
    log(format!(
        "{}: rewrote {} chunk(s)",
        in_path.display(),
        report.chunks_rewritten
    ));
    Ok(report)
}

const SYNTHESIZED_BLOCK_ENTITIES: &[(&str, &str)] = &[
    ("minecraft:ender_chest", "minecraft:ender_chest"),
    ("minecraft:enchanting_table", "minecraft:enchanting_table"),
    ("minecraft:daylight_detector", "minecraft:daylight_detector"),
    ("minecraft:conduit", "minecraft:conduit"),
    ("minecraft:end_portal", "minecraft:end_portal"),
    ("minecraft:beacon", "minecraft:beacon"),
    ("minecraft:campfire", "minecraft:campfire"),
    ("minecraft:soul_campfire", "minecraft:campfire"),
    ("minecraft:jukebox", "minecraft:jukebox"),
];

fn block_entity_id_for(block_name: &str) -> Option<&'static str> {
    if block_name.ends_with("_bed") {
        return Some("minecraft:bed");
    }
    SYNTHESIZED_BLOCK_ENTITIES
        .iter()
        .find(|(block, _)| *block == block_name)
        .map(|(_, be)| *be)
}

fn palette_bits(palette_len: usize) -> usize {
    if palette_len <= 1 {
        return 4;
    }
    let needed = (usize::BITS - (palette_len - 1).leading_zeros()) as usize;
    needed.max(4)
}

fn ensure_block_entities(map: &mut std::collections::HashMap<String, Value>) -> u64 {
    use std::collections::{HashMap, HashSet};

    let (Some(Value::Int(chunk_x)), Some(Value::Int(chunk_z))) =
        (map.get("xPos").cloned(), map.get("zPos").cloned())
    else {
        return 0;
    };

    let mut existing: HashSet<(i32, i32, i32)> = HashSet::new();
    if let Some(Value::List(block_entities)) = map.get("block_entities") {
        for be in block_entities {
            let Value::Compound(be) = be else { continue };
            if let (Some(Value::Int(x)), Some(Value::Int(y)), Some(Value::Int(z))) =
                (be.get("x"), be.get("y"), be.get("z"))
            {
                existing.insert((*x, *y, *z));
            }
        }
    }

    let mut added: Vec<Value> = Vec::new();
    if let Some(Value::List(sections)) = map.get("sections") {
        for section in sections {
            let Value::Compound(section) = section else {
                continue;
            };
            let section_y = match section.get("Y") {
                Some(Value::Byte(y)) => *y as i32,
                Some(Value::Int(y)) => *y,
                _ => continue,
            };
            let Some(Value::Compound(block_states)) = section.get("block_states") else {
                continue;
            };
            let Some(Value::List(palette)) = block_states.get("palette") else {
                continue;
            };

            let mut targets: HashMap<usize, &'static str> = HashMap::new();
            for (index, entry) in palette.iter().enumerate() {
                let Value::Compound(block) = entry else {
                    continue;
                };
                let Some(Value::String(name)) = block.get("Name") else {
                    continue;
                };
                if let Some(be_id) = block_entity_id_for(name) {
                    targets.insert(index, be_id);
                }
            }
            if targets.is_empty() {
                continue;
            }

            let bits = palette_bits(palette.len());
            let entries_per_long = 64 / bits;
            let mask = (1u64 << bits) - 1;
            let data: Option<Vec<i64>> = match block_states.get("data") {
                Some(Value::LongArray(data)) => Some(data.iter().copied().collect()),
                _ => None,
            };

            for i in 0..4096usize {
                let palette_index = match &data {
                    None => 0,
                    Some(longs) => {
                        let Some(long) = longs.get(i / entries_per_long) else {
                            continue;
                        };
                        ((*long as u64) >> ((i % entries_per_long) * bits) & mask) as usize
                    }
                };
                let Some(be_id) = targets.get(&palette_index) else {
                    continue;
                };
                let x = chunk_x * 16 + (i & 15) as i32;
                let y = section_y * 16 + ((i >> 8) & 15) as i32;
                let z = chunk_z * 16 + ((i >> 4) & 15) as i32;
                if existing.contains(&(x, y, z)) {
                    continue;
                }
                let mut be = HashMap::new();
                be.insert("id".to_string(), Value::String((*be_id).to_string()));
                be.insert("x".to_string(), Value::Int(x));
                be.insert("y".to_string(), Value::Int(y));
                be.insert("z".to_string(), Value::Int(z));
                be.insert("keepPacked".to_string(), Value::Byte(0));
                added.push(Value::Compound(be));
            }
        }
    }

    let count = added.len() as u64;
    if !added.is_empty() {
        match map.get_mut("block_entities") {
            Some(Value::List(block_entities)) => block_entities.extend(added),
            _ => {
                map.insert("block_entities".to_string(), Value::List(added));
            }
        }
    }
    count
}

fn strip_light_data(map: &mut std::collections::HashMap<String, Value>) {
    if !map.contains_key("sections") && !map.contains_key("isLightOn") {
        return;
    }
    map.remove("isLightOn");
    if let Some(Value::List(sections)) = map.get_mut("sections") {
        for section in sections {
            if let Value::Compound(section) = section {
                section.remove("BlockLight");
                section.remove("SkyLight");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn bed_block_entity_synthesized() {
        let mut bed_props = HashMap::new();
        bed_props.insert("part".to_string(), Value::String("foot".into()));
        let mut bed = HashMap::new();
        bed.insert(
            "Name".to_string(),
            Value::String("minecraft:yellow_bed".into()),
        );
        bed.insert("Properties".to_string(), Value::Compound(bed_props));
        let mut air = HashMap::new();
        air.insert("Name".to_string(), Value::String("minecraft:air".into()));

        let mut block_states = HashMap::new();
        block_states.insert(
            "palette".to_string(),
            Value::List(vec![Value::Compound(air), Value::Compound(bed)]),
        );
        let mut data = vec![0i64; 256];
        data[0] = 1;
        block_states.insert(
            "data".to_string(),
            Value::LongArray(fastnbt::LongArray::new(data)),
        );
        let mut section = HashMap::new();
        section.insert("Y".to_string(), Value::Byte(4));
        section.insert("block_states".to_string(), Value::Compound(block_states));

        let mut chunk = HashMap::new();
        chunk.insert("xPos".to_string(), Value::Int(2));
        chunk.insert("zPos".to_string(), Value::Int(-1));
        chunk.insert(
            "sections".to_string(),
            Value::List(vec![Value::Compound(section)]),
        );

        let added = ensure_block_entities(&mut chunk);
        assert_eq!(added, 1);
        let Some(Value::List(block_entities)) = chunk.get("block_entities") else {
            panic!()
        };
        let Value::Compound(be) = &block_entities[0] else {
            panic!()
        };
        assert_eq!(be.get("id"), Some(&Value::String("minecraft:bed".into())));
        assert_eq!(be.get("x"), Some(&Value::Int(32)));
        assert_eq!(be.get("y"), Some(&Value::Int(64)));
        assert_eq!(be.get("z"), Some(&Value::Int(-16)));

        let added_again = ensure_block_entities(&mut chunk);
        assert_eq!(added_again, 0);
    }

    #[test]
    fn region_dataversion_rewrite() {
        let dir = std::env::temp_dir().join(format!("mcconvert-test-{}", std::process::id()));
        let in_dir = dir.join("in");
        let out_dir = dir.join("out");
        fs::create_dir_all(&in_dir).unwrap();

        let mut section = HashMap::new();
        section.insert("Y".to_string(), Value::Byte(0));
        section.insert(
            "SkyLight".to_string(),
            Value::ByteArray(fastnbt::ByteArray::new(vec![0; 2048])),
        );
        let mut chunk = HashMap::new();
        chunk.insert("DataVersion".to_string(), Value::Int(4786));
        chunk.insert("xPos".to_string(), Value::Int(0));
        chunk.insert("zPos".to_string(), Value::Int(0));
        chunk.insert("isLightOn".to_string(), Value::Byte(1));
        chunk.insert(
            "sections".to_string(),
            Value::List(vec![Value::Compound(section)]),
        );
        let chunk_bytes = fastnbt::to_bytes(&Value::Compound(chunk)).unwrap();

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(in_dir.join("r.0.0.mca"))
            .unwrap();
        let mut region = Region::create(file).unwrap();
        region.write_chunk(0, 0, &chunk_bytes).unwrap();
        drop(region);

        let report = rewrite_region_dir(&in_dir, &out_dir, 4671, None).unwrap();
        assert_eq!(report.chunks_rewritten, 1);
        assert_eq!(report.region_files, 1);

        let out_file = fs::File::open(out_dir.join("r.0.0.mca")).unwrap();
        let mut out_region = Region::from_stream(out_file).unwrap();
        let data = out_region.read_chunk(0, 0).unwrap().unwrap();
        let value: Value = fastnbt::from_bytes(&data).unwrap();
        let Value::Compound(map) = value else {
            panic!("not a compound")
        };
        assert_eq!(map.get("DataVersion"), Some(&Value::Int(4671)));
        assert!(!map.contains_key("isLightOn"));
        let Some(Value::List(sections)) = map.get("sections") else {
            panic!()
        };
        let Value::Compound(section) = &sections[0] else {
            panic!()
        };
        assert!(!section.contains_key("SkyLight"));
        assert!(section.contains_key("Y"));

        fs::remove_dir_all(&dir).ok();
    }
}
