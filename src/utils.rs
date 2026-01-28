use brotli::Decompressor;
use std::io::Read;

/// Decompress a Brotli-compressed byte slice into a UTF-8 string.
pub fn decompress_brotli(data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let mut decompressor = Decompressor::new(data, 4096);
    let mut decompressed = String::new();
    decompressor.read_to_string(&mut decompressed)?;
    Ok(decompressed)
}
