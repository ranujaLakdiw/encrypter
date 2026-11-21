//! Checks the Rust port against what the JavaScript page produced, and each
//! format since against what the build that introduced it produced.
//!
//! `vectors.json` was written by the old page's own code (app_2.js with crypto-js
//! and mlkem): huH codec output, messages and files locked the pre-compression
//! way, and an ML-KEM-1024 exchange.

use huh_encrypter::{
    b64, cipher, envelope, huh, kem,
    locker::{self, unlock, Key, Needs, Unlocked},
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
        let pass = Key::Password(s(case, "pass"));
        assert_eq!(unlock(s(case, "huh"), pass), want, "huH");
        assert_eq!(unlock(s(case, "legacyHuh"), pass), want, "first huH format");
        assert_eq!(unlock(s(case, "base64"), pass), want, "pre-huH base64");
        assert!(unlock(s(case, "huh"), Key::Password("not the password")).is_err());
        // Every one of them is in the old envelope, and says so when opened.
        for form in ["huh", "legacyHuh", "base64"] {
            let sealed = locker::read(s(case, form)).unwrap();
            assert_eq!(sealed.needs(), Needs::Old, "{form}");
            assert!(sealed.open(pass).unwrap().is_old(), "{form}");
        }
    }
}

#[test]
fn old_files_still_unlock() {
    let v = vectors();
    for case in v["files"].as_array().unwrap() {
        let (name, mime, data) = (s(case, "name"), s(case, "type"), bytes(case, "bytes"));
        assert_eq!(
            unlock(s(case, "huh"), Key::Password(s(case, "pass"))),
            Ok(Unlocked::File { name: name.to_owned(), mime: mime.to_owned(), bytes: data.clone() })
        );
        // The Rust packer writes the very bytes the page's packFile did.
        assert_eq!(payload::pack_file(&data, name, mime).unwrap(), bytes(case, "packed"));
    }
}

/// What a fixture's huH unlocks to, checked against what the fixture says of it:
/// the text itself where it holds one, the file's name and type, and the length
/// and checksum of the bytes either way.
fn check_unlocked(case: &Value, unlocked: Unlocked) {
    let name = s(case, "name");
    let bytes = match unlocked {
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
}

/// '-' for a payload that went in as it was, else the letter of its squeezed form.
fn letter_of(payload: &[u8]) -> char {
    if squeeze::is_squeezed(payload) { payload[1] as char } else { '-' }
}

/// `squeezed.json` holds huHs locked by the Rust build that introduced each
/// squeezed form: unsqueezed, long (`L`), short (`S`-`V`) and word list (`W`-`Z`).
/// Whatever changes later, these have to keep unlocking. (They are in the old
/// envelope, which is what huHs were sealed in at the time.)
#[test]
fn squeezed_huhs_stay_readable() {
    let v: Value = serde_json::from_str(include_str!("squeezed.json")).unwrap();
    let mut seen = String::new();
    for case in v["huhs"].as_array().unwrap() {
        let (name, pass, locked) = (s(case, "name"), s(case, "pass"), s(case, "huh"));
        check_unlocked(case, unlock(locked, Key::Password(pass)).unwrap_or_else(|e| panic!("{name}: {e:?}")));

        // And it really is in the form it is here to pin.
        let payload = cipher::open(&huh::decode(locked).unwrap(), pass.as_bytes()).unwrap();
        assert_eq!(letter_of(&payload).to_string(), s(case, "letter"), "{name}");
        seen.push(letter_of(&payload));
    }
    for family in ["-", "L", "STUV", "WXYZ"] {
        assert!(seen.chars().any(|c| family.contains(c)), "no fixture in the {family} forms");
    }
}

/// `enveloped.json` holds huHs locked by the build that introduced today's
/// envelope (AES-256-GCM): with a password and in a session, text and files,
/// squeezed and not. This is the format people's huHs are in from here on, so
/// these have to keep unlocking too. (When they were made they were also opened
/// with Node's own PBKDF2, HKDF and AES-256-GCM, and a huH decoder written from
/// the format's description.)
#[test]
fn enveloped_huhs_stay_readable() {
    let v: Value = serde_json::from_str(include_str!("enveloped.json")).unwrap();
    let secret: kem::Secret = bytes(&v, "session").try_into().unwrap();
    let other: kem::Secret = [0x5a; 32];
    let (mut with_password, mut in_session, mut seen) = (0, 0, String::new());
    for case in v["huhs"].as_array().unwrap() {
        let (name, locked) = (s(case, "name"), s(case, "huh"));
        let sealed = locker::read(locked).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let bytes = huh::decode(locked).unwrap();
        // The opened huH, and what was inside its envelope (still squeezed, if it was).
        let (opened, payload) = match (s(case, "with"), sealed.needs()) {
            ("password", Needs::Password { salt, rounds }) => {
                assert_eq!(rounds, 600_000, "{name}");
                assert_eq!(locker::glance(locked), Ok(Some(envelope::Kind::Password)), "{name}");
                // A session is not what opens it, and it says what would.
                assert_eq!(unlock(locked, Key::Session(&secret)), Err(locker::UnlockError::NeedsPassword), "{name}");
                // Kind, salt, key check and tag around a whole number of 16-byte steps.
                assert_eq!((bytes.len() - 1 - 16 - 8 - 16) % 16, 0, "{name}");
                with_password += 1;
                // (Stretched once, for both.)
                let stretched = envelope::stretch(s(case, "pass"), &salt);
                (sealed.open_stretched(&stretched), envelope::open_password(&bytes, &stretched))
            }
            ("session", Needs::Session) => {
                assert_eq!(locker::glance(locked), Ok(Some(envelope::Kind::Session)), "{name}");
                assert_eq!(unlock(locked, Key::Session(&other)), Err(locker::UnlockError::WrongKey), "{name}");
                assert_eq!(unlock(locked, Key::Password("a password")), Err(locker::UnlockError::NeedsSession), "{name}");
                // Kind, salt and tag around a whole number of 16-byte steps.
                assert_eq!((bytes.len() - 1 - 16 - 16) % 16, 0, "{name}");
                in_session += 1;
                (sealed.open_session(&secret), envelope::open_session(&bytes, &secret))
            }
            (with, needs) => panic!("{name}: locked with {with}, but it asks for {needs:?}"),
        };
        let opened = opened.unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(!opened.is_old(), "{name}");
        check_unlocked(case, opened.finish().unwrap_or_else(|e| panic!("{name}: {e:?}")));

        // And it really is in the form it is here to pin.
        let payload = payload.unwrap_or_else(|| panic!("{name}: the envelope didn't open"));
        assert_eq!(letter_of(&payload).to_string(), s(case, "letter"), "{name}");
        seen.push(letter_of(&payload));
    }
    assert!(with_password >= 2 && in_session >= 2);
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
