use std::fs;

use fastanvil::Region;
use fastnbt::Value;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: scan_component <region dir> <component key> [full]");
    let key = args
        .next()
        .expect("usage: scan_component <region dir> <component key> [full]");
    let full = args.next().is_some_and(|a| a == "full");
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
            let Ok(value) = fastnbt::from_bytes::<Value>(&chunk.data) else {
                continue;
            };
            search(&value, &key, full, &path.display().to_string());
        }
    }
}

fn search(value: &Value, key: &str, full: bool, context: &str) {
    match value {
        Value::Compound(map) => {
            if let Some(found) = map.get(key) {
                println!("=== {context} ===");
                if full {
                    println!("{map:#?}");
                } else {
                    println!("{found:#?}");
                }
            }
            for v in map.values() {
                search(v, key, full, context);
            }
        }
        Value::List(items) => {
            for v in items {
                search(v, key, full, context);
            }
        }
        _ => {}
    }
}
