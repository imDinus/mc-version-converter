use std::collections::HashSet;
use std::fs;

use fastanvil::Region;
use fastnbt::Value;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: scan_blocks <region dir> <substring>");
    let needle = args
        .next()
        .expect("usage: scan_blocks <region dir> <substring>");
    let mut seen: HashSet<String> = HashSet::new();

    for entry in fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "mca") {
            continue;
        }
        let Ok(file) = fs::File::open(&path) else {
            continue;
        };
        let Ok(mut region) = Region::from_stream(file) else {
            continue;
        };
        for chunk in region.iter().flatten() {
            let Ok(Value::Compound(map)) = fastnbt::from_bytes::<Value>(&chunk.data) else {
                continue;
            };
            if let Some(Value::List(sections)) = map.get("sections") {
                for section in sections {
                    let Value::Compound(section) = section else {
                        continue;
                    };
                    let Some(Value::Compound(bs)) = section.get("block_states") else {
                        continue;
                    };
                    let Some(Value::List(palette)) = bs.get("palette") else {
                        continue;
                    };
                    for block in palette {
                        let Value::Compound(block) = block else {
                            continue;
                        };
                        let Some(Value::String(name)) = block.get("Name") else {
                            continue;
                        };
                        if name.contains(&needle) && seen.insert(name.clone()) {
                            println!("palette: {}", name);
                            println!("{block:#?}");
                        }
                    }
                }
            }
            if let Some(Value::List(block_entities)) = map.get("block_entities") {
                for be in block_entities {
                    let Value::Compound(be) = be else { continue };
                    let Some(Value::String(id)) = be.get("id") else {
                        continue;
                    };
                    if id.contains(&needle) && seen.insert(format!("be:{id}")) {
                        println!("block entity:");
                        println!("{be:#?}");
                    }
                }
            }
        }
    }
    if seen.is_empty() {
        println!("(no matches for '{needle}')");
    }
}
