use std::collections::HashMap;

use fastnbt::Value;

use crate::report::Report;

const DV_PAINTING_FIELDS: i32 = 3105;
const DV_SIGN_TEXT: i32 = 3463;
const DV_EFFECT_FIELDS: i32 = 3578;
const DV_ITEM_COMPONENTS: i32 = 3837;
const DV_ATTRIBUTE_IDS: i32 = 4080;
const DV_FURNACE_FIELDS: i32 = 4189;
const DV_EQUIPMENT: i32 = 4325;

pub fn downgrade_compound(
    map: &mut HashMap<String, Value>,
    target_data_version: i32,
    report: &mut Report,
) {
    if target_data_version >= DV_EQUIPMENT {
        return;
    }
    apply(map, target_data_version, report);
}

fn apply(map: &mut HashMap<String, Value>, dv: i32, report: &mut Report) {
    if dv < DV_EQUIPMENT {
        split_equipment(map);
        rename_fall_distance(map);
        normalize_custom_name(map, report);
        normalize_sign_text(map);
        convert_respawn(map);
        convert_hanging_entity(map);
    }
    if dv < DV_FURNACE_FIELDS {
        rename_furnace_fields(map);
    }
    if dv < DV_PAINTING_FIELDS {
        convert_painting(map, report);
    }
    if (DV_ITEM_COMPONENTS..DV_EQUIPMENT).contains(&dv) {
        normalize_component_text(map);
    }
    if dv < DV_ITEM_COMPONENTS && is_item_stack(map) {
        convert_item_stack(map, dv, report);
    }
    downgrade_attributes(map, dv, report);
    if dv < DV_EFFECT_FIELDS {
        convert_active_effects(map, report);
    }
    if dv < DV_SIGN_TEXT && map.contains_key("front_text") {
        convert_sign(map, report);
    }
    if dv < DV_ITEM_COMPONENTS && !is_item_stack(map) && map.remove("components").is_some() {
        report.dropped_data += 1;
        report.warn("Some component data was dropped (no equivalent before 1.20.5).");
    }
    for value in map.values_mut() {
        walk(value, dv, report);
    }
}

fn walk(value: &mut Value, dv: i32, report: &mut Report) {
    match value {
        Value::Compound(map) => apply(map, dv, report),
        Value::List(items) => {
            for item in items {
                walk(item, dv, report);
            }
        }
        _ => {}
    }
}

fn is_item_stack(map: &HashMap<String, Value>) -> bool {
    matches!(map.get("id"), Some(Value::String(_)))
        && matches!(map.get("count"), Some(Value::Int(_)))
}

fn convert_item_stack(map: &mut HashMap<String, Value>, dv: i32, report: &mut Report) {
    let count = match map.remove("count") {
        Some(Value::Int(count)) => count,
        _ => 1,
    };
    map.insert("Count".into(), Value::Byte(count.clamp(0, 127) as i8));

    let mut tag: HashMap<String, Value> = HashMap::new();
    if let Some(Value::Compound(components)) = map.remove("components") {
        for (key, value) in components {
            match key.as_str() {
                "minecraft:custom_data" => {
                    if let Value::Compound(custom) = value {
                        for (k, v) in custom {
                            tag.insert(k, v);
                        }
                    }
                }
                "minecraft:damage" => {
                    tag.insert("Damage".into(), value);
                }
                "minecraft:repair_cost" => {
                    tag.insert("RepairCost".into(), value);
                }
                "minecraft:unbreakable" => {
                    tag.insert("Unbreakable".into(), Value::Byte(1));
                }
                "minecraft:enchantments" => {
                    if let Some(list) = convert_enchantments(value) {
                        tag.insert("Enchantments".into(), list);
                    }
                }
                "minecraft:stored_enchantments" => {
                    if let Some(list) = convert_enchantments(value) {
                        tag.insert("StoredEnchantments".into(), list);
                    }
                }
                "minecraft:custom_name" => {
                    if let Some(json) = text_to_json_string(&value) {
                        display_tag(&mut tag).insert("Name".into(), Value::String(json));
                    }
                }
                "minecraft:lore" => {
                    if let Value::List(lines) = value {
                        let converted: Vec<Value> = lines
                            .iter()
                            .filter_map(text_to_json_string)
                            .map(Value::String)
                            .collect();
                        display_tag(&mut tag).insert("Lore".into(), Value::List(converted));
                    }
                }
                "minecraft:container" => {
                    let items = convert_container(value, dv, report);
                    block_entity_tag(&mut tag).insert("Items".into(), items);
                }
                "minecraft:block_entity_data" => {
                    if let Value::Compound(data) = value {
                        let bet = block_entity_tag(&mut tag);
                        for (k, v) in data {
                            bet.insert(k, v);
                        }
                    }
                }
                "minecraft:entity_data" => {
                    if matches!(value, Value::Compound(_)) {
                        tag.insert("EntityTag".into(), value);
                    }
                }
                "minecraft:bucket_entity_data" => {
                    if let Value::Compound(data) = value {
                        for (k, v) in data {
                            tag.insert(k, v);
                        }
                    }
                }
                "minecraft:custom_model_data" => match value {
                    Value::Int(_) => {
                        tag.insert("CustomModelData".into(), value);
                    }
                    _ => {
                        report.dropped_data += 1;
                        report.warn(
                            "custom_model_data in a non-integer format was dropped (no pre-1.21.4 equivalent).",
                        );
                    }
                },
                "minecraft:dyed_color" => {
                    let color = match &value {
                        Value::Int(c) => Some(*c),
                        Value::Compound(m) => match m.get("rgb") {
                            Some(Value::Int(c)) => Some(*c),
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(color) = color {
                        display_tag(&mut tag).insert("color".into(), Value::Int(color));
                    }
                }
                "minecraft:lodestone_tracker" => {
                    if let Value::Compound(mut tracker) = value {
                        let tracked = match tracker.remove("tracked") {
                            Some(Value::Byte(tracked)) => tracked,
                            _ => 1,
                        };
                        tag.insert("LodestoneTracked".into(), Value::Byte(tracked));
                        if let Some(Value::Compound(mut target)) = tracker.remove("target") {
                            if let Some(dimension @ Value::String(_)) = target.remove("dimension") {
                                tag.insert("LodestoneDimension".into(), dimension);
                            }
                            if let Some(Value::IntArray(pos)) = target.remove("pos") {
                                let pos: Vec<i32> = pos.iter().copied().collect();
                                if pos.len() == 3 {
                                    let mut lodestone_pos = HashMap::new();
                                    lodestone_pos.insert("X".to_string(), Value::Int(pos[0]));
                                    lodestone_pos.insert("Y".to_string(), Value::Int(pos[1]));
                                    lodestone_pos.insert("Z".to_string(), Value::Int(pos[2]));
                                    tag.insert(
                                        "LodestonePos".into(),
                                        Value::Compound(lodestone_pos),
                                    );
                                }
                            }
                        }
                    }
                }
                "minecraft:map_id" => {
                    if matches!(value, Value::Int(_)) {
                        tag.insert("map".into(), value);
                    }
                }
                "minecraft:potion_contents" => convert_potion(value, &mut tag, report),
                "minecraft:profile" => convert_profile(value, &mut tag),
                "minecraft:writable_book_content" => {
                    if let Value::Compound(book) = value {
                        if let Some(Value::List(pages)) = book.get("pages") {
                            let converted: Vec<Value> = pages
                                .iter()
                                .filter_map(page_raw)
                                .filter_map(|raw| match raw {
                                    Value::String(s) => Some(Value::String(s.clone())),
                                    _ => None,
                                })
                                .collect();
                            tag.insert("pages".into(), Value::List(converted));
                        }
                    }
                }
                "minecraft:written_book_content" => {
                    if let Value::Compound(mut book) = value {
                        if let Some(title) = book.remove("title") {
                            let raw = match &title {
                                Value::Compound(m) => m.get("raw").cloned(),
                                other => Some(other.clone()),
                            };
                            if let Some(Value::String(t)) = raw {
                                tag.insert("title".into(), Value::String(t));
                            }
                        }
                        if let Some(author @ Value::String(_)) = book.remove("author") {
                            tag.insert("author".into(), author);
                        }
                        if let Some(generation @ Value::Int(_)) = book.remove("generation") {
                            tag.insert("generation".into(), generation);
                        }
                        if let Some(Value::List(pages)) = book.remove("pages") {
                            let converted: Vec<Value> = pages
                                .iter()
                                .filter_map(page_raw)
                                .filter_map(text_to_json_string)
                                .map(Value::String)
                                .collect();
                            tag.insert("pages".into(), Value::List(converted));
                        }
                    }
                }
                "minecraft:charged_projectiles" => {
                    if let Value::List(projectiles) = value {
                        let converted: Vec<Value> = projectiles
                            .into_iter()
                            .filter_map(|projectile| {
                                let Value::Compound(mut item) = projectile else {
                                    return None;
                                };
                                convert_item_stack(&mut item, dv, report);
                                Some(Value::Compound(item))
                            })
                            .collect();
                        if !converted.is_empty() {
                            tag.insert("Charged".into(), Value::Byte(1));
                        }
                        tag.insert("ChargedProjectiles".into(), Value::List(converted));
                    }
                }
                other => {
                    report.dropped_data += 1;
                    report.warn(format!(
                        "Item component '{other}' has no equivalent before 1.20.5 and was dropped."
                    ));
                }
            }
        }
    }
    if !tag.is_empty() {
        map.insert("tag".into(), Value::Compound(tag));
    }
    report.items_converted += 1;
}

fn display_tag(tag: &mut HashMap<String, Value>) -> &mut HashMap<String, Value> {
    let entry = tag
        .entry("display".to_string())
        .or_insert_with(|| Value::Compound(HashMap::new()));
    let Value::Compound(display) = entry else {
        unreachable!()
    };
    display
}

fn block_entity_tag(tag: &mut HashMap<String, Value>) -> &mut HashMap<String, Value> {
    let entry = tag
        .entry("BlockEntityTag".to_string())
        .or_insert_with(|| Value::Compound(HashMap::new()));
    let Value::Compound(bet) = entry else {
        unreachable!()
    };
    bet
}

fn page_raw(entry: &Value) -> Option<&Value> {
    match entry {
        Value::Compound(map) => map.get("raw"),
        other => Some(other),
    }
}

fn convert_potion(value: Value, tag: &mut HashMap<String, Value>, report: &mut Report) {
    match value {
        Value::String(potion) => {
            tag.insert("Potion".into(), Value::String(potion));
        }
        Value::Compound(mut contents) => {
            if let Some(potion @ Value::String(_)) = contents.remove("potion") {
                tag.insert("Potion".into(), potion);
            }
            if let Some(color @ Value::Int(_)) = contents.remove("custom_color") {
                tag.insert("CustomPotionColor".into(), color);
            }
            if contents.remove("custom_effects").is_some() {
                report.dropped_data += 1;
                report.warn(
                    "Custom potion effects were dropped (incompatible format before 1.20.2).",
                );
            }
        }
        _ => {}
    }
}

fn convert_profile(value: Value, tag: &mut HashMap<String, Value>) {
    let mut owner = HashMap::new();
    match value {
        Value::String(name) => {
            owner.insert("Name".to_string(), Value::String(name));
        }
        Value::Compound(mut profile) => {
            if let Some(name @ Value::String(_)) = profile.remove("name") {
                owner.insert("Name".to_string(), name);
            }
            if let Some(id @ Value::IntArray(_)) = profile.remove("id") {
                owner.insert("Id".to_string(), id);
            }
            if let Some(Value::List(properties)) = profile.remove("properties") {
                let mut textures = Vec::new();
                for property in properties {
                    let Value::Compound(mut property) = property else {
                        continue;
                    };
                    if !matches!(property.get("name"), Some(Value::String(n)) if n == "textures") {
                        continue;
                    }
                    let mut texture = HashMap::new();
                    if let Some(v @ Value::String(_)) = property.remove("value") {
                        texture.insert("Value".to_string(), v);
                    }
                    if let Some(s @ Value::String(_)) = property.remove("signature") {
                        texture.insert("Signature".to_string(), s);
                    }
                    textures.push(Value::Compound(texture));
                }
                if !textures.is_empty() {
                    let mut props = HashMap::new();
                    props.insert("textures".to_string(), Value::List(textures));
                    owner.insert("Properties".to_string(), Value::Compound(props));
                }
            }
        }
        _ => {}
    }
    if !owner.is_empty() {
        tag.insert("SkullOwner".into(), Value::Compound(owner));
    }
}

fn convert_enchantments(value: Value) -> Option<Value> {
    let Value::Compound(map) = value else {
        return None;
    };
    let levels = match map.get("levels") {
        Some(Value::Compound(levels)) => levels.clone(),
        _ => map,
    };
    let list: Vec<Value> = levels
        .into_iter()
        .filter_map(|(id, level)| {
            let level = match level {
                Value::Int(v) => v as i16,
                Value::Short(v) => v,
                Value::Byte(v) => v as i16,
                _ => return None,
            };
            let mut entry = HashMap::new();
            entry.insert("id".to_string(), Value::String(id));
            entry.insert("lvl".to_string(), Value::Short(level));
            Some(Value::Compound(entry))
        })
        .collect();
    Some(Value::List(list))
}

fn convert_container(value: Value, dv: i32, report: &mut Report) -> Value {
    let Value::List(entries) = value else {
        return Value::List(Vec::new());
    };
    let mut items = Vec::new();
    for entry in entries {
        let Value::Compound(mut entry) = entry else {
            continue;
        };
        let slot = match entry.remove("slot") {
            Some(Value::Int(s)) => s,
            _ => 0,
        };
        let Some(Value::Compound(mut item)) = entry.remove("item") else {
            continue;
        };
        convert_item_stack(&mut item, dv, report);
        item.insert("Slot".into(), Value::Byte(slot.clamp(0, 127) as i8));
        items.push(Value::Compound(item));
    }
    Value::List(items)
}

fn split_equipment(map: &mut HashMap<String, Value>) {
    let Some(Value::Compound(mut equipment)) = map.remove("equipment") else {
        return;
    };
    let mut take = |slot: &str| {
        equipment
            .remove(slot)
            .unwrap_or(Value::Compound(HashMap::new()))
    };
    let hand_items = vec![take("mainhand"), take("offhand")];
    let armor_items = vec![take("feet"), take("legs"), take("chest"), take("head")];
    map.insert("HandItems".into(), Value::List(hand_items));
    map.insert("ArmorItems".into(), Value::List(armor_items));
    if let Some(body) = equipment.remove("body") {
        map.insert("body_armor_item".into(), body);
    }
    if let Some(saddle) = equipment.remove("saddle") {
        map.insert("SaddleItem".into(), saddle);
    }
}

fn rename_fall_distance(map: &mut HashMap<String, Value>) {
    if let Some(value) = map.remove("fall_distance") {
        let fall = match value {
            Value::Double(d) => d as f32,
            Value::Float(f) => f,
            _ => 0.0,
        };
        map.insert("FallDistance".into(), Value::Float(fall));
    }
}

fn normalize_custom_name(map: &mut HashMap<String, Value>, report: &mut Report) {
    let Some(value) = map.get("CustomName") else {
        return;
    };
    match text_to_json_string(value) {
        Some(json) => {
            map.insert("CustomName".into(), Value::String(json));
        }
        None => {
            map.remove("CustomName");
            report.dropped_data += 1;
            report.warn("A custom name could not be converted to the pre-1.21.5 format.");
        }
    }
}

const HANGING_ENTITIES: &[&str] = &[
    "minecraft:item_frame",
    "minecraft:glow_item_frame",
    "minecraft:painting",
    "minecraft:leash_knot",
];

fn convert_hanging_entity(map: &mut HashMap<String, Value>) {
    let is_hanging = matches!(
        map.get("id"),
        Some(Value::String(id)) if HANGING_ENTITIES.contains(&id.as_str())
    );
    if !is_hanging {
        return;
    }
    let Some(Value::IntArray(pos)) = map.remove("block_pos") else {
        return;
    };
    let pos: Vec<i32> = pos.iter().copied().collect();
    if pos.len() == 3 {
        map.insert("TileX".into(), Value::Int(pos[0]));
        map.insert("TileY".into(), Value::Int(pos[1]));
        map.insert("TileZ".into(), Value::Int(pos[2]));
    }
}

const LEGACY_PAINTINGS: &[&str] = &[
    "minecraft:kebab",
    "minecraft:aztec",
    "minecraft:alban",
    "minecraft:aztec2",
    "minecraft:bomb",
    "minecraft:plant",
    "minecraft:wasteland",
    "minecraft:pool",
    "minecraft:courbet",
    "minecraft:sea",
    "minecraft:sunset",
    "minecraft:creebet",
    "minecraft:wanderer",
    "minecraft:graham",
    "minecraft:match",
    "minecraft:bust",
    "minecraft:stage",
    "minecraft:void",
    "minecraft:skull_and_roses",
    "minecraft:wither",
    "minecraft:fighters",
    "minecraft:pointer",
    "minecraft:pigscene",
    "minecraft:burning_skull",
    "minecraft:skeleton",
    "minecraft:donkey_kong",
];

fn convert_painting(map: &mut HashMap<String, Value>, report: &mut Report) {
    if !matches!(map.get("id"), Some(Value::String(id)) if id == "minecraft:painting") {
        return;
    }
    if let Some(facing) = map.remove("facing") {
        map.insert("Facing".into(), facing);
    }
    if let Some(variant @ Value::String(_)) = map.remove("variant") {
        if let Value::String(name) = &variant {
            if !LEGACY_PAINTINGS.contains(&name.as_str()) {
                report.dropped_data += 1;
                report.warn(format!(
                    "Painting variant '{name}' does not exist before 1.19; the painting will fall back to the default artwork."
                ));
            }
        }
        map.insert("Motive".into(), variant);
    }
}

const FURNACE_KEY_MAP: &[(&str, &str)] = &[
    ("lit_time_remaining", "BurnTime"),
    ("cooking_time_spent", "CookTime"),
    ("cooking_total_time", "CookTimeTotal"),
];

fn rename_furnace_fields(map: &mut HashMap<String, Value>) {
    if !map.contains_key("lit_time_remaining") && !map.contains_key("cooking_time_spent") {
        return;
    }
    for (new, legacy) in FURNACE_KEY_MAP {
        if let Some(value) = map.remove(*new) {
            map.insert((*legacy).to_string(), value);
        }
    }
    map.remove("lit_total_time");
}

const EFFECT_NUMERIC_IDS: &[(&str, i8)] = &[
    ("minecraft:speed", 1),
    ("minecraft:slowness", 2),
    ("minecraft:haste", 3),
    ("minecraft:mining_fatigue", 4),
    ("minecraft:strength", 5),
    ("minecraft:instant_health", 6),
    ("minecraft:instant_damage", 7),
    ("minecraft:jump_boost", 8),
    ("minecraft:nausea", 9),
    ("minecraft:regeneration", 10),
    ("minecraft:resistance", 11),
    ("minecraft:fire_resistance", 12),
    ("minecraft:water_breathing", 13),
    ("minecraft:invisibility", 14),
    ("minecraft:blindness", 15),
    ("minecraft:night_vision", 16),
    ("minecraft:hunger", 17),
    ("minecraft:weakness", 18),
    ("minecraft:poison", 19),
    ("minecraft:wither", 20),
    ("minecraft:health_boost", 21),
    ("minecraft:absorption", 22),
    ("minecraft:saturation", 23),
    ("minecraft:glowing", 24),
    ("minecraft:levitation", 25),
    ("minecraft:luck", 26),
    ("minecraft:unluck", 27),
    ("minecraft:slow_falling", 28),
    ("minecraft:conduit_power", 29),
    ("minecraft:dolphins_grace", 30),
    ("minecraft:bad_omen", 31),
    ("minecraft:hero_of_the_village", 32),
    ("minecraft:darkness", 33),
];

fn convert_active_effects(map: &mut HashMap<String, Value>, report: &mut Report) {
    let Some(Value::List(effects)) = map.remove("active_effects") else {
        return;
    };
    let mut converted = Vec::new();
    for effect in effects {
        let Value::Compound(mut effect) = effect else {
            continue;
        };
        let id = match effect.remove("id") {
            Some(Value::String(id)) => id,
            _ => continue,
        };
        let Some((_, numeric)) = EFFECT_NUMERIC_IDS.iter().find(|(name, _)| *name == id) else {
            report.dropped_data += 1;
            report.warn(format!(
                "Status effect '{id}' has no numeric id before 1.20.2 and was dropped."
            ));
            continue;
        };
        let mut legacy = HashMap::new();
        legacy.insert("Id".to_string(), Value::Byte(*numeric));
        let amplifier = match effect.remove("amplifier") {
            Some(Value::Byte(a)) => a,
            Some(Value::Int(a)) => a.clamp(0, 127) as i8,
            _ => 0,
        };
        legacy.insert("Amplifier".to_string(), Value::Byte(amplifier));
        let duration = match effect.remove("duration") {
            Some(Value::Int(d)) => d,
            _ => 0,
        };
        legacy.insert("Duration".to_string(), Value::Int(duration));
        let byte_of = |value: Option<Value>, default: i8| match value {
            Some(Value::Byte(b)) => b,
            _ => default,
        };
        legacy.insert(
            "Ambient".to_string(),
            Value::Byte(byte_of(effect.remove("ambient"), 0)),
        );
        let show_particles = byte_of(effect.remove("show_particles"), 1);
        legacy.insert("ShowParticles".to_string(), Value::Byte(show_particles));
        legacy.insert(
            "ShowIcon".to_string(),
            Value::Byte(byte_of(effect.remove("show_icon"), show_particles)),
        );
        converted.push(Value::Compound(legacy));
    }
    if !converted.is_empty() {
        map.insert("ActiveEffects".into(), Value::List(converted));
    }
}

fn convert_respawn(map: &mut HashMap<String, Value>) {
    let Some(Value::Compound(respawn)) = map.get("respawn") else {
        return;
    };
    if !respawn.contains_key("pos") || !respawn.contains_key("dimension") {
        return;
    }
    let Some(Value::Compound(mut respawn)) = map.remove("respawn") else {
        return;
    };
    if let Some(Value::IntArray(pos)) = respawn.remove("pos") {
        let pos: Vec<i32> = pos.iter().copied().collect();
        if pos.len() == 3 {
            map.insert("SpawnX".into(), Value::Int(pos[0]));
            map.insert("SpawnY".into(), Value::Int(pos[1]));
            map.insert("SpawnZ".into(), Value::Int(pos[2]));
        }
    }
    if let Some(dimension @ Value::String(_)) = respawn.remove("dimension") {
        map.insert("SpawnDimension".into(), dimension);
    }
    let yaw = match respawn.remove("yaw") {
        Some(Value::Float(yaw)) => yaw,
        _ => 0.0,
    };
    map.insert("SpawnAngle".into(), Value::Float(yaw));
    let forced = matches!(respawn.remove("forced"), Some(Value::Byte(1)));
    map.insert("SpawnForced".into(), Value::Byte(forced as i8));
}

fn normalize_component_text(map: &mut HashMap<String, Value>) {
    let Some(Value::Compound(components)) = map.get_mut("components") else {
        return;
    };
    for key in ["minecraft:custom_name", "minecraft:item_name"] {
        if let Some(value) = components.get(key) {
            if let Some(json) = text_to_json_string(value) {
                components.insert(key.to_string(), Value::String(json));
            }
        }
    }
    if let Some(Value::List(lore)) = components.get_mut("minecraft:lore") {
        for line in lore.iter_mut() {
            if let Some(json) = text_to_json_string(line) {
                *line = Value::String(json);
            }
        }
    }
    if let Some(Value::Compound(book)) = components.get_mut("minecraft:written_book_content") {
        if let Some(Value::List(pages)) = book.get_mut("pages") {
            for page in pages.iter_mut() {
                match page {
                    Value::Compound(entry) => {
                        if let Some(raw) = entry.get("raw") {
                            if let Some(json) = text_to_json_string(raw) {
                                entry.insert("raw".to_string(), Value::String(json));
                            }
                        }
                    }
                    other => {
                        if let Some(json) = text_to_json_string(other) {
                            *other = Value::String(json);
                        }
                    }
                }
            }
        }
    }
}

fn normalize_sign_text(map: &mut HashMap<String, Value>) {
    for side in ["front_text", "back_text"] {
        let Some(Value::Compound(text)) = map.get_mut(side) else {
            continue;
        };
        for key in ["messages", "filtered_messages"] {
            let Some(Value::List(messages)) = text.get_mut(key) else {
                continue;
            };
            for message in messages.iter_mut() {
                if let Some(json) = text_to_json_string(message) {
                    *message = Value::String(json);
                }
            }
        }
    }
}

fn text_to_json_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => {
            if s.starts_with('{') || s.starts_with('"') || s.starts_with('[') {
                Some(s.clone())
            } else {
                Some(serde_json::json!({ "text": s }).to_string())
            }
        }
        Value::Compound(map) => {
            let text = match map.get("text") {
                Some(Value::String(t)) => t.clone(),
                _ => String::new(),
            };
            Some(serde_json::json!({ "text": text }).to_string())
        }
        _ => None,
    }
}

const PLAYER_ATTRIBUTES: &[&str] = &[
    "block_break_speed",
    "block_interaction_range",
    "entity_interaction_range",
    "mining_efficiency",
    "sneaking_speed",
    "submerged_mining_speed",
    "sweeping_damage_ratio",
];

fn legacy_attribute_id(id: &str) -> String {
    let name = id.strip_prefix("minecraft:").unwrap_or(id);
    if name.contains('.') {
        return id.to_string();
    }
    if PLAYER_ATTRIBUTES.contains(&name) {
        format!("minecraft:player.{name}")
    } else if name == "spawn_reinforcements" {
        format!("minecraft:zombie.{name}")
    } else {
        format!("minecraft:generic.{name}")
    }
}

fn downgrade_attributes(map: &mut HashMap<String, Value>, dv: i32, report: &mut Report) {
    if dv >= DV_ATTRIBUTE_IDS {
        return;
    }
    let Some(Value::List(entries)) = map.get("attributes") else {
        return;
    };
    let looks_like_attributes = entries.iter().all(|entry| {
        matches!(entry, Value::Compound(a) if a.contains_key("base") || a.contains_key("id"))
    });
    if !looks_like_attributes {
        return;
    }
    let Some(Value::List(mut entries)) = map.remove("attributes") else {
        return;
    };
    let mut dropped_modifiers = false;
    for entry in entries.iter_mut() {
        let Value::Compound(attribute) = entry else {
            continue;
        };
        if let Some(Value::String(id)) = attribute.get("id") {
            let legacy = legacy_attribute_id(id);
            attribute.insert("id".into(), Value::String(legacy));
        }
        if dv < DV_ITEM_COMPONENTS {
            if let Some(id) = attribute.remove("id") {
                attribute.insert("Name".into(), id);
            }
            if let Some(base) = attribute.remove("base") {
                attribute.insert("Base".into(), base);
            }
            if attribute.remove("modifiers").is_some() {
                dropped_modifiers = true;
            }
        }
    }
    if dropped_modifiers {
        report.dropped_data += 1;
        report.warn("Attribute modifiers were dropped (incompatible format before 1.20.5).");
    }
    let key = if dv < DV_ITEM_COMPONENTS {
        "Attributes"
    } else {
        "attributes"
    };
    map.insert(key.into(), Value::List(entries));
}

fn convert_sign(map: &mut HashMap<String, Value>, report: &mut Report) {
    if let Some(Value::Compound(front)) = map.remove("front_text") {
        let empty = "{\"text\":\"\"}".to_string();
        if let Some(Value::List(messages)) = front.get("messages") {
            for i in 0..4 {
                let json = messages
                    .get(i)
                    .and_then(text_to_json_string)
                    .unwrap_or_else(|| empty.clone());
                map.insert(format!("Text{}", i + 1), Value::String(json));
            }
        }
        if let Some(color @ Value::String(_)) = front.get("color") {
            map.insert("Color".into(), color.clone());
        }
        if let Some(Value::Byte(glowing)) = front.get("has_glowing_text") {
            map.insert("GlowingText".into(), Value::Byte(*glowing));
        }
    }
    if map.remove("back_text").is_some() {
        report.dropped_data += 1;
        report.warn("Sign back-side text was dropped (not supported before 1.20).");
    }
    map.remove("is_waxed");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compound(entries: Vec<(&str, Value)>) -> HashMap<String, Value> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    #[test]
    fn item_stack_to_legacy_format() {
        let mut enchant_levels = HashMap::new();
        enchant_levels.insert("minecraft:sharpness".to_string(), Value::Int(10));
        let mut enchantments = HashMap::new();
        enchantments.insert("levels".to_string(), Value::Compound(enchant_levels));

        let mut item = compound(vec![
            ("id", Value::String("minecraft:diamond_sword".into())),
            ("count", Value::Int(1)),
            (
                "components",
                Value::Compound(compound(vec![
                    ("minecraft:damage", Value::Int(5)),
                    ("minecraft:enchantments", Value::Compound(enchantments)),
                    ("minecraft:custom_name", Value::String("Slayer".into())),
                ])),
            ),
        ]);

        let mut report = Report::default();
        downgrade_compound(&mut item, 2975, &mut report);

        assert_eq!(item.get("Count"), Some(&Value::Byte(1)));
        assert!(!item.contains_key("count"));
        assert!(!item.contains_key("components"));
        let Some(Value::Compound(tag)) = item.get("tag") else {
            panic!("missing tag")
        };
        assert_eq!(tag.get("Damage"), Some(&Value::Int(5)));
        let Some(Value::List(enchants)) = tag.get("Enchantments") else {
            panic!("missing Enchantments")
        };
        let Value::Compound(first) = &enchants[0] else {
            panic!()
        };
        assert_eq!(
            first.get("id"),
            Some(&Value::String("minecraft:sharpness".into()))
        );
        assert_eq!(first.get("lvl"), Some(&Value::Short(10)));
        let Some(Value::Compound(display)) = tag.get("display") else {
            panic!()
        };
        assert_eq!(
            display.get("Name"),
            Some(&Value::String("{\"text\":\"Slayer\"}".into()))
        );
        assert_eq!(report.items_converted, 1);
    }

    #[test]
    fn nested_container_items() {
        let plain_item = compound(vec![("id", Value::String("minecraft:stone".into()))]);
        let mut plain_entry = HashMap::new();
        plain_entry.insert("slot".to_string(), Value::Int(0));
        plain_entry.insert("item".to_string(), Value::Compound(plain_item));

        let mut direct_enchants = HashMap::new();
        direct_enchants.insert("minecraft:feather_falling".to_string(), Value::Int(3));
        let enchanted_item = compound(vec![
            ("id", Value::String("minecraft:diamond_boots".into())),
            (
                "components",
                Value::Compound(compound(vec![
                    ("minecraft:enchantments", Value::Compound(direct_enchants)),
                    ("minecraft:repair_cost", Value::Int(1)),
                ])),
            ),
        ]);
        let mut enchanted_entry = HashMap::new();
        enchanted_entry.insert("slot".to_string(), Value::Int(3));
        enchanted_entry.insert("item".to_string(), Value::Compound(enchanted_item));

        let mut shulker = compound(vec![
            ("id", Value::String("minecraft:shulker_box".into())),
            ("count", Value::Int(1)),
            (
                "components",
                Value::Compound(compound(vec![(
                    "minecraft:container",
                    Value::List(vec![
                        Value::Compound(plain_entry),
                        Value::Compound(enchanted_entry),
                    ]),
                )])),
            ),
        ]);

        let mut report = Report::default();
        downgrade_compound(&mut shulker, 3465, &mut report);

        let Some(Value::Compound(tag)) = shulker.get("tag") else {
            panic!()
        };
        let Some(Value::Compound(bet)) = tag.get("BlockEntityTag") else {
            panic!()
        };
        let Some(Value::List(items)) = bet.get("Items") else {
            panic!()
        };

        let Value::Compound(plain) = &items[0] else {
            panic!()
        };
        assert_eq!(plain.get("Slot"), Some(&Value::Byte(0)));
        assert_eq!(plain.get("Count"), Some(&Value::Byte(1)));
        assert!(!plain.contains_key("count"));

        let Value::Compound(enchanted) = &items[1] else {
            panic!()
        };
        assert_eq!(enchanted.get("Slot"), Some(&Value::Byte(3)));
        assert_eq!(enchanted.get("Count"), Some(&Value::Byte(1)));
        let Some(Value::Compound(inner_tag)) = enchanted.get("tag") else {
            panic!()
        };
        assert_eq!(inner_tag.get("RepairCost"), Some(&Value::Int(1)));
        let Some(Value::List(enchants)) = inner_tag.get("Enchantments") else {
            panic!()
        };
        let Value::Compound(enchant) = &enchants[0] else {
            panic!()
        };
        assert_eq!(
            enchant.get("id"),
            Some(&Value::String("minecraft:feather_falling".into()))
        );
        assert_eq!(enchant.get("lvl"), Some(&Value::Short(3)));

        assert_eq!(report.items_converted, 3);
    }

    #[test]
    fn equipment_split_for_pre_1_21_5() {
        let sword = compound(vec![
            ("id", Value::String("minecraft:iron_sword".into())),
            ("count", Value::Int(1)),
        ]);
        let helmet = compound(vec![
            ("id", Value::String("minecraft:iron_helmet".into())),
            ("count", Value::Int(1)),
        ]);
        let mut entity = compound(vec![
            ("id", Value::String("minecraft:zombie".into())),
            (
                "equipment",
                Value::Compound(compound(vec![
                    ("mainhand", Value::Compound(sword)),
                    ("head", Value::Compound(helmet)),
                ])),
            ),
            ("fall_distance", Value::Double(2.5)),
        ]);

        let mut report = Report::default();
        downgrade_compound(&mut entity, 4189, &mut report);

        assert!(!entity.contains_key("equipment"));
        assert_eq!(entity.get("FallDistance"), Some(&Value::Float(2.5)));
        let Some(Value::List(hands)) = entity.get("HandItems") else {
            panic!()
        };
        assert_eq!(hands.len(), 2);
        let Value::Compound(main) = &hands[0] else {
            panic!()
        };
        assert_eq!(
            main.get("id"),
            Some(&Value::String("minecraft:iron_sword".into()))
        );
        assert_eq!(main.get("count"), Some(&Value::Int(1)));
        let Some(Value::List(armor)) = entity.get("ArmorItems") else {
            panic!()
        };
        assert_eq!(armor.len(), 4);
        let Value::Compound(head) = &armor[3] else {
            panic!()
        };
        assert_eq!(
            head.get("id"),
            Some(&Value::String("minecraft:iron_helmet".into()))
        );
        assert_eq!(report.items_converted, 0);
    }

    #[test]
    fn attribute_rename_by_target() {
        let attribute = compound(vec![
            ("id", Value::String("minecraft:max_health".into())),
            ("base", Value::Double(20.0)),
        ]);
        let mut entity = compound(vec![(
            "attributes",
            Value::List(vec![Value::Compound(attribute.clone())]),
        )]);
        let mut report = Report::default();
        downgrade_compound(&mut entity, 3953, &mut report);
        let Some(Value::List(entries)) = entity.get("attributes") else {
            panic!()
        };
        let Value::Compound(a) = &entries[0] else {
            panic!()
        };
        assert_eq!(
            a.get("id"),
            Some(&Value::String("minecraft:generic.max_health".into()))
        );
        assert_eq!(a.get("base"), Some(&Value::Double(20.0)));

        let mut entity = compound(vec![(
            "attributes",
            Value::List(vec![Value::Compound(attribute)]),
        )]);
        downgrade_compound(&mut entity, 2975, &mut report);
        assert!(!entity.contains_key("attributes"));
        let Some(Value::List(entries)) = entity.get("Attributes") else {
            panic!()
        };
        let Value::Compound(a) = &entries[0] else {
            panic!()
        };
        assert_eq!(
            a.get("Name"),
            Some(&Value::String("minecraft:generic.max_health".into()))
        );
        assert_eq!(a.get("Base"), Some(&Value::Double(20.0)));
    }

    #[test]
    fn sign_to_legacy_format() {
        let front = compound(vec![
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
        ]);
        let mut sign = compound(vec![
            ("id", Value::String("minecraft:sign".into())),
            ("front_text", Value::Compound(front)),
            ("back_text", Value::Compound(HashMap::new())),
            ("is_waxed", Value::Byte(0)),
        ]);

        let mut report = Report::default();
        downgrade_compound(&mut sign, 3337, &mut report);

        assert_eq!(
            sign.get("Text1"),
            Some(&Value::String("{\"text\":\"hello\"}".into()))
        );
        assert_eq!(sign.get("Color"), Some(&Value::String("black".into())));
        assert!(!sign.contains_key("front_text"));
        assert!(!sign.contains_key("back_text"));
        assert!(!sign.contains_key("is_waxed"));
    }

    #[test]
    fn mob_custom_name_json_normalized() {
        let mut entity = compound(vec![
            ("id", Value::String("minecraft:wolf".into())),
            ("CustomName", Value::String("Doggy".into())),
        ]);
        let mut report = Report::default();
        downgrade_compound(&mut entity, 3465, &mut report);
        assert_eq!(
            entity.get("CustomName"),
            Some(&Value::String("{\"text\":\"Doggy\"}".into()))
        );

        let mut already_json = compound(vec![(
            "CustomName",
            Value::String("{\"text\":\"Bob\"}".into()),
        )]);
        downgrade_compound(&mut already_json, 3465, &mut report);
        assert_eq!(
            already_json.get("CustomName"),
            Some(&Value::String("{\"text\":\"Bob\"}".into()))
        );

        let mut untouched = compound(vec![("CustomName", Value::String("Doggy".into()))]);
        downgrade_compound(&mut untouched, 4671, &mut report);
        assert_eq!(
            untouched.get("CustomName"),
            Some(&Value::String("Doggy".into()))
        );
    }

    #[test]
    fn sign_text_json_normalized_for_1_20_targets() {
        let front = compound(vec![(
            "messages",
            Value::List(vec![
                Value::String("hello".into()),
                Value::String("".into()),
                Value::String("".into()),
                Value::String("".into()),
            ]),
        )]);
        let mut sign = compound(vec![
            ("id", Value::String("minecraft:sign".into())),
            ("front_text", Value::Compound(front)),
        ]);

        let mut report = Report::default();
        downgrade_compound(&mut sign, 3465, &mut report);

        let Some(Value::Compound(front)) = sign.get("front_text") else {
            panic!("front_text must be kept for 1.20.1");
        };
        let Some(Value::List(messages)) = front.get("messages") else {
            panic!()
        };
        assert_eq!(messages[0], Value::String("{\"text\":\"hello\"}".into()));
        assert!(!sign.contains_key("Text1"));
    }

    #[test]
    fn item_frame_block_pos_converted() {
        let mut frame = compound(vec![
            ("id", Value::String("minecraft:item_frame".into())),
            ("Facing", Value::Byte(5)),
            (
                "block_pos",
                Value::IntArray(fastnbt::IntArray::new(vec![-2217, 111, -1887])),
            ),
        ]);
        let mut report = Report::default();
        downgrade_compound(&mut frame, 4189, &mut report);

        assert!(!frame.contains_key("block_pos"));
        assert_eq!(frame.get("TileX"), Some(&Value::Int(-2217)));
        assert_eq!(frame.get("TileY"), Some(&Value::Int(111)));
        assert_eq!(frame.get("TileZ"), Some(&Value::Int(-1887)));
        assert_eq!(frame.get("Facing"), Some(&Value::Byte(5)));

        let mut untouched = compound(vec![
            ("id", Value::String("minecraft:item_frame".into())),
            (
                "block_pos",
                Value::IntArray(fastnbt::IntArray::new(vec![1, 2, 3])),
            ),
        ]);
        downgrade_compound(&mut untouched, 4671, &mut report);
        assert!(untouched.contains_key("block_pos"));
    }

    #[test]
    fn painting_converted_for_1_18() {
        let mut painting = compound(vec![
            ("id", Value::String("minecraft:painting".into())),
            ("facing", Value::Byte(3)),
            ("variant", Value::String("minecraft:wanderer".into())),
        ]);
        let mut report = Report::default();
        downgrade_compound(&mut painting, 2975, &mut report);

        assert_eq!(painting.get("Facing"), Some(&Value::Byte(3)));
        assert_eq!(
            painting.get("Motive"),
            Some(&Value::String("minecraft:wanderer".into()))
        );
        assert!(!painting.contains_key("facing"));
        assert!(!painting.contains_key("variant"));
        assert_eq!(report.dropped_data, 0);

        let mut modern = compound(vec![
            ("id", Value::String("minecraft:painting".into())),
            ("facing", Value::Byte(3)),
            ("variant", Value::String("minecraft:meditative".into())),
        ]);
        downgrade_compound(&mut modern, 2975, &mut report);
        assert_eq!(report.dropped_data, 1);

        let mut untouched = compound(vec![
            ("id", Value::String("minecraft:painting".into())),
            ("facing", Value::Byte(3)),
            ("variant", Value::String("minecraft:wanderer".into())),
        ]);
        downgrade_compound(&mut untouched, 3337, &mut report);
        assert!(untouched.contains_key("variant"));
        assert!(untouched.contains_key("facing"));

        let mut mob = compound(vec![
            ("id", Value::String("minecraft:pig".into())),
            ("variant", Value::String("minecraft:temperate".into())),
        ]);
        downgrade_compound(&mut mob, 2975, &mut report);
        assert!(mob.contains_key("variant"));
        assert!(!mob.contains_key("Motive"));
    }

    #[test]
    fn furnace_fields_renamed() {
        let mut furnace = compound(vec![
            ("id", Value::String("minecraft:furnace".into())),
            ("lit_time_remaining", Value::Short(1347)),
            ("cooking_time_spent", Value::Short(55)),
            ("cooking_total_time", Value::Short(200)),
            ("lit_total_time", Value::Short(1600)),
        ]);
        let mut report = Report::default();
        downgrade_compound(&mut furnace, 4082, &mut report);

        assert_eq!(furnace.get("BurnTime"), Some(&Value::Short(1347)));
        assert_eq!(furnace.get("CookTime"), Some(&Value::Short(55)));
        assert_eq!(furnace.get("CookTimeTotal"), Some(&Value::Short(200)));
        assert!(!furnace.contains_key("lit_time_remaining"));
        assert!(!furnace.contains_key("lit_total_time"));

        let mut untouched = compound(vec![("lit_time_remaining", Value::Short(5))]);
        downgrade_compound(&mut untouched, 4189, &mut report);
        assert!(untouched.contains_key("lit_time_remaining"));
    }

    #[test]
    fn active_effects_converted_to_numeric_ids() {
        let effect = compound(vec![
            ("id", Value::String("minecraft:speed".into())),
            ("amplifier", Value::Byte(1)),
            ("duration", Value::Int(1200)),
        ]);
        let unknown = compound(vec![
            ("id", Value::String("minecraft:weaving".into())),
            ("duration", Value::Int(100)),
        ]);
        let mut player = compound(vec![(
            "active_effects",
            Value::List(vec![
                Value::Compound(effect.clone()),
                Value::Compound(unknown),
            ]),
        )]);
        let mut report = Report::default();
        downgrade_compound(&mut player, 2975, &mut report);

        assert!(!player.contains_key("active_effects"));
        let Some(Value::List(effects)) = player.get("ActiveEffects") else {
            panic!("missing ActiveEffects");
        };
        assert_eq!(effects.len(), 1);
        let Value::Compound(speed) = &effects[0] else {
            panic!()
        };
        assert_eq!(speed.get("Id"), Some(&Value::Byte(1)));
        assert_eq!(speed.get("Amplifier"), Some(&Value::Byte(1)));
        assert_eq!(speed.get("Duration"), Some(&Value::Int(1200)));
        assert_eq!(speed.get("ShowParticles"), Some(&Value::Byte(1)));
        assert_eq!(report.dropped_data, 1);

        let mut kept = compound(vec![(
            "active_effects",
            Value::List(vec![Value::Compound(effect)]),
        )]);
        downgrade_compound(&mut kept, 3839, &mut report);
        assert!(kept.contains_key("active_effects"));
        assert!(!kept.contains_key("ActiveEffects"));
    }

    #[test]
    fn respawn_converted_to_legacy_spawn_fields() {
        let respawn = compound(vec![
            (
                "pos",
                Value::IntArray(fastnbt::IntArray::new(vec![6, 72, 4])),
            ),
            ("dimension", Value::String("minecraft:overworld".into())),
            ("yaw", Value::Float(3.66)),
            ("pitch", Value::Float(60.64)),
        ]);
        let mut player = compound(vec![
            ("Health", Value::Float(20.0)),
            ("respawn", Value::Compound(respawn.clone())),
        ]);
        let mut report = Report::default();
        downgrade_compound(&mut player, 3465, &mut report);

        assert!(!player.contains_key("respawn"));
        assert_eq!(player.get("SpawnX"), Some(&Value::Int(6)));
        assert_eq!(player.get("SpawnY"), Some(&Value::Int(72)));
        assert_eq!(player.get("SpawnZ"), Some(&Value::Int(4)));
        assert_eq!(player.get("SpawnAngle"), Some(&Value::Float(3.66)));
        assert_eq!(
            player.get("SpawnDimension"),
            Some(&Value::String("minecraft:overworld".into()))
        );
        assert_eq!(player.get("SpawnForced"), Some(&Value::Byte(0)));

        let mut untouched = compound(vec![("respawn", Value::Compound(respawn))]);
        downgrade_compound(&mut untouched, 4671, &mut report);
        assert!(untouched.contains_key("respawn"));
        assert!(!untouched.contains_key("SpawnX"));
    }

    #[test]
    fn component_text_json_normalized_for_1_20_5_to_1_21_4() {
        let mut item = compound(vec![
            ("id", Value::String("minecraft:iron_pickaxe".into())),
            ("count", Value::Int(1)),
            (
                "components",
                Value::Compound(compound(vec![
                    ("minecraft:custom_name", Value::String("MightyPick".into())),
                    (
                        "minecraft:lore",
                        Value::List(vec![Value::String("Lore line".into())]),
                    ),
                ])),
            ),
        ]);
        let mut report = Report::default();
        downgrade_compound(&mut item, 4189, &mut report);

        assert_eq!(item.get("count"), Some(&Value::Int(1)));
        let Some(Value::Compound(components)) = item.get("components") else {
            panic!("components must be kept for 1.21.4");
        };
        assert_eq!(
            components.get("minecraft:custom_name"),
            Some(&Value::String("{\"text\":\"MightyPick\"}".into()))
        );
        let Some(Value::List(lore)) = components.get("minecraft:lore") else {
            panic!()
        };
        assert_eq!(lore[0], Value::String("{\"text\":\"Lore line\"}".into()));

        let mut untouched = compound(vec![
            ("id", Value::String("minecraft:iron_pickaxe".into())),
            ("count", Value::Int(1)),
            (
                "components",
                Value::Compound(compound(vec![(
                    "minecraft:custom_name",
                    Value::String("MightyPick".into()),
                )])),
            ),
        ]);
        downgrade_compound(&mut untouched, 4671, &mut report);
        let Some(Value::Compound(components)) = untouched.get("components") else {
            panic!()
        };
        assert_eq!(
            components.get("minecraft:custom_name"),
            Some(&Value::String("MightyPick".into()))
        );
    }

    #[test]
    fn map_item_keeps_map_id() {
        let mut map_item = compound(vec![
            ("id", Value::String("minecraft:filled_map".into())),
            ("count", Value::Int(1)),
            (
                "components",
                Value::Compound(compound(vec![("minecraft:map_id", Value::Int(7))])),
            ),
        ]);
        let mut report = Report::default();
        downgrade_compound(&mut map_item, 2975, &mut report);

        let Some(Value::Compound(tag)) = map_item.get("tag") else {
            panic!("missing tag")
        };
        assert_eq!(tag.get("map"), Some(&Value::Int(7)));
        assert_eq!(report.dropped_data, 0);
    }

    #[test]
    fn lodestone_compass_keeps_target() {
        let target = compound(vec![
            (
                "pos",
                Value::IntArray(fastnbt::IntArray::new(vec![8, 69, 17])),
            ),
            ("dimension", Value::String("minecraft:overworld".into())),
        ]);
        let mut compass = compound(vec![
            ("id", Value::String("minecraft:compass".into())),
            ("count", Value::Int(1)),
            (
                "components",
                Value::Compound(compound(vec![(
                    "minecraft:lodestone_tracker",
                    Value::Compound(compound(vec![("target", Value::Compound(target))])),
                )])),
            ),
        ]);

        let mut report = Report::default();
        downgrade_compound(&mut compass, 2975, &mut report);

        assert!(!compass.contains_key("components"));
        let Some(Value::Compound(tag)) = compass.get("tag") else {
            panic!("missing tag")
        };
        assert_eq!(tag.get("LodestoneTracked"), Some(&Value::Byte(1)));
        assert_eq!(
            tag.get("LodestoneDimension"),
            Some(&Value::String("minecraft:overworld".into()))
        );
        let Some(Value::Compound(pos)) = tag.get("LodestonePos") else {
            panic!("missing LodestonePos")
        };
        assert_eq!(pos.get("X"), Some(&Value::Int(8)));
        assert_eq!(pos.get("Y"), Some(&Value::Int(69)));
        assert_eq!(pos.get("Z"), Some(&Value::Int(17)));
        assert_eq!(report.dropped_data, 0);
    }

    #[test]
    fn command_block_item_keeps_command() {
        let block_entity_data = compound(vec![
            ("id", Value::String("minecraft:command_block".into())),
            ("Command", Value::String("/say hi".into())),
            ("auto", Value::Byte(0)),
        ]);
        let mut item = compound(vec![
            ("id", Value::String("minecraft:command_block".into())),
            ("count", Value::Int(1)),
            (
                "components",
                Value::Compound(compound(vec![(
                    "minecraft:block_entity_data",
                    Value::Compound(block_entity_data),
                )])),
            ),
        ]);

        let mut report = Report::default();
        downgrade_compound(&mut item, 3465, &mut report);

        assert!(!item.contains_key("components"));
        let Some(Value::Compound(tag)) = item.get("tag") else {
            panic!("missing tag")
        };
        let Some(Value::Compound(bet)) = tag.get("BlockEntityTag") else {
            panic!("missing BlockEntityTag")
        };
        assert_eq!(bet.get("Command"), Some(&Value::String("/say hi".into())));
        assert_eq!(bet.get("auto"), Some(&Value::Byte(0)));
        assert_eq!(report.dropped_data, 0);
    }

    #[test]
    fn no_changes_for_1_21_5_and_later() {
        let mut entity = compound(vec![
            ("fall_distance", Value::Double(1.0)),
            ("equipment", Value::Compound(HashMap::new())),
        ]);
        let mut report = Report::default();
        downgrade_compound(&mut entity, 4671, &mut report);
        assert!(entity.contains_key("fall_distance"));
        assert!(entity.contains_key("equipment"));
    }
}
