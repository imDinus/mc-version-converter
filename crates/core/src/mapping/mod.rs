use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use fastnbt::Value;

use crate::error::{Error, Result};

pub struct BlockTable {
    allowed: HashSet<String>,
}

impl BlockTable {
    pub fn load_json(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        let ids: Vec<String> =
            serde_json::from_str(&text).map_err(|e| Error::nbt(path.display(), e))?;
        Ok(Self {
            allowed: ids.into_iter().collect(),
        })
    }

    pub fn contains(&self, id: &str) -> bool {
        self.allowed.contains(id)
    }

    pub fn len(&self) -> usize {
        self.allowed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.allowed.is_empty()
    }
}

pub fn filter_chunk_blocks(chunk: &mut HashMap<String, Value>, table: &BlockTable) -> u64 {
    let mut replaced = 0;
    let Some(Value::List(sections)) = chunk.get_mut("sections") else {
        return 0;
    };
    for section in sections {
        let Value::Compound(section) = section else {
            continue;
        };
        let Some(Value::Compound(block_states)) = section.get_mut("block_states") else {
            continue;
        };
        let Some(Value::List(palette)) = block_states.get_mut("palette") else {
            continue;
        };
        for entry in palette {
            let Value::Compound(block) = entry else {
                continue;
            };
            let Some(Value::String(name)) = block.get("Name") else {
                continue;
            };
            if !table.contains(name) {
                block.insert("Name".into(), Value::String("minecraft:air".into()));
                block.remove("Properties");
                replaced += 1;
            }
        }
    }
    replaced
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(ids: &[&str]) -> BlockTable {
        BlockTable {
            allowed: ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn unsupported_block_to_air() {
        let mut palette = Vec::new();
        for name in ["minecraft:stone", "minecraft:future_block"] {
            let mut block = HashMap::new();
            block.insert("Name".to_string(), Value::String(name.into()));
            palette.push(Value::Compound(block));
        }
        let mut block_states = HashMap::new();
        block_states.insert("palette".to_string(), Value::List(palette));
        let mut section = HashMap::new();
        section.insert("block_states".to_string(), Value::Compound(block_states));
        let mut chunk = HashMap::new();
        chunk.insert(
            "sections".to_string(),
            Value::List(vec![Value::Compound(section)]),
        );

        let replaced = filter_chunk_blocks(&mut chunk, &table(&["minecraft:stone"]));
        assert_eq!(replaced, 1);

        let Value::List(sections) = &chunk["sections"] else {
            panic!()
        };
        let Value::Compound(section) = &sections[0] else {
            panic!()
        };
        let Value::Compound(bs) = &section["block_states"] else {
            panic!()
        };
        let Value::List(palette) = &bs["palette"] else {
            panic!()
        };
        let Value::Compound(second) = &palette[1] else {
            panic!()
        };
        assert_eq!(second["Name"], Value::String("minecraft:air".into()));
    }
}
