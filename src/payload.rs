//! What goes inside the AES envelope for a locked file.
//!
//! `MAGIC · 2-byte header length · JSON header {"n": name, "t": type} · bytes`.
//! MAGIC starts with NUL, which no typed message can, so unlock can tell a file
//! from text.

pub const FILE_MAGIC: [u8; 5] = [0x00, b'h', b'u', b'H', b'F'];
const BASE: usize = FILE_MAGIC.len() + 2;

#[derive(Debug, PartialEq, Eq)]
pub struct NameTooLong;

pub fn pack_file(bytes: &[u8], name: &str, mime: &str) -> Result<Vec<u8>, NameTooLong> {
    let header = format!(
        "{{\"n\":{},\"t\":{}}}",
        serde_json::Value::from(name),
        serde_json::Value::from(mime)
    );
    let header = header.as_bytes();
    if header.len() > 0xffff {
        return Err(NameTooLong);
    }
    let mut out = Vec::with_capacity(BASE + header.len() + bytes.len());
    out.extend_from_slice(&FILE_MAGIC);
    out.extend_from_slice(&(header.len() as u16).to_be_bytes());
    out.extend_from_slice(header);
    out.extend_from_slice(bytes);
    Ok(out)
}

/// A file found inside a payload: its name, its type, and where its bytes start.
#[derive(Debug, PartialEq, Eq)]
pub struct FileHeader {
    pub name: String,
    pub mime: String,
    pub data_start: usize,
}

/// Reads the file header of a payload, or `None` for anything that isn't a file.
pub fn unpack_file(payload: &[u8]) -> Option<FileHeader> {
    if payload.len() < BASE || payload[..FILE_MAGIC.len()] != FILE_MAGIC {
        return None;
    }
    let hlen = u16::from_be_bytes([payload[FILE_MAGIC.len()], payload[FILE_MAGIC.len() + 1]]) as usize;
    let data_start = BASE + hlen;
    if data_start > payload.len() {
        return None;
    }
    let header: serde_json::Value = serde_json::from_slice(&payload[BASE..data_start]).ok()?;
    let name = header.get("n")?.as_str()?.to_owned();
    let mime = header.get("t").and_then(|t| t.as_str()).unwrap_or("").to_owned();
    Some(FileHeader { name, mime, data_start })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let name = "රූපය 🐸 \"quoted\" \\ back\n.bin";
        let packed = pack_file(&[1, 2, 3], name, "image/png").unwrap();
        let h = unpack_file(&packed).unwrap();
        assert_eq!(h.name, name);
        assert_eq!(h.mime, "image/png");
        assert_eq!(&packed[h.data_start..], &[1, 2, 3]);
    }

    #[test]
    fn empty_file_and_missing_type() {
        let packed = pack_file(&[], "a", "").unwrap();
        let h = unpack_file(&packed).unwrap();
        assert_eq!((h.name.as_str(), h.mime.as_str(), h.data_start), ("a", "", packed.len()));
    }

    #[test]
    fn not_a_file() {
        assert!(unpack_file(b"hello").is_none());
        assert!(unpack_file(&[0, b'h', b'u', b'H', b'F', 0xff, 0xff, b'{']).is_none());
        assert!(unpack_file(&[0, b'h', b'u', b'H', b'F', 0, 2, b'[', b']']).is_none());
        assert_eq!(pack_file(&[], &"x".repeat(70_000), ""), Err(NameTooLong));
    }
}
