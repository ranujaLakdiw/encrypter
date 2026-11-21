//! Turning secrets into keys, with the two standard functions made for it.
//!
//! * **PBKDF2-HMAC-SHA256** makes a key out of something a person typed. It is
//!   slow on purpose (hundreds of thousands of rounds), so that trying one
//!   password after another against a stolen huH is slow as well.
//! * **HKDF-SHA256** (RFC 5869) makes keys out of something that is already
//!   random, such as a session's secret. It takes no time at all, and every
//!   `info` it is given makes a key that says nothing about the others.
//!
//! Neither is this program's own: the browser's `crypto.subtle`, OpenSSL and
//! Node all compute the very same bytes, and the tests hold their answers.

use pbkdf2::{
    hmac::{Hmac, KeyInit, Mac},
    pbkdf2_hmac_array,
    sha2::{Digest, Sha256},
};

/// What SHA-256 gives: 32 bytes.
const HASH: usize = 32;

pub type Key = [u8; 32];

/// PBKDF2-HMAC-SHA256, 32 bytes out.
pub fn pbkdf2(pass: &str, salt: &[u8], rounds: u32) -> Key {
    pbkdf2_hmac_array::<Sha256, 32>(pass.as_bytes(), salt, rounds)
}

/// SHA-256 of the parts, one after another.
pub fn sha256(parts: &[&[u8]]) -> [u8; HASH] {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update(part);
    }
    hash.finalize().into()
}

fn hmac(key: &[u8], parts: &[&[u8]]) -> [u8; HASH] {
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(key).expect("HMAC takes a key of any length");
    for part in parts {
        mac.update(part);
    }
    mac.finalize().into_bytes().into()
}

/// HKDF-SHA256: `N` bytes made from `secret`, for the purpose `info` names.
/// `salt` may be empty.
pub fn hkdf<const N: usize>(secret: &[u8], salt: &[u8], info: &[u8]) -> [u8; N] {
    assert!(N <= 255 * HASH, "HKDF-SHA256 gives at most 8160 bytes");
    // Extract. (An empty salt is the same key to HMAC as the 32 zero bytes the
    // standard asks for, since HMAC pads its key with zeros.)
    let prk = hmac(salt, &[secret]);
    // Expand: each block is HMAC(prk, previous block · info · its number).
    let mut out = [0u8; N];
    let mut block: [u8; HASH] = [0; HASH];
    for (i, chunk) in out.chunks_mut(HASH).enumerate() {
        let previous: &[u8] = if i == 0 { &[] } else { &block };
        block = hmac(&prk, &[previous, info, &[i as u8 + 1]]);
        chunk.copy_from_slice(&block[..chunk.len()]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn hkdf_as_the_standard_says() {
        // RFC 5869, appendix A, test cases 1 and 3 (SHA-256).
        let secret = [0x0b; 22];
        let salt: Vec<u8> = (0x00..=0x0c).collect();
        let info: Vec<u8> = (0xf0..=0xf9).collect();
        assert_eq!(
            hkdf::<42>(&secret, &salt, &info).to_vec(),
            hex("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865")
        );
        assert_eq!(
            hkdf::<42>(&secret, &[], &[]).to_vec(),
            hex("8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8")
        );
        // A shorter answer is the start of a longer one, and `info` changes all of it.
        assert_eq!(hkdf::<16>(&secret, &salt, &info)[..], hkdf::<42>(&secret, &salt, &info)[..16]);
        assert_ne!(hkdf::<32>(&secret, &salt, b"one"), hkdf::<32>(&secret, &salt, b"two"));
    }

    #[test]
    fn hkdf_across_many_blocks() {
        // RFC 5869, test case 2: long inputs, 82 bytes out (three blocks).
        let secret: Vec<u8> = (0x00..=0x4f).collect();
        let salt: Vec<u8> = (0x60..=0xaf).collect();
        let info: Vec<u8> = (0xb0..=0xff).collect();
        assert_eq!(
            hkdf::<82>(&secret, &salt, &info).to_vec(),
            hex(concat!(
                "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c",
                "59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71",
                "cc30c58179ec3e87c14c01d5c1f3434f1d87"
            ))
        );
    }

    #[test]
    fn hkdf_at_odd_lengths() {
        // A block and one byte of the next (the answer is Node's), a longer one
        // that has to begin the same way, and nothing at all when asked for nothing.
        let secret: [u8; 32] = core::array::from_fn(|i| (i * 7 + 3) as u8);
        let salt: [u8; 16] = core::array::from_fn(|i| 0xA0 + i as u8);
        let thirty_three = hkdf::<33>(&secret, &salt, b"huH test");
        assert_eq!(
            thirty_three.to_vec(),
            hex("56bb3bcd199f015010a415192e5cdc0a69aadbcc82e2884b42e9c84bb0c52cd5aa")
        );
        assert_eq!(hkdf::<65>(&secret, &salt, b"huH test")[..33], thirty_three);
        assert_eq!(hkdf::<0>(&secret, &salt, b"huH test"), [0u8; 0]);
    }

    #[test]
    fn sha256_and_pbkdf2_as_the_standards_say() {
        assert_eq!(
            sha256(&[b"a", b"bc"]).to_vec(),
            hex("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
        assert_eq!(sha256(&[b"abc"]), sha256(&[b"ab", b"", b"c"]));
        // RFC 7914, section 11: PBKDF2-HMAC-SHA256("passwd", "salt", 1 round), first 32 bytes.
        assert_eq!(
            pbkdf2("passwd", b"salt", 1).to_vec(),
            hex("55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc")
        );
    }
}
