pub mod data_files;
pub mod level_dat;

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use fastnbt::Value;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::error::{Error, Result};

pub fn parse_nbt_bytes(bytes: &[u8], context: &str) -> Result<Value> {
    let raw: Vec<u8> = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        GzDecoder::new(bytes)
            .read_to_end(&mut out)
            .map_err(|e| Error::nbt(context, e))?;
        out
    } else if bytes.first() == Some(&0x78) {
        let mut out = Vec::new();
        ZlibDecoder::new(bytes)
            .read_to_end(&mut out)
            .map_err(|e| Error::nbt(context, e))?;
        out
    } else {
        bytes.to_vec()
    };
    fastnbt::from_bytes(&raw).map_err(|e| Error::nbt(context, e))
}

pub fn read_nbt_file(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).map_err(|e| Error::io(path, e))?;
    parse_nbt_bytes(&bytes, &path.display().to_string())
}

pub fn write_nbt_gzip(path: &Path, value: &Value) -> Result<()> {
    let raw = fastnbt::to_bytes(value).map_err(|e| Error::nbt(path.display(), e))?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&raw)
        .and_then(|_| encoder.finish())
        .map_err(|e| Error::io(path, e))
        .and_then(|compressed| fs::write(path, compressed).map_err(|e| Error::io(path, e)))
}
