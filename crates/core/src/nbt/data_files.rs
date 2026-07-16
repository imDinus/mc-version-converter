use std::collections::HashMap;
use std::fs;
use std::path::Path;

use fastnbt::Value;

use super::{read_nbt_file, write_nbt_gzip};
use crate::error::{Error, Result};
use crate::report::Report;

pub const SHARED_MERGE_FILES: &[&str] = &[
    "game_rules.dat",
    "weather.dat",
    "world_gen_settings.dat",
    "custom_boss_events.dat",
    "scheduled_events.dat",
    "world_clocks.dat",
    "wandering_trader.dat",
];

pub const SHARED_SKIP_FILES: &[&str] = &["stopwatches.dat"];

pub const DIM_MERGE_FILES: &[&str] = &["ender_dragon_fight.dat", "world_border.dat"];

pub const DIM_SKIP_FILES: &[&str] = &["chunk_tickets.dat"];

pub fn split_wrapper(mut map: HashMap<String, Value>) -> HashMap<String, Value> {
    map.remove("DataVersion");
    if map.len() == 1 && matches!(map.get("data"), Some(Value::Compound(_))) {
        if let Some(Value::Compound(inner)) = map.remove("data") {
            return inner;
        }
    }
    map
}

pub fn read_wrapped_dat(path: &Path) -> Result<Option<HashMap<String, Value>>> {
    if !path.is_file() {
        return Ok(None);
    }
    let value = read_nbt_file(path)?;
    let Value::Compound(map) = value else {
        return Err(Error::nbt(path.display(), "root tag is not a compound"));
    };
    Ok(Some(split_wrapper(map)))
}

pub fn copy_dat_rewrite_dataversion(
    src: &Path,
    dst: &Path,
    target_data_version: i32,
    report: &mut Report,
) -> Result<()> {
    match read_nbt_file(src) {
        Ok(Value::Compound(mut map)) => {
            if map.contains_key("DataVersion") {
                map.insert("DataVersion".into(), Value::Int(target_data_version));
            }
            write_nbt_gzip(dst, &Value::Compound(map))
        }
        _ => {
            fs::copy(src, dst).map_err(|e| Error::io(src, e))?;
            report.warn(format!(
                "Could not parse {} as NBT; copied as-is.",
                src.display()
            ));
            Ok(())
        }
    }
}

pub fn copy_player_dat(
    src: &Path,
    dst: &Path,
    target_data_version: i32,
    report: &mut Report,
) -> Result<()> {
    match read_nbt_file(src) {
        Ok(Value::Compound(mut map)) => {
            if map.contains_key("DataVersion") {
                map.insert("DataVersion".into(), Value::Int(target_data_version));
            }
            crate::downgrade::downgrade_compound(&mut map, target_data_version, report);
            write_nbt_gzip(dst, &Value::Compound(map))
        }
        _ => {
            fs::copy(src, dst).map_err(|e| Error::io(src, e))?;
            report.warn(format!(
                "Could not parse {} as NBT; copied as-is.",
                src.display()
            ));
            Ok(())
        }
    }
}

pub fn snake_to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper_next = false;
    for ch in s.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn snake_to_pascal(s: &str) -> String {
    let camel = snake_to_camel(s);
    let mut chars = camel.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => camel,
    }
}

const RAIDS_KEY_MAP: &[(&str, &str)] = &[
    ("next_id", "NextAvailableID"),
    ("tick", "Tick"),
    ("raids", "Raids"),
];

pub fn convert_raids_file(
    src: &Path,
    dst: &Path,
    target_data_version: i32,
    report: &mut Report,
) -> Result<()> {
    let Some(inner) = read_wrapped_dat(src)? else {
        return Ok(());
    };
    let mut converted = HashMap::new();
    for (key, value) in inner {
        match RAIDS_KEY_MAP.iter().find(|(new, _)| *new == key) {
            Some((_, legacy)) => {
                converted.insert((*legacy).to_string(), value);
            }
            None => {
                let legacy = snake_to_pascal(&key);
                report.warn(format!(
                    "Unknown key '{key}' in raids.dat converted to '{legacy}'."
                ));
                converted.insert(legacy, value);
            }
        }
    }
    let mut root = HashMap::new();
    root.insert("DataVersion".to_string(), Value::Int(target_data_version));
    root.insert("data".to_string(), Value::Compound(converted));
    write_nbt_gzip(dst, &Value::Compound(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_conversion() {
        assert_eq!(snake_to_camel("keep_inventory"), "keepInventory");
        assert_eq!(snake_to_camel("raining"), "raining");
        assert_eq!(snake_to_pascal("dragon_killed"), "DragonKilled");
        assert_eq!(snake_to_pascal("gateways"), "Gateways");
    }

    #[test]
    fn wrapper_unwrap() {
        let mut inner = HashMap::new();
        inner.insert("tick".to_string(), Value::Int(3));
        let mut root = HashMap::new();
        root.insert("DataVersion".to_string(), Value::Int(4786));
        root.insert("data".to_string(), Value::Compound(inner));

        let unwrapped = split_wrapper(root);
        assert_eq!(unwrapped.get("tick"), Some(&Value::Int(3)));

        let mut player = HashMap::new();
        player.insert("DataVersion".to_string(), Value::Int(4786));
        player.insert("Health".to_string(), Value::Float(20.0));
        player.insert("Air".to_string(), Value::Short(300));
        let kept = split_wrapper(player);
        assert_eq!(kept.get("Health"), Some(&Value::Float(20.0)));
        assert!(kept.contains_key("Air"));
    }
}
