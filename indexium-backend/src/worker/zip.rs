use flate2::read::DeflateDecoder;
use std::io::Read;

use super::jar_parser::JarParserError;

const EOCD_SIG: u32 = 0x0605_4b50;
const CENTRAL_DIR_SIG: u32 = 0x0201_4b50;

#[derive(Debug, Clone)]
pub struct Eocd {
    pub central_dir_offset: u32,
    pub central_dir_size: u32,
    pub num_entries: u16,
}

#[derive(Debug, Clone)]
pub struct CentralDirEntry {
    pub file_name: String,
    pub local_header_offset: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub compression_method: u16,
}

const MANIFEST_CANDIDATES: &[&str] = &[
    "fabric.mod.json",
    "quilt.mod.json",
    "neoforge.mods.toml",
    "mcmod.info",
];

/// Find EOCD in tail bytes (scan backwards).
pub fn find_eocd(tail: &[u8]) -> Result<Eocd, JarParserError> {
    if tail.len() < 22 {
        return Err(JarParserError::EocdNotFound);
    }
    for i in (0..=tail.len() - 4).rev() {
        if u32::from_le_bytes(tail[i..i + 4].try_into().unwrap()) == EOCD_SIG {
            if tail.len() < i + 22 {
                continue;
            }
            let num_entries = u16::from_le_bytes(tail[i + 10..i + 12].try_into().unwrap());
            let central_dir_size = u32::from_le_bytes(tail[i + 12..i + 16].try_into().unwrap());
            let central_dir_offset = u32::from_le_bytes(tail[i + 16..i + 20].try_into().unwrap());
            return Ok(Eocd {
                central_dir_offset,
                central_dir_size,
                num_entries,
            });
        }
    }
    Err(JarParserError::EocdNotFound)
}

/// Parse central directory slice into entries.
pub fn parse_central_dir(data: &[u8], eocd: &Eocd) -> Result<Vec<CentralDirEntry>, JarParserError> {
    let mut entries = Vec::with_capacity(eocd.num_entries as usize);
    let mut offset = 0usize;

    for _ in 0..eocd.num_entries {
        if offset + 46 > data.len() {
            return Err(JarParserError::InvalidCentralDir("truncated header".into()));
        }
        if u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) != CENTRAL_DIR_SIG {
            return Err(JarParserError::InvalidCentralDir("bad signature".into()));
        }
        let compression_method =
            u16::from_le_bytes(data[offset + 10..offset + 12].try_into().unwrap());
        let compressed_size =
            u32::from_le_bytes(data[offset + 20..offset + 24].try_into().unwrap());
        let uncompressed_size =
            u32::from_le_bytes(data[offset + 24..offset + 28].try_into().unwrap());
        let file_name_len =
            u16::from_le_bytes(data[offset + 28..offset + 30].try_into().unwrap()) as usize;
        let extra_len =
            u16::from_le_bytes(data[offset + 30..offset + 32].try_into().unwrap()) as usize;
        let comment_len =
            u16::from_le_bytes(data[offset + 32..offset + 34].try_into().unwrap()) as usize;
        let local_header_offset =
            u32::from_le_bytes(data[offset + 42..offset + 46].try_into().unwrap());

        let name_start = offset + 46;
        let name_end = name_start + file_name_len;
        if name_end > data.len() {
            return Err(JarParserError::InvalidCentralDir(
                "name out of bounds".into(),
            ));
        }
        let file_name = String::from_utf8_lossy(&data[name_start..name_end]).to_string();

        entries.push(CentralDirEntry {
            file_name,
            local_header_offset,
            compressed_size,
            uncompressed_size,
            compression_method,
        });

        offset = name_end + extra_len + comment_len;
    }
    Ok(entries)
}

pub fn find_manifest_entry<'a>(entries: &'a [CentralDirEntry]) -> Option<&'a CentralDirEntry> {
    for cand in MANIFEST_CANDIDATES {
        if let Some(e) = entries.iter().find(|e| e.file_name == *cand) {
            return Some(e);
        }
    }
    None
}

pub fn decompress_entry(raw: &[u8], entry: &CentralDirEntry) -> Result<Vec<u8>, JarParserError> {
    match entry.compression_method {
        0 => Ok(raw.to_vec()),
        8 => {
            let mut decoder = DeflateDecoder::new(raw);
            let mut out = Vec::with_capacity(entry.uncompressed_size as usize);
            decoder
                .read_to_end(&mut out)
                .map_err(|e| JarParserError::Decompression(e.to_string()))?;
            Ok(out)
        }
        m => Err(JarParserError::Decompression(format!(
            "unsupported method {m}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_eocd_bytes(offset: u32, size: u32, num: u16) -> Vec<u8> {
        let mut v = vec![0u8; 22];
        v[0..4].copy_from_slice(&EOCD_SIG.to_le_bytes());
        v[10..12].copy_from_slice(&num.to_le_bytes());
        v[12..16].copy_from_slice(&size.to_le_bytes());
        v[16..20].copy_from_slice(&offset.to_le_bytes());
        v
    }

    #[test]
    fn find_eocd_ok() {
        let tail = [vec![0u8; 100], make_eocd_bytes(123, 456, 2)].concat();
        let eocd = find_eocd(&tail).unwrap();
        assert_eq!(eocd.central_dir_offset, 123);
        assert_eq!(eocd.num_entries, 2);
    }

    #[test]
    fn find_eocd_not_found() {
        assert!(find_eocd(&[0u8; 100]).is_err());
    }
}
