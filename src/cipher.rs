//! AES exactly as the page has always done it, so every huH made before still opens.
//!
//! That is the OpenSSL "Salted__" layout CryptoJS writes for a passphrase:
//! `"Salted__" · 8-byte salt · AES-256-CBC(PKCS#7)`, with the key and IV stretched
//! from passphrase + salt by one round of MD5 (`EVP_BytesToKey`).

use aes::cipher::{block_padding::Pkcs7, BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};
use md5::{Digest, Md5};

type Enc = cbc::Encryptor<aes::Aes256>;
type Dec = cbc::Decryptor<aes::Aes256>;

const HEADER: &[u8; 8] = b"Salted__";
const SALT: usize = 8;
const BLOCK: usize = 16;
const PREFIX: usize = HEADER.len() + SALT;

/// How many AES blocks a payload of `n` bytes takes once padded.
pub fn blocks(n: usize) -> usize {
    n / BLOCK + 1
}

fn derive(pass: &[u8], salt: &[u8]) -> ([u8; 32], [u8; 16]) {
    let mut material = [0u8; 48];
    let mut prev: Option<[u8; 16]> = None;
    for chunk in material.chunks_mut(16) {
        let mut h = Md5::new();
        if let Some(p) = prev {
            h.update(p);
        }
        h.update(pass);
        h.update(salt);
        let d: [u8; 16] = h.finalize().into();
        chunk.copy_from_slice(&d);
        prev = Some(d);
    }
    let mut key = [0u8; 32];
    let mut iv = [0u8; 16];
    key.copy_from_slice(&material[..32]);
    iv.copy_from_slice(&material[32..]);
    (key, iv)
}

/// Encrypts `plain` under the passphrase with the given salt.
pub fn seal_with_salt(plain: &[u8], pass: &[u8], salt: [u8; SALT]) -> Vec<u8> {
    let (key, iv) = derive(pass, &salt);
    let mut out = Vec::with_capacity(PREFIX + (plain.len() / BLOCK + 1) * BLOCK);
    out.extend_from_slice(HEADER);
    out.extend_from_slice(&salt);
    out.extend_from_slice(plain);
    out.resize(PREFIX + (plain.len() / BLOCK + 1) * BLOCK, 0);
    let n = Enc::new(&key.into(), &iv.into())
        .encrypt_padded::<Pkcs7>(&mut out[PREFIX..], plain.len())
        .expect("buffer holds the padded plaintext")
        .len();
    out.truncate(PREFIX + n);
    out
}

/// Encrypts `plain` under the passphrase with a fresh random salt.
pub fn seal(plain: &[u8], pass: &[u8]) -> Vec<u8> {
    let mut salt = [0u8; SALT];
    crate::rng::fill(&mut salt);
    seal_with_salt(plain, pass, salt)
}

/// Decrypts what `seal` (or CryptoJS) made. `None` for anything that isn't a whole,
/// correctly padded ciphertext under this passphrase.
pub fn open(data: &[u8], pass: &[u8]) -> Option<Vec<u8>> {
    if data.len() < PREFIX + BLOCK || &data[..HEADER.len()] != HEADER {
        return None;
    }
    let (salt, body) = data[HEADER.len()..].split_at(SALT);
    if body.len() % BLOCK != 0 {
        return None;
    }
    let (key, iv) = derive(pass, salt);
    let mut buf = body.to_vec();
    let n = Dec::new(&key.into(), &iv.into())
        .decrypt_padded::<Pkcs7>(&mut buf)
        .ok()?
        .len();
    buf.truncate(n);
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for len in [0usize, 1, 15, 16, 17, 1000] {
            let plain: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let sealed = seal(&plain, b"pw");
            assert_eq!(sealed.len(), PREFIX + blocks(len) * BLOCK);
            assert_eq!(open(&sealed, b"pw").unwrap(), plain);
            assert!(open(&sealed, b"pW").map_or(true, |p| p != plain));
        }
    }

    #[test]
    fn rejects_cut_or_headerless_input() {
        let sealed = seal(b"some plaintext here", b"pw");
        assert!(open(&sealed[..sealed.len() - 3], b"pw").is_none());
        assert!(open(&sealed[8..], b"pw").is_none());
        assert!(open(b"", b"pw").is_none());
    }
}
