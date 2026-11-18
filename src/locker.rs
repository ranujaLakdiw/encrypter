//! Locking and unlocking, end to end: text or a file in, huH out, and back.
//!
//! Lock: payload → squeeze (LZMA2, when it helps) → AES → huH.
//! Unlock reads everything the page has ever produced: today's huH, the first huH
//! format, and raw CryptoJS base64 from before huH existed.
//!
//! Each direction comes in two steps, so the page can show which one it is on:
//! locking is [`prepare_text`] / [`prepare_file`] (compress) and then
//! [`Prepared::seal`] (encrypt); unlocking is [`open`] (decrypt) and then
//! [`Opened::finish`] (decompress). [`lock_text`], [`lock_file`] and [`unlock`] do
//! both at once.

use crate::{b64, cipher, huh, payload, squeeze};

#[derive(Debug, PartialEq, Eq)]
pub enum Unlocked {
    Text(String),
    File { name: String, mime: String, bytes: Vec<u8> },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnlockError {
    /// Not a huH at all.
    NotHuh,
    /// It didn't open: the wrong password or session, or a huH that isn't whole.
    WrongKey,
    /// The key was right, but what was inside is cut short or altered.
    Damaged,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LockError {
    NameTooLong,
    TooBig,
}

impl LockError {
    pub fn message(self) -> &'static str {
        match self {
            LockError::NameTooLong => "That filename is too long.",
            LockError::TooBig => "That one is too big to lock.",
        }
    }
}

/// A payload ready to be encrypted: squeezed, if that made it smaller.
#[derive(Debug)]
pub struct Prepared(Vec<u8>);

impl Prepared {
    /// Whether squeezing helped (a two-word note or a JPEG goes in as it is).
    pub fn is_squeezed(&self) -> bool {
        squeeze::is_squeezed(&self.0)
    }

    /// Encrypts under a password or session key and writes the result as huH.
    pub fn seal(self, key: &str) -> String {
        let sealed = cipher::seal(&self.0, key.as_bytes());
        drop(self.0);
        huh::encode(&sealed)
    }
}

fn prepare(payload: Vec<u8>) -> Result<Prepared, LockError> {
    squeeze::squeeze(payload).map(Prepared).map_err(|_| LockError::TooBig)
}

/// The first step of locking a typed message: compress it.
pub fn prepare_text(msg: &str) -> Result<Prepared, LockError> {
    prepare(msg.as_bytes().to_vec())
}

/// The first step of locking a file: pack its bytes, name and type, and compress that.
pub fn prepare_file(bytes: &[u8], name: &str, mime: &str) -> Result<Prepared, LockError> {
    prepare(payload::pack_file(bytes, name, mime).map_err(|_| LockError::NameTooLong)?)
}

/// Locks a typed message into huH.
pub fn lock_text(msg: &str, key: &str) -> Result<String, LockError> {
    Ok(prepare_text(msg)?.seal(key))
}

/// Locks a file (its bytes, name and type) into huH.
pub fn lock_file(bytes: &[u8], name: &str, mime: &str, key: &str) -> Result<String, LockError> {
    Ok(prepare_file(bytes, name, mime)?.seal(key))
}

/// What came out of the AES envelope: the payload, possibly still squeezed.
#[derive(Debug)]
pub struct Opened(Vec<u8>);

/// The first step of unlocking: read the huH (already trimmed) and decrypt it
/// with a password or session key.
pub fn open(msg: &str, key: &str) -> Result<Opened, UnlockError> {
    let sealed = if huh::is_huh(msg) {
        huh::decode(msg).map_err(|_| UnlockError::NotHuh)?
    } else if msg.starts_with("U2FsdGVk") {
        // Raw CryptoJS output (the pre-huH format) always starts with "Salted__" in base64.
        b64::decode(msg).unwrap_or_default()
    } else {
        return Err(UnlockError::NotHuh);
    };
    cipher::open(&sealed, key.as_bytes())
        .filter(|p| !p.is_empty())
        .map(Opened)
        .ok_or(UnlockError::WrongKey)
}

impl Opened {
    /// Whether there is anything to decompress.
    pub fn is_squeezed(&self) -> bool {
        squeeze::is_squeezed(&self.0)
    }

    /// The second step of unlocking: decompress if it was squeezed, and work out
    /// whether it is a file or a message.
    pub fn finish(self) -> Result<Unlocked, UnlockError> {
        let mut payload = self.0;
        // The key was right to get this far, so a failure here is the huH itself.
        if squeeze::is_squeezed(&payload) {
            payload = squeeze::unsqueeze(&payload).map_err(|_| UnlockError::Damaged)?;
        }

        if let Some(file) = payload::unpack_file(&payload) {
            payload.drain(..file.data_start);
            return Ok(Unlocked::File { name: file.name, mime: file.mime, bytes: payload });
        }

        match String::from_utf8(payload) {
            Ok(text) if !text.is_empty() => Ok(Unlocked::Text(text)),
            _ => Err(UnlockError::WrongKey),
        }
    }
}

/// Unlocks a huH (already trimmed) with a password or session key.
pub fn unlock(msg: &str, key: &str) -> Result<Unlocked, UnlockError> {
    open(msg, key)?.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trip() {
        for msg in ["hi", "  spaces kept  \n", "සිංහල 🐸 \u{feff}bom", &"long message. ".repeat(500)] {
            let huh = lock_text(msg, "pw").unwrap();
            assert!(huh::is_huh(&huh));
            assert_eq!(unlock(&huh, "pw"), Ok(Unlocked::Text(msg.to_owned())));
            assert_eq!(unlock(&huh, "other"), Err(UnlockError::WrongKey));
        }
    }

    #[test]
    fn in_two_steps() {
        let msg = "Let me know when you get home. ".repeat(6);
        let prepared = prepare_text(&msg).unwrap();
        assert!(prepared.is_squeezed());
        let huh = prepared.seal("pw");
        let opened = open(&huh, "pw").unwrap();
        assert!(opened.is_squeezed());
        assert_eq!(opened.finish(), Ok(Unlocked::Text(msg)));
        assert_eq!(open(&huh, "nope").err(), Some(UnlockError::WrongKey));

        // A note too short to squeeze leaves nothing to decompress.
        let short = prepare_text("hi").unwrap();
        assert!(!short.is_squeezed());
        assert!(!open(&short.seal("pw"), "pw").unwrap().is_squeezed());

        let file = prepare_file(&[7u8; 4000], "sevens.bin", "").unwrap();
        assert!(file.is_squeezed());
        assert_eq!(
            open(&file.seal("pw"), "pw").unwrap().finish(),
            Ok(Unlocked::File { name: "sevens.bin".into(), mime: String::new(), bytes: vec![7u8; 4000] })
        );
    }

    #[test]
    fn long_text_gets_shorter() {
        let msg = "The quick brown fox jumps over the lazy dog. ".repeat(20);
        let plain_blocks = cipher::blocks(msg.len());
        let huh = lock_text(&msg, "pw").unwrap();
        let sealed = huh::decode(&huh).unwrap();
        assert!(sealed.len() < 16 + plain_blocks * 16 / 4);
    }

    #[test]
    fn file_round_trip() {
        let compressible = "id,name,score\n1,someone,42\n".repeat(400).into_bytes();
        let mut noise = vec![0u8; 4000];
        crate::rng::fill(&mut noise);
        for (bytes, name, mime) in [
            (compressible, "scores.csv", "text/csv"),
            (noise, "රූපය 🐸.bin", ""),
            (Vec::new(), "empty", "application/octet-stream"),
        ] {
            let huh = lock_file(&bytes, name, mime, "pw").unwrap();
            assert_eq!(
                unlock(&huh, "pw"),
                Ok(Unlocked::File { name: name.to_owned(), mime: mime.to_owned(), bytes })
            );
        }
    }

    #[test]
    fn not_a_huh() {
        assert_eq!(unlock("hello there", "pw"), Err(UnlockError::NotHuh));
        assert_eq!(unlock("huHUhuHUhuH", "pw"), Err(UnlockError::WrongKey));
        assert_eq!(unlock("U2FsdGVkX1+++", "pw"), Err(UnlockError::WrongKey));
    }

    #[test]
    fn a_cut_huh_says_so() {
        let msg = "all work and no play makes jack a dull boy. ".repeat(40);
        let sealed = huh::decode(&lock_text(&msg, "pw").unwrap()).unwrap();
        // Re-seal a squeezed payload with its tail missing: right key, broken contents.
        let mut payload = cipher::open(&sealed, b"pw").unwrap();
        assert!(squeeze::is_squeezed(&payload));
        payload.truncate(payload.len() - 7);
        let broken = huh::encode(&cipher::seal(&payload, b"pw"));
        assert_eq!(unlock(&broken, "pw"), Err(UnlockError::Damaged));
    }
}
