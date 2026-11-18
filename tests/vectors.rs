//! Checks the Rust port against what the JavaScript page produced.
//!
//! `vectors.json` was written by the old page's own code (app_2.js with crypto-js
//! and mlkem): huH codec output, messages and files locked the pre-compression
//! way, and an ML-KEM-1024 exchange.

use huh_encrypter::{
    b64, cipher, huh, kem,
    locker::{unlock, Unlocked},
    payload, squeeze,
};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!("vectors.json")).unwrap()
}

fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or_else(|| panic!("missing {key}"))
}

fn bytes(v: &Value, key: &str) -> Vec<u8> {
    b64::decode(s(v, key)).unwrap()
}

#[test]
fn codec_matches_the_page() {
    let v = vectors();
    let cases = v["codec"].as_array().unwrap();
    assert!(cases.len() > 20);
    for case in cases {
        let raw = bytes(case, "bytes");
        assert_eq!(huh::encode(&raw), s(case, "huh"), "encode {raw:?}");
        if raw.len() >= 2 {
            assert_eq!(huh::decode(s(case, "huh")).unwrap(), raw, "decode {raw:?}");
        }
    }
}

#[test]
fn old_messages_still_unlock() {
    let v = vectors();
    for case in v["text"].as_array().unwrap() {
        let want = Ok(Unlocked::Text(s(case, "msg").to_owned()));
        let pass = s(case, "pass");
        assert_eq!(unlock(s(case, "huh"), pass), want, "huH");
        assert_eq!(unlock(s(case, "legacyHuh"), pass), want, "first huH format");
        assert_eq!(unlock(s(case, "base64"), pass), want, "pre-huH base64");
        assert!(unlock(s(case, "huh"), "not the password").is_err());
    }
}

#[test]
fn old_files_still_unlock() {
    let v = vectors();
    for case in v["files"].as_array().unwrap() {
        let (name, mime, data) = (s(case, "name"), s(case, "type"), bytes(case, "bytes"));
        assert_eq!(
            unlock(s(case, "huh"), s(case, "pass")),
            Ok(Unlocked::File { name: name.to_owned(), mime: mime.to_owned(), bytes: data.clone() })
        );
        // The Rust packer writes the very bytes the page's packFile did.
        assert_eq!(payload::pack_file(&data, name, mime).unwrap(), bytes(case, "packed"));
    }
}

/// `squeezed.json` holds huHs locked by the Rust build that introduced each
/// squeezed form: unsqueezed, long (`L`), short (`S`-`V`) and word list (`W`-`Z`).
/// Whatever changes later, these have to keep unlocking.
#[test]
fn squeezed_huhs_stay_readable() {
    let v: Value = serde_json::from_str(include_str!("squeezed.json")).unwrap();
    let mut seen = String::new();
    for case in v["huhs"].as_array().unwrap() {
        let (name, pass, locked) = (s(case, "name"), s(case, "pass"), s(case, "huh"));
        let bytes = match unlock(locked, pass).unwrap_or_else(|e| panic!("{name}: {e:?}")) {
            Unlocked::Text(text) => {
                assert!(case["file"].is_null(), "{name} should be a file");
                if let Some(want) = case["text"].as_str() {
                    assert_eq!(text, want, "{name}");
                }
                text.into_bytes()
            }
            Unlocked::File { name: file, mime, bytes } => {
                assert_eq!(file, s(&case["file"], "name"), "{name}");
                assert_eq!(mime, s(&case["file"], "type"), "{name}");
                bytes
            }
        };
        assert_eq!(bytes.len() as u64, case["len"].as_u64().unwrap(), "{name}");
        assert_eq!(u64::from(crc32fast::hash(&bytes)), case["crc32"].as_u64().unwrap(), "{name}");

        // And it really is in the form it is here to pin.
        let payload = cipher::open(&huh::decode(locked).unwrap(), pass.as_bytes()).unwrap();
        let letter = if squeeze::is_squeezed(&payload) { payload[1] as char } else { '-' };
        assert_eq!(letter.to_string(), s(case, "letter"), "{name}");
        seen.push(letter);
    }
    for family in ["-", "L", "STUV", "WXYZ"] {
        assert!(seen.chars().any(|c| family.contains(c)), "no fixture in the {family} forms");
    }
}

#[test]
#[allow(deprecated)]
fn ml_kem_agrees_with_the_page() {
    use ml_kem::{kem::Decapsulate, DecapsulationKey, MlKem1024};

    let v = vectors();
    let k = &v["kem"];
    // The page's private key is in the expanded form; ours opens the page's reply with it.
    let sk = bytes(k, "sk");
    let dk = DecapsulationKey::<MlKem1024>::from_expanded(sk.as_slice().try_into().unwrap()).unwrap();
    let shared: [u8; 32] = dk.decapsulate_slice(&bytes(k, "ct")).unwrap().into();
    assert_eq!(shared.to_vec(), bytes(k, "ss"));

    // And the page's public key is one we can make a reply for.
    let (reply, secret) = kem::encapsulate(&bytes(k, "pk")).unwrap();
    assert_eq!(reply.len(), 1568);
    let back: [u8; 32] = dk.decapsulate_slice(&reply).unwrap().into();
    assert_eq!(back, secret);
}
