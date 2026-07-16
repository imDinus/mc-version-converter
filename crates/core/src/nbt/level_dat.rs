use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use fastnbt::Value;

use super::data_files::{read_wrapped_dat, snake_to_camel, snake_to_pascal};
use super::{read_nbt_file, write_nbt_gzip};
use crate::error::{Error, Result};
use crate::logging::log;
use crate::report::Report;
use crate::version::VersionInfo;

pub fn downgrade_level_dat(
    input_world: &Path,
    output_world: &Path,
    target: &'static VersionInfo,
    report: &mut Report,
) -> Result<()> {
    let path = input_world.join("level.dat");
    let mut root = read_nbt_file(&path)?;

    {
        let Value::Compound(root_map) = &mut root else {
            return Err(Error::nbt("level.dat", "root tag is not a compound"));
        };
        let Some(Value::Compound(data)) = root_map.get_mut("Data") else {
            return Err(Error::nbt("level.dat", "missing Data compound"));
        };

        apply_difficulty(data, report);
        apply_spawn(data, report);
        merge_shared_files(input_world, data, report)?;
        merge_dimension_files(input_world, data, report)?;
        embed_singleplayer(input_world, data, target.data_version, report)?;
        apply_version_tags(data, target);
    }

    write_nbt_gzip(&output_world.join("level.dat"), &root)?;
    log("Finished rebuilding level.dat");
    Ok(())
}

pub(crate) fn apply_difficulty(data: &mut HashMap<String, Value>, report: &mut Report) {
    let Some(ds) = data.remove("difficulty_settings") else {
        return;
    };
    let Value::Compound(ds) = ds else {
        report.warn(
            "difficulty_settings has an unexpected format; using default difficulty (normal).",
        );
        data.insert("Difficulty".into(), Value::Byte(2));
        return;
    };
    let level: i8 = match ds.get("difficulty") {
        Some(Value::String(s)) => match s.as_str() {
            "peaceful" => 0,
            "easy" => 1,
            "normal" => 2,
            "hard" => 3,
            other => {
                report.warn(format!(
                    "Unknown difficulty '{other}' — falling back to normal."
                ));
                2
            }
        },
        _ => {
            report.warn("difficulty string not found — falling back to normal.");
            2
        }
    };
    let locked = matches!(ds.get("locked"), Some(Value::Byte(1)));
    let hardcore = matches!(ds.get("hardcore"), Some(Value::Byte(1)));
    data.insert("Difficulty".into(), Value::Byte(level));
    data.insert("DifficultyLocked".into(), Value::Byte(locked as i8));
    data.insert("hardcore".into(), Value::Byte(hardcore as i8));
    log("Converted difficulty settings");
}

pub(crate) fn apply_spawn(data: &mut HashMap<String, Value>, report: &mut Report) {
    let Some(spawn) = data.remove("spawn") else {
        return;
    };
    let Value::Compound(spawn) = spawn else {
        report.warn("spawn has an unexpected format; skipping spawn conversion.");
        return;
    };
    if let Some(Value::IntArray(pos)) = spawn.get("pos") {
        let pos: Vec<i32> = pos.iter().copied().collect();
        if pos.len() == 3 {
            data.insert("SpawnX".into(), Value::Int(pos[0]));
            data.insert("SpawnY".into(), Value::Int(pos[1]));
            data.insert("SpawnZ".into(), Value::Int(pos[2]));
        }
    }
    if let Some(Value::Float(yaw)) = spawn.get("yaw") {
        data.insert("SpawnAngle".into(), Value::Float(*yaw));
    }
    if let Some(Value::String(dim)) = spawn.get("dimension") {
        if dim != "minecraft:overworld" {
            report.warn(format!(
                "World spawn is in dimension '{dim}', but older versions only support overworld spawns."
            ));
        }
    }
    log("Converted spawn point");
}

fn merge_shared_files(
    world: &Path,
    data: &mut HashMap<String, Value>,
    report: &mut Report,
) -> Result<()> {
    let base = world.join("data").join("minecraft");

    if let Some(rules) = read_wrapped_dat(&base.join("game_rules.dat"))? {
        let converted = convert_game_rules(rules, report);
        log(format!("Merged {} game rules", converted.len()));
        data.insert("GameRules".into(), Value::Compound(converted));
    }

    if let Some(weather) = read_wrapped_dat(&base.join("weather.dat"))? {
        merge_with_key_map(weather, data, WEATHER_KEY_MAP, "weather.dat", report);
        log("Merged weather data");
    }

    if let Some(settings) = read_wrapped_dat(&base.join("world_gen_settings.dat"))? {
        data.insert("WorldGenSettings".into(), Value::Compound(settings));
        log("Merged world generation settings");
    }

    if let Some(events) = read_wrapped_dat(&base.join("custom_boss_events.dat"))? {
        data.insert("CustomBossEvents".into(), Value::Compound(events));
    }

    if let Some(mut scheduled) = read_wrapped_dat(&base.join("scheduled_events.dat"))? {
        if let Some(events @ Value::List(_)) = scheduled.remove("events") {
            data.insert("ScheduledEvents".into(), events);
        }
    }

    if let Some(trader) = read_wrapped_dat(&base.join("wandering_trader.dat"))? {
        merge_with_key_map(trader, data, TRADER_KEY_MAP, "wandering_trader.dat", report);
        log("Merged wandering trader data");
    }

    if let Some(clocks) = read_wrapped_dat(&base.join("world_clocks.dat"))? {
        if let Some(Value::Compound(overworld)) = clocks.get("minecraft:overworld") {
            if let Some(Value::Long(ticks)) = overworld.get("total_ticks") {
                data.insert("DayTime".into(), Value::Long(*ticks));
                log(format!("Merged world clock → DayTime({ticks})"));
            }
        }
    }

    Ok(())
}

fn merge_dimension_files(
    world: &Path,
    data: &mut HashMap<String, Value>,
    report: &mut Report,
) -> Result<()> {
    let dims = world.join("dimensions").join("minecraft");

    let dragon_path = dims
        .join("the_end")
        .join("data")
        .join("minecraft")
        .join("ender_dragon_fight.dat");
    if let Some(fight) = read_wrapped_dat(&dragon_path)? {
        let mut converted = HashMap::new();
        for (key, value) in fight {
            let legacy = match DRAGON_FIGHT_KEY_MAP.iter().find(|(new, _)| *new == key) {
                Some((_, legacy)) => (*legacy).to_string(),
                None => {
                    let generic = snake_to_pascal(&key);
                    report.warn(format!(
                        "Unknown key '{key}' in ender_dragon_fight.dat converted to '{generic}'."
                    ));
                    generic
                }
            };
            converted.insert(legacy, value);
        }
        data.insert("DragonFight".into(), Value::Compound(converted));
        log("Merged dragon fight data");
    }

    let border_path = dims
        .join("overworld")
        .join("data")
        .join("minecraft")
        .join("world_border.dat");
    if let Some(border) = read_wrapped_dat(&border_path)? {
        for (key, value) in border {
            let Some((_, legacy)) = WORLD_BORDER_KEY_MAP.iter().find(|(new, _)| *new == key) else {
                report.warn(format!(
                    "Unknown key '{key}' in world_border.dat was ignored."
                ));
                continue;
            };
            let value = match (*legacy, value) {
                ("BorderWarningBlocks" | "BorderWarningTime", Value::Int(v)) => {
                    Value::Double(v as f64)
                }
                (_, v) => v,
            };
            data.insert((*legacy).to_string(), value);
        }
        log("Merged world border data");
    }

    Ok(())
}

const TRADER_KEY_MAP: &[(&str, &str)] = &[
    ("id", "WanderingTraderId"),
    ("spawn_chance", "WanderingTraderSpawnChance"),
    ("spawn_delay", "WanderingTraderSpawnDelay"),
];

const WEATHER_KEY_MAP: &[(&str, &str)] = &[
    ("raining", "raining"),
    ("thundering", "thundering"),
    ("rain_time", "rainTime"),
    ("thunder_time", "thunderTime"),
    ("clear_weather_time", "clearWeatherTime"),
];

const DRAGON_FIGHT_KEY_MAP: &[(&str, &str)] = &[
    ("dragon_killed", "DragonKilled"),
    ("previously_killed", "PreviouslyKilled"),
    ("needs_state_scanning", "NeedsStateScanning"),
    ("gateways", "Gateways"),
    ("respawn_time", "RespawnTime"),
    ("exit_portal_location", "ExitPortalLocation"),
    ("dragon", "Dragon"),
];

const WORLD_BORDER_KEY_MAP: &[(&str, &str)] = &[
    ("center_x", "BorderCenterX"),
    ("center_z", "BorderCenterZ"),
    ("damage_per_block", "BorderDamagePerBlock"),
    ("lerp_target", "BorderSizeLerpTarget"),
    ("lerp_time", "BorderSizeLerpTime"),
    ("safe_zone", "BorderSafeZone"),
    ("size", "BorderSize"),
    ("warning_blocks", "BorderWarningBlocks"),
    ("warning_time", "BorderWarningTime"),
];

const GAME_RULE_NAME_MAP: &[(&str, &str)] = &[
    ("advance_time", "doDaylightCycle"),
    ("advance_weather", "doWeatherCycle"),
    ("block_drops", "doTileDrops"),
    ("entity_drops", "doEntityDrops"),
    ("mob_drops", "doMobLoot"),
    ("spawn_mobs", "doMobSpawning"),
    ("spawn_phantoms", "doInsomnia"),
    ("spawn_patrols", "doPatrolSpawning"),
    ("spawn_wandering_traders", "doTraderSpawning"),
    ("spawn_wardens", "doWardenSpawning"),
    ("spread_vines", "doVinesSpread"),
    ("fire_tick", "doFireTick"),
    ("immediate_respawn", "doImmediateRespawn"),
    ("limited_crafting", "doLimitedCrafting"),
    ("natural_health_regeneration", "naturalRegeneration"),
    ("respawn_radius", "spawnRadius"),
    ("show_advancement_messages", "announceAdvancements"),
    ("max_block_modifications", "commandModificationBlockLimit"),
    ("max_command_forks", "maxCommandForkCount"),
    ("max_command_sequence_length", "maxCommandChainLength"),
    ("max_snow_accumulation_height", "snowAccumulationHeight"),
    ("command_blocks_work", "commandBlocksEnabled"),
];

const GAME_RULE_INVERTED: &[(&str, &str)] = &[
    ("elytra_movement_check", "disableElytraMovementCheck"),
    ("player_movement_check", "disablePlayerMovementCheck"),
    ("raids", "disableRaids"),
];

pub(crate) fn convert_game_rules(
    rules: HashMap<String, Value>,
    report: &mut Report,
) -> HashMap<String, Value> {
    let mut converted = HashMap::new();
    for (key, value) in rules {
        let name = key.strip_prefix("minecraft:").unwrap_or(&key);

        let (string_value, is_bool) = match &value {
            Value::Byte(0) => ("false".to_string(), true),
            Value::Byte(_) => ("true".to_string(), true),
            Value::Int(v) => (v.to_string(), false),
            Value::Long(v) => (v.to_string(), false),
            Value::String(s) => (s.clone(), false),
            other => {
                report.warn(format!(
                    "Could not interpret value ({other:?}) of game rule '{key}'; skipping."
                ));
                continue;
            }
        };

        if let Some((_, legacy)) = GAME_RULE_INVERTED.iter().find(|(new, _)| *new == name) {
            if is_bool {
                let inverted = if string_value == "true" {
                    "false"
                } else {
                    "true"
                };
                converted.insert((*legacy).to_string(), Value::String(inverted.into()));
                continue;
            }
        }
        let legacy = match GAME_RULE_NAME_MAP.iter().find(|(new, _)| *new == name) {
            Some((_, legacy)) => (*legacy).to_string(),
            None => snake_to_camel(name),
        };
        converted.insert(legacy, Value::String(string_value));
    }
    converted
}

fn merge_with_key_map(
    map: HashMap<String, Value>,
    data: &mut HashMap<String, Value>,
    key_map: &[(&str, &str)],
    file: &str,
    report: &mut Report,
) {
    for (key, value) in map {
        match key_map.iter().find(|(new, _)| *new == key) {
            Some((_, old)) => {
                data.insert((*old).into(), value);
            }
            None => {
                report.warn(format!(
                    "Unknown key '{key}' in {file} merged with its original name."
                ));
                data.insert(key, value);
            }
        }
    }
}

fn embed_singleplayer(
    world: &Path,
    data: &mut HashMap<String, Value>,
    target_data_version: i32,
    report: &mut Report,
) -> Result<()> {
    let Some(value) = data.remove("singleplayer_uuid") else {
        return Ok(());
    };
    let Value::IntArray(arr) = &value else {
        report.warn("singleplayer_uuid has an unexpected format; skipping player embedding.");
        return Ok(());
    };
    let ints: Vec<i32> = arr.iter().copied().collect();
    if ints.len() != 4 {
        report.warn("singleplayer_uuid length is not 4; skipping player embedding.");
        return Ok(());
    }
    let uuid = format_uuid(&ints);
    match find_player_file(&world.join("players"), &uuid) {
        Some(path) => {
            let player = read_nbt_file(&path)?;
            if let Value::Compound(mut player_map) = player {
                if player_map.contains_key("DataVersion") {
                    player_map.insert("DataVersion".into(), Value::Int(target_data_version));
                }
                crate::downgrade::downgrade_compound(&mut player_map, target_data_version, report);
                data.insert("Player".into(), Value::Compound(player_map));
                log(format!("Embedded singleplayer data (uuid: {uuid})"));
            } else {
                report.warn(format!("Player file has an unexpected format: {}", path.display()));
            }
        }
        None => report.warn(format!(
            "Could not find the singleplayer data file (uuid: {uuid}). The world may start with a fresh player in older versions."
        )),
    }
    Ok(())
}

pub(crate) fn format_uuid(ints: &[i32]) -> String {
    let mut value: u128 = 0;
    for &part in ints {
        value = (value << 32) | (part as u32 as u128);
    }
    let hex = format!("{value:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn find_player_file(dir: &Path, uuid: &str) -> Option<PathBuf> {
    let undashed = uuid.replace('-', "");
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_player_file(&path, uuid) {
                return Some(found);
            }
        } else if path.extension().is_some_and(|e| e == "dat") {
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            if stem == uuid || stem == undashed {
                return Some(path);
            }
        }
    }
    None
}

fn apply_version_tags(data: &mut HashMap<String, Value>, target: &'static VersionInfo) {
    data.insert("DataVersion".into(), Value::Int(target.data_version));
    let mut version = HashMap::new();
    version.insert("Id".into(), Value::Int(target.data_version));
    version.insert("Name".into(), Value::String(target.name.into()));
    version.insert("Series".into(), Value::String("main".into()));
    version.insert("Snapshot".into(), Value::Byte(0));
    data.insert("Version".into(), Value::Compound(version));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version;
    use fastnbt::IntArray;

    #[test]
    fn difficulty_and_hardcore_conversion() {
        let mut data = HashMap::new();
        let mut ds = HashMap::new();
        ds.insert("difficulty".into(), Value::String("hard".into()));
        ds.insert("hardcore".into(), Value::Byte(1));
        ds.insert("locked".into(), Value::Byte(0));
        data.insert("difficulty_settings".into(), Value::Compound(ds));

        let mut report = Report::default();
        apply_difficulty(&mut data, &mut report);

        assert_eq!(data.get("Difficulty"), Some(&Value::Byte(3)));
        assert_eq!(data.get("DifficultyLocked"), Some(&Value::Byte(0)));
        assert_eq!(data.get("hardcore"), Some(&Value::Byte(1)));
        assert!(!data.contains_key("difficulty_settings"));
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn spawn_conversion() {
        let mut data = HashMap::new();
        let mut spawn = HashMap::new();
        spawn.insert(
            "dimension".into(),
            Value::String("minecraft:overworld".into()),
        );
        spawn.insert(
            "pos".into(),
            Value::IntArray(IntArray::new(vec![10, 69, -3])),
        );
        spawn.insert("yaw".into(), Value::Float(90.0));
        spawn.insert("pitch".into(), Value::Float(0.0));
        data.insert("spawn".into(), Value::Compound(spawn));

        let mut report = Report::default();
        apply_spawn(&mut data, &mut report);

        assert_eq!(data.get("SpawnX"), Some(&Value::Int(10)));
        assert_eq!(data.get("SpawnY"), Some(&Value::Int(69)));
        assert_eq!(data.get("SpawnZ"), Some(&Value::Int(-3)));
        assert_eq!(data.get("SpawnAngle"), Some(&Value::Float(90.0)));
        assert!(!data.contains_key("spawn"));
    }

    #[test]
    fn game_rule_conversion() {
        let mut rules = HashMap::new();
        rules.insert("minecraft:keep_inventory".to_string(), Value::Byte(1));
        rules.insert("minecraft:advance_time".to_string(), Value::Byte(0));
        rules.insert("minecraft:random_tick_speed".to_string(), Value::Int(3));
        rules.insert("minecraft:raids".to_string(), Value::Byte(1));
        rules.insert(
            "minecraft:elytra_movement_check".to_string(),
            Value::Byte(0),
        );

        let mut report = Report::default();
        let converted = convert_game_rules(rules, &mut report);

        assert_eq!(
            converted.get("keepInventory"),
            Some(&Value::String("true".into()))
        );
        assert_eq!(
            converted.get("doDaylightCycle"),
            Some(&Value::String("false".into()))
        );
        assert_eq!(
            converted.get("randomTickSpeed"),
            Some(&Value::String("3".into()))
        );
        assert_eq!(
            converted.get("disableRaids"),
            Some(&Value::String("false".into()))
        );
        assert_eq!(
            converted.get("disableElytraMovementCheck"),
            Some(&Value::String("true".into()))
        );
    }

    #[test]
    fn uuid_format() {
        assert_eq!(
            format_uuid(&[0, 0, 0, 1]),
            "00000000-0000-0000-0000-000000000001"
        );
        assert_eq!(
            format_uuid(&[-1843843886, 1685802836, -1916530805, 1963746845]),
            "92192cd2-647b-4f54-8dc4-0f8b750c661d"
        );
    }

    #[test]
    fn version_tag_reset() {
        let mut data = HashMap::new();
        let target = version::find("1.21.11").unwrap();
        apply_version_tags(&mut data, target);

        assert_eq!(data.get("DataVersion"), Some(&Value::Int(4671)));
        let Some(Value::Compound(v)) = data.get("Version") else {
            panic!("missing Version compound");
        };
        assert_eq!(v.get("Name"), Some(&Value::String("1.21.11".into())));
    }
}
