use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use fastanvil::Region;
use fastnbt::{IntArray, Value};
use mcconvert_core::nbt::{read_nbt_file, write_nbt_gzip};
use mcconvert_core::pipeline::{convert, ConvertOptions};

fn compound(entries: Vec<(&str, Value)>) -> Value {
    let mut map = HashMap::new();
    for (key, value) in entries {
        map.insert(key.to_string(), value);
    }
    Value::Compound(map)
}

fn write_wrapped_dat(path: &Path, data: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let root = compound(vec![("DataVersion", Value::Int(4786)), ("data", data)]);
    write_nbt_gzip(path, &root).unwrap();
}

fn write_region(path: &Path, data_version: i32) {
    write_region_with_block_entities(path, data_version, Vec::new());
}

fn write_region_with_block_entities(path: &Path, data_version: i32, block_entities: Vec<Value>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let chunk = compound(vec![
        ("DataVersion", Value::Int(data_version)),
        ("xPos", Value::Int(0)),
        ("zPos", Value::Int(0)),
        ("block_entities", Value::List(block_entities)),
    ]);
    let bytes = fastnbt::to_bytes(&chunk).unwrap();
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .unwrap();
    let mut region = Region::create(file).unwrap();
    region.write_chunk(0, 0, &bytes).unwrap();
}

const UUID: &str = "00000000-0000-0000-0000-000000000001";

fn build_synthetic_world(root: &Path) {
    fs::create_dir_all(root).unwrap();

    let level = compound(vec![(
        "Data",
        compound(vec![
            ("LevelName", Value::String("TestWorld".into())),
            ("DataVersion", Value::Int(4786)),
            ("GameType", Value::Int(0)),
            ("Time", Value::Long(32)),
            (
                "difficulty_settings",
                compound(vec![
                    ("difficulty", Value::String("hard".into())),
                    ("hardcore", Value::Byte(0)),
                    ("locked", Value::Byte(1)),
                ]),
            ),
            (
                "spawn",
                compound(vec![
                    ("dimension", Value::String("minecraft:overworld".into())),
                    ("pos", Value::IntArray(IntArray::new(vec![0, 69, -8]))),
                    ("yaw", Value::Float(90.0)),
                    ("pitch", Value::Float(0.0)),
                ]),
            ),
            (
                "singleplayer_uuid",
                Value::IntArray(IntArray::new(vec![0, 0, 0, 1])),
            ),
        ]),
    )]);
    write_nbt_gzip(&root.join("level.dat"), &level).unwrap();

    let chest_item = compound(vec![
        ("id", Value::String("minecraft:diamond_sword".into())),
        ("count", Value::Int(1)),
        ("Slot", Value::Byte(0)),
        (
            "components",
            Value::Compound({
                let mut c = HashMap::new();
                c.insert("minecraft:damage".to_string(), Value::Int(5));
                c
            }),
        ),
    ]);
    let chest = compound(vec![
        ("id", Value::String("minecraft:chest".into())),
        ("x", Value::Int(0)),
        ("y", Value::Int(64)),
        ("z", Value::Int(0)),
        ("Items", Value::List(vec![chest_item])),
    ]);
    let sign = compound(vec![
        ("id", Value::String("minecraft:sign".into())),
        ("x", Value::Int(1)),
        ("y", Value::Int(64)),
        ("z", Value::Int(0)),
        (
            "front_text",
            compound(vec![
                (
                    "messages",
                    Value::List(vec![
                        Value::String("hello".into()),
                        Value::String("".into()),
                        Value::String("".into()),
                        Value::String("".into()),
                    ]),
                ),
                ("color", Value::String("black".into())),
                ("has_glowing_text", Value::Byte(0)),
            ]),
        ),
        ("back_text", Value::Compound(HashMap::new())),
        ("is_waxed", Value::Byte(0)),
    ]);
    write_region_with_block_entities(
        &root.join("dimensions/minecraft/overworld/region/r.0.0.mca"),
        4786,
        vec![chest, sign],
    );
    write_region(
        &root.join("dimensions/minecraft/the_nether/region/r.0.0.mca"),
        4786,
    );

    write_wrapped_dat(
        &root.join("data/minecraft/game_rules.dat"),
        compound(vec![
            ("minecraft:keep_inventory", Value::Byte(1)),
            ("minecraft:advance_time", Value::Byte(0)),
            ("minecraft:random_tick_speed", Value::Int(3)),
            ("minecraft:raids", Value::Byte(1)),
        ]),
    );
    write_wrapped_dat(
        &root.join("data/minecraft/weather.dat"),
        compound(vec![
            ("raining", Value::Byte(1)),
            ("rain_time", Value::Int(100)),
            ("thundering", Value::Byte(0)),
            ("thunder_time", Value::Int(5)),
            ("clear_weather_time", Value::Int(0)),
        ]),
    );
    write_wrapped_dat(
        &root.join("data/minecraft/world_clocks.dat"),
        compound(vec![(
            "minecraft:overworld",
            compound(vec![("total_ticks", Value::Long(1234))]),
        )]),
    );
    write_wrapped_dat(
        &root.join("data/minecraft/stopwatches.dat"),
        compound(vec![]),
    );
    write_wrapped_dat(
        &root.join("data/minecraft/scoreboard.dat"),
        compound(vec![]),
    );
    write_wrapped_dat(
        &root.join("data/minecraft/maps/0.dat"),
        compound(vec![("xCenter", Value::Int(64))]),
    );
    write_wrapped_dat(
        &root.join("data/minecraft/maps/last_id.dat"),
        compound(vec![("map", Value::Int(0))]),
    );

    write_wrapped_dat(
        &root.join("dimensions/minecraft/the_end/data/minecraft/ender_dragon_fight.dat"),
        compound(vec![
            ("dragon_killed", Value::Byte(1)),
            ("previously_killed", Value::Byte(1)),
            ("needs_state_scanning", Value::Byte(0)),
            ("respawn_time", Value::Int(0)),
        ]),
    );
    write_wrapped_dat(
        &root.join("dimensions/minecraft/overworld/data/minecraft/world_border.dat"),
        compound(vec![
            ("center_x", Value::Double(1.5)),
            ("size", Value::Double(1000.0)),
            ("warning_blocks", Value::Int(5)),
        ]),
    );
    write_wrapped_dat(
        &root.join("dimensions/minecraft/overworld/data/minecraft/raids.dat"),
        compound(vec![("next_id", Value::Int(1)), ("tick", Value::Int(0))]),
    );
    write_wrapped_dat(
        &root.join("dimensions/minecraft/overworld/data/minecraft/chunk_tickets.dat"),
        compound(vec![]),
    );

    let player = compound(vec![
        ("DataVersion", Value::Int(4786)),
        ("Health", Value::Float(20.0)),
        (
            "Inventory",
            Value::List(vec![
                compound(vec![
                    ("id", Value::String("minecraft:apple".into())),
                    ("count", Value::Int(3)),
                    ("Slot", Value::Byte(0)),
                ]),
                compound(vec![
                    ("id", Value::String("minecraft:shulker_box".into())),
                    ("count", Value::Int(1)),
                    ("Slot", Value::Byte(1)),
                    (
                        "components",
                        Value::Compound(
                            vec![(
                                "minecraft:container".to_string(),
                                Value::List(vec![compound(vec![
                                    ("slot", Value::Int(0)),
                                    (
                                        "item",
                                        compound(vec![
                                            ("id", Value::String("minecraft:diamond_boots".into())),
                                            (
                                                "components",
                                                Value::Compound(
                                                    vec![(
                                                        "minecraft:enchantments".to_string(),
                                                        Value::Compound(
                                                            vec![(
                                                                "minecraft:feather_falling"
                                                                    .to_string(),
                                                                Value::Int(7),
                                                            )]
                                                            .into_iter()
                                                            .collect(),
                                                        ),
                                                    )]
                                                    .into_iter()
                                                    .collect(),
                                                ),
                                            ),
                                        ]),
                                    ),
                                ])]),
                            )]
                            .into_iter()
                            .collect(),
                        ),
                    ),
                ]),
            ]),
        ),
    ]);
    fs::create_dir_all(root.join("players/data")).unwrap();
    write_nbt_gzip(&root.join(format!("players/data/{UUID}.dat")), &player).unwrap();
    fs::create_dir_all(root.join("players/advancements")).unwrap();
    fs::write(root.join(format!("players/advancements/{UUID}.json")), "{}").unwrap();
    fs::create_dir_all(root.join("players/stats")).unwrap();
    fs::write(root.join(format!("players/stats/{UUID}.json")), "{}").unwrap();

    fs::write(root.join("icon.png"), b"png-dummy").unwrap();
}

fn read_level_data(path: &Path) -> HashMap<String, Value> {
    let Value::Compound(mut root) = read_nbt_file(path).unwrap() else {
        panic!("level.dat root is not a compound");
    };
    let Some(Value::Compound(data)) = root.remove("Data") else {
        panic!("missing Data compound");
    };
    data
}

fn string_value(v: &str) -> Value {
    Value::String(v.to_string())
}

#[test]
fn full_conversion_of_synthetic_26x_world() {
    let base = std::env::temp_dir().join(format!("mcconvert-e2e-{}", std::process::id()));
    fs::remove_dir_all(&base).ok();
    let input = base.join("world_26x");
    let output = base.join("world_121");
    build_synthetic_world(&input);

    let report = convert(&ConvertOptions {
        input: input.clone(),
        output: output.clone(),
        target: "1.21.11".into(),
        block_table: None,
    })
    .unwrap();

    assert_eq!(report.chunks_rewritten, 2);
    let out_file = fs::File::open(output.join("region/r.0.0.mca")).unwrap();
    let mut region = Region::from_stream(out_file).unwrap();
    let chunk_data = region.read_chunk(0, 0).unwrap().unwrap();
    let Value::Compound(chunk) = fastnbt::from_bytes(&chunk_data).unwrap() else {
        panic!("chunk is not a compound");
    };
    assert_eq!(chunk.get("DataVersion"), Some(&Value::Int(4671)));
    assert!(output.join("DIM-1/region/r.0.0.mca").is_file());

    let data = read_level_data(&output.join("level.dat"));
    assert_eq!(data.get("DataVersion"), Some(&Value::Int(4671)));
    assert_eq!(data.get("Difficulty"), Some(&Value::Byte(3)));
    assert_eq!(data.get("DifficultyLocked"), Some(&Value::Byte(1)));
    assert_eq!(data.get("hardcore"), Some(&Value::Byte(0)));
    assert_eq!(data.get("SpawnX"), Some(&Value::Int(0)));
    assert_eq!(data.get("SpawnY"), Some(&Value::Int(69)));
    assert_eq!(data.get("SpawnZ"), Some(&Value::Int(-8)));
    assert_eq!(data.get("SpawnAngle"), Some(&Value::Float(90.0)));
    assert!(!data.contains_key("difficulty_settings"));
    assert!(!data.contains_key("singleplayer_uuid"));
    assert!(!data.contains_key("spawn"));

    let Some(Value::Compound(rules)) = data.get("GameRules") else {
        panic!("missing GameRules");
    };
    assert_eq!(rules.get("keepInventory"), Some(&string_value("true")));
    assert_eq!(rules.get("doDaylightCycle"), Some(&string_value("false")));
    assert_eq!(rules.get("randomTickSpeed"), Some(&string_value("3")));
    assert_eq!(rules.get("disableRaids"), Some(&string_value("false")));

    assert_eq!(data.get("raining"), Some(&Value::Byte(1)));
    assert_eq!(data.get("rainTime"), Some(&Value::Int(100)));
    assert_eq!(data.get("clearWeatherTime"), Some(&Value::Int(0)));

    assert_eq!(data.get("DayTime"), Some(&Value::Long(1234)));

    let Some(Value::Compound(fight)) = data.get("DragonFight") else {
        panic!("missing DragonFight");
    };
    assert_eq!(fight.get("DragonKilled"), Some(&Value::Byte(1)));
    assert_eq!(fight.get("PreviouslyKilled"), Some(&Value::Byte(1)));

    assert_eq!(data.get("BorderCenterX"), Some(&Value::Double(1.5)));
    assert_eq!(data.get("BorderSize"), Some(&Value::Double(1000.0)));
    assert_eq!(data.get("BorderWarningBlocks"), Some(&Value::Double(5.0)));

    let Some(Value::Compound(player)) = data.get("Player") else {
        panic!("missing Player");
    };
    assert_eq!(player.get("Health"), Some(&Value::Float(20.0)));
    assert_eq!(player.get("DataVersion"), Some(&Value::Int(4671)));

    let playerdata_file = output.join(format!("playerdata/{UUID}.dat"));
    assert!(playerdata_file.is_file());
    let Value::Compound(copied) = read_nbt_file(&playerdata_file).unwrap() else {
        panic!("player file is not a compound");
    };
    assert_eq!(copied.get("DataVersion"), Some(&Value::Int(4671)));
    assert!(output.join(format!("advancements/{UUID}.json")).is_file());
    assert!(output.join(format!("stats/{UUID}.json")).is_file());

    let Value::Compound(scoreboard) = read_nbt_file(&output.join("data/scoreboard.dat")).unwrap()
    else {
        panic!("scoreboard is not a compound");
    };
    assert_eq!(scoreboard.get("DataVersion"), Some(&Value::Int(4671)));
    assert!(!output.join("data/stopwatches.dat").exists());
    assert!(!output.join("data/game_rules.dat").exists());
    assert!(!output.join("data/minecraft").exists());

    let Value::Compound(map_file) = read_nbt_file(&output.join("data/map_0.dat")).unwrap() else {
        panic!("map_0 is not a compound");
    };
    assert_eq!(map_file.get("DataVersion"), Some(&Value::Int(4671)));
    assert!(output.join("data/idcounts.dat").is_file());
    assert!(!output.join("data/maps").exists());

    let Value::Compound(mut raids) = read_nbt_file(&output.join("data/raids.dat")).unwrap() else {
        panic!("raids is not a compound");
    };
    assert_eq!(raids.get("DataVersion"), Some(&Value::Int(4671)));
    let Some(Value::Compound(raids_data)) = raids.remove("data") else {
        panic!("missing raids data");
    };
    assert_eq!(raids_data.get("NextAvailableID"), Some(&Value::Int(1)));
    assert_eq!(raids_data.get("Tick"), Some(&Value::Int(0)));
    assert!(!output.join("data/world_border.dat").exists());
    assert!(!output.join("data/chunk_tickets.dat").exists());
    assert!(!output.join("DIM1/data/ender_dragon_fight.dat").exists());

    assert!(output.join("icon.png").is_file());

    assert!(input
        .join("dimensions/minecraft/overworld/region/r.0.0.mca")
        .is_file());
    let original = read_level_data(&input.join("level.dat"));
    assert_eq!(original.get("DataVersion"), Some(&Value::Int(4786)));

    fs::remove_dir_all(&base).ok();
}

#[test]
fn batch_conversion() {
    let base = std::env::temp_dir().join(format!("mcconvert-e2e-batch-{}", std::process::id()));
    fs::remove_dir_all(&base).ok();
    let input_dir = base.join("input_worlds");
    let output_dir = base.join("output_worlds");

    build_synthetic_world(&input_dir.join("WorldA"));
    build_synthetic_world(&input_dir.join("WorldB"));
    fs::create_dir_all(input_dir.join("not_a_world")).unwrap();

    let outcomes =
        mcconvert_core::pipeline::batch::convert_batch(&input_dir, &output_dir, "1.21.11", None)
            .unwrap();

    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().all(|o| o.result.is_ok()));
    assert!(output_dir
        .join("WorldA (1.21.11)/region/r.0.0.mca")
        .is_file());
    assert!(output_dir.join("WorldB (1.21.11)/level.dat").is_file());

    fs::remove_dir_all(&base).ok();
}

#[test]
fn legacy_layout_rejected() {
    let base = std::env::temp_dir().join(format!("mcconvert-e2e-legacy-{}", std::process::id()));
    fs::remove_dir_all(&base).ok();
    let input = base.join("legacy");
    fs::create_dir_all(&input).unwrap();
    let level = compound(vec![(
        "Data",
        compound(vec![("DataVersion", Value::Int(4671))]),
    )]);
    write_nbt_gzip(&input.join("level.dat"), &level).unwrap();

    let result = convert(&ConvertOptions {
        input,
        output: base.join("out"),
        target: "1.21.11".into(),
        block_table: None,
    });
    assert!(result.is_err());

    fs::remove_dir_all(&base).ok();
}

#[test]
fn unsupported_target_rejected() {
    let base = std::env::temp_dir().join(format!("mcconvert-e2e-target-{}", std::process::id()));
    fs::remove_dir_all(&base).ok();
    let input = base.join("world");
    build_synthetic_world(&input);

    let result = convert(&ConvertOptions {
        input,
        output: base.join("out"),
        target: "26.1".into(),
        block_table: None,
    });
    assert!(result.is_err());

    fs::remove_dir_all(&base).ok();
}

#[test]
fn downgrade_to_1_18_2() {
    let base = std::env::temp_dir().join(format!("mcconvert-e2e-118-{}", std::process::id()));
    fs::remove_dir_all(&base).ok();
    let input = base.join("world_26x");
    let output = base.join("world_118");
    build_synthetic_world(&input);

    let report = convert(&ConvertOptions {
        input,
        output: output.clone(),
        target: "1.18.2".into(),
        block_table: None,
    })
    .unwrap();
    assert!(report.items_converted >= 3);

    let out_file = fs::File::open(output.join("region/r.0.0.mca")).unwrap();
    let mut region = Region::from_stream(out_file).unwrap();
    let chunk_data = region.read_chunk(0, 0).unwrap().unwrap();
    let Value::Compound(chunk) = fastnbt::from_bytes(&chunk_data).unwrap() else {
        panic!("chunk is not a compound");
    };
    assert_eq!(chunk.get("DataVersion"), Some(&Value::Int(2975)));

    let Some(Value::List(block_entities)) = chunk.get("block_entities") else {
        panic!("missing block_entities");
    };
    let find = |id: &str| {
        block_entities.iter().find_map(|be| {
            let Value::Compound(be) = be else { return None };
            match be.get("id") {
                Some(Value::String(s)) if s == id => Some(be),
                _ => None,
            }
        })
    };

    let chest = find("minecraft:chest").expect("missing chest");
    let Some(Value::List(items)) = chest.get("Items") else {
        panic!("missing Items")
    };
    let Value::Compound(item) = &items[0] else {
        panic!()
    };
    assert_eq!(item.get("Count"), Some(&Value::Byte(1)));
    assert_eq!(item.get("Slot"), Some(&Value::Byte(0)));
    assert!(!item.contains_key("count"));
    assert!(!item.contains_key("components"));
    let Some(Value::Compound(tag)) = item.get("tag") else {
        panic!("missing tag")
    };
    assert_eq!(tag.get("Damage"), Some(&Value::Int(5)));

    let sign = find("minecraft:sign").expect("missing sign");
    assert_eq!(
        sign.get("Text1"),
        Some(&Value::String("{\"text\":\"hello\"}".into()))
    );
    assert!(!sign.contains_key("front_text"));

    let playerdata_file = output.join(format!("playerdata/{UUID}.dat"));
    let Value::Compound(player) = read_nbt_file(&playerdata_file).unwrap() else {
        panic!("player file is not a compound");
    };
    assert_eq!(player.get("DataVersion"), Some(&Value::Int(2975)));
    let Some(Value::List(inventory)) = player.get("Inventory") else {
        panic!("missing Inventory");
    };
    let Value::Compound(apple) = &inventory[0] else {
        panic!()
    };
    assert_eq!(apple.get("Count"), Some(&Value::Byte(3)));
    assert!(!apple.contains_key("count"));

    let Value::Compound(shulker) = &inventory[1] else {
        panic!()
    };
    assert_eq!(shulker.get("Count"), Some(&Value::Byte(1)));
    assert!(!shulker.contains_key("components"));
    let Some(Value::Compound(shulker_tag)) = shulker.get("tag") else {
        panic!("missing shulker tag");
    };
    let Some(Value::Compound(bet)) = shulker_tag.get("BlockEntityTag") else {
        panic!("missing BlockEntityTag");
    };
    let Some(Value::List(shulker_items)) = bet.get("Items") else {
        panic!("missing shulker Items");
    };
    let Value::Compound(boots) = &shulker_items[0] else {
        panic!()
    };
    assert_eq!(boots.get("Count"), Some(&Value::Byte(1)));
    assert_eq!(boots.get("Slot"), Some(&Value::Byte(0)));
    let Some(Value::Compound(boots_tag)) = boots.get("tag") else {
        panic!("missing boots tag");
    };
    let Some(Value::List(enchants)) = boots_tag.get("Enchantments") else {
        panic!("missing Enchantments");
    };
    let Value::Compound(enchant) = &enchants[0] else {
        panic!()
    };
    assert_eq!(
        enchant.get("id"),
        Some(&Value::String("minecraft:feather_falling".into()))
    );
    assert_eq!(enchant.get("lvl"), Some(&Value::Short(7)));

    let data = read_level_data(&output.join("level.dat"));
    assert_eq!(data.get("DataVersion"), Some(&Value::Int(2975)));
    let Some(Value::Compound(embedded)) = data.get("Player") else {
        panic!("missing Player")
    };
    let Some(Value::List(embedded_inv)) = embedded.get("Inventory") else {
        panic!()
    };
    let Value::Compound(embedded_apple) = &embedded_inv[0] else {
        panic!()
    };
    assert_eq!(embedded_apple.get("Count"), Some(&Value::Byte(3)));

    fs::remove_dir_all(&base).ok();
}

#[test]
fn nonempty_output_rejected() {
    let base = std::env::temp_dir().join(format!("mcconvert-e2e-out-{}", std::process::id()));
    fs::remove_dir_all(&base).ok();
    let input = base.join("world");
    build_synthetic_world(&input);
    let output: PathBuf = base.join("occupied");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("existing_file.txt"), b"x").unwrap();

    let result = convert(&ConvertOptions {
        input,
        output,
        target: "1.21.11".into(),
        block_table: None,
    });
    assert!(result.is_err());

    fs::remove_dir_all(&base).ok();
}
