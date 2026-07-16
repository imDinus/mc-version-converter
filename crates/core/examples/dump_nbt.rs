use std::fs;

use fastanvil::Region;
use fastnbt::Value;
use mcconvert_core::nbt::parse_nbt_bytes;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: dump_nbt <file> [max_depth]");
    let max_depth: usize = args.next().and_then(|d| d.parse().ok()).unwrap_or(6);

    if path.ends_with(".mca") {
        let file = fs::File::open(&path).expect("failed to open file");
        let mut region = Region::from_stream(file).expect("failed to parse region file");
        let chunk = region
            .iter()
            .next()
            .expect("no chunks")
            .expect("failed to read chunk");
        println!("=== first chunk ({}, {}) ===", chunk.x, chunk.z);
        let value: Value = fastnbt::from_bytes(&chunk.data).expect("failed to parse chunk NBT");
        dump(&value, 0, max_depth);
    } else {
        let bytes = fs::read(&path).expect("failed to read file");
        let value = parse_nbt_bytes(&bytes, &path).expect("failed to parse NBT");
        dump(&value, 0, max_depth);
    }
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

fn dump(value: &Value, depth: usize, max_depth: usize) {
    match value {
        Value::Compound(map) => {
            if depth >= max_depth {
                println!("{}{{...{} keys}}", indent(depth), map.len());
                return;
            }
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                let v = &map[key];
                match v {
                    Value::Compound(_) | Value::List(_) => {
                        println!("{}{key}: {}", indent(depth), type_name(v));
                        dump(v, depth + 1, max_depth);
                    }
                    _ => println!("{}{key}: {}", indent(depth), scalar(v)),
                }
            }
        }
        Value::List(items) => {
            if items.is_empty() {
                println!("{}(empty list)", indent(depth));
            } else {
                println!("{}[0]/{} items:", indent(depth), items.len());
                dump(&items[0], depth + 1, max_depth);
            }
        }
        other => println!("{}{}", indent(depth), scalar(other)),
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Compound(_) => "(compound)",
        Value::List(_) => "(list)",
        _ => "",
    }
}

fn scalar(value: &Value) -> String {
    match value {
        Value::Byte(v) => format!("{v}b"),
        Value::Short(v) => format!("{v}s"),
        Value::Int(v) => format!("{v}"),
        Value::Long(v) => format!("{v}L"),
        Value::Float(v) => format!("{v}f"),
        Value::Double(v) => format!("{v}d"),
        Value::String(v) => format!("\"{v}\""),
        Value::ByteArray(v) => format!("[B;{} items]", v.len()),
        Value::IntArray(v) => {
            if v.len() <= 4 {
                format!("[I;{:?}]", v.iter().collect::<Vec<_>>())
            } else {
                format!("[I;{} items]", v.len())
            }
        }
        Value::LongArray(v) => format!("[L;{} items]", v.len()),
        other => format!("{other:?}"),
    }
}
