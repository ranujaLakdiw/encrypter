//! Remembering a session on this device.
//!
//! A paired session is nothing but its 32-byte secret. To keep one between visits
//! the secret is sealed under a passphrase, and the sealed form is all that is ever
//! stored: PBKDF2-HMAC-SHA256 stretches the passphrase into a key (600,000 rounds
//! of it, so guessing passphrases against a stolen copy is slow going), and
//! AES-256-GCM locks the secret under that key. The nickname it is filed under is
//! authenticated along with it, so a sealed session opens only with its own
//! passphrase and only under its own name.
//!
//! Sealed layout (then base64):
//! `"huK1" · rounds (u32 LE) · 16-byte salt · 12-byte nonce · the secret,
//! encrypted (32 bytes) · 16-byte tag`.
//! The number of rounds travels with it, so it can be raised one day without
//! stranding what was sealed before.
//!
//! Stretching is the slow part by design, and it is a separate step ([`Recipe`],
//! then [`stretch`]) so the page can hand it to the browser's own crypto, which
//! computes the very same function many times faster. That matters most where
//! this program runs translated to JavaScript, with no WebAssembly to lean on.

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm,
};
use crate::{
    b64, kdf,
    kem::{Secret, SECRET_LEN},
    rng,
};

const MAGIC: &[u8; 4] = b"huK1";
const SALT: usize = 16;
const NONCE: usize = 12;
const TAG: usize = 16;
/// Everything ahead of the nonce: the magic, the rounds and the salt.
const HEAD: usize = MAGIC.len() + 4 + SALT;
const SEALED: usize = HEAD + NONCE + SECRET_LEN + TAG;

/// The longest nickname, in characters.
pub const NICK_MAX: usize = 32;
/// The shortest passphrase a session can be sealed under, in characters.
pub const PASS_MIN: usize = 8;

/// How many rounds of PBKDF2 sessions are sealed with today (OWASP's figure for
/// HMAC-SHA256).
pub const ROUNDS: u32 = 600_000;

/// The most a sealed session may ask for. Storage can be edited by hand; this
/// keeps a doctored entry from tying the page up for good.
const MOST_ROUNDS: u32 = 10_000_000;

pub type Key = [u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeepError {
    /// Not a sealed session, or not a whole one.
    Damaged,
    /// It didn't open: the wrong passphrase (or it was altered, or filed under
    /// another name; from here those look the same).
    WrongPass,
}

/// A remembered session as it is stored: its nickname, when it was saved
/// (milliseconds since 1970) and the sealed secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    pub nick: String,
    pub saved: u64,
    pub sealed: String,
}

/// What a passphrase is stretched with to make one sealed session's key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub rounds: u32,
    pub salt: [u8; SALT],
}

impl Recipe {
    /// For sealing something new: today's rounds and a salt of its own.
    pub fn fresh() -> Self {
        let mut salt = [0u8; SALT];
        rng::fill(&mut salt);
        Self { rounds: ROUNDS, salt }
    }

    /// The one a sealed session was made with, read off the front of it.
    pub fn of(sealed: &str) -> Result<Self, KeepError> {
        parts(&b64::decode(sealed).ok_or(KeepError::Damaged)?).map(|(recipe, ..)| recipe)
    }
}

/// Stretches a passphrase into the key: PBKDF2-HMAC-SHA256, 32 bytes out. The
/// browser's `crypto.subtle` computes the same thing, and faster.
pub fn stretch(pass: &str, recipe: &Recipe) -> Key {
    kdf::pbkdf2(pass, &recipe.salt, recipe.rounds)
}

/// Takes a sealed session apart: its recipe, its head, its nonce and what is locked.
fn parts(data: &[u8]) -> Result<(Recipe, &[u8], [u8; NONCE], &[u8]), KeepError> {
    if data.len() != SEALED || &data[..MAGIC.len()] != MAGIC {
        return Err(KeepError::Damaged);
    }
    let (head, rest) = data.split_at(HEAD);
    let (nonce, locked) = rest.split_at(NONCE);
    let rounds = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    if rounds == 0 || rounds > MOST_ROUNDS {
        return Err(KeepError::Damaged);
    }
    let salt = head[HEAD - SALT..].try_into().map_err(|_| KeepError::Damaged)?;
    let nonce = nonce.try_into().map_err(|_| KeepError::Damaged)?;
    Ok((Recipe { rounds, salt }, head, nonce, locked))
}

/// What is authenticated but not encrypted: the head and the nickname.
fn bound(head: &[u8], nick: &str) -> Vec<u8> {
    [head, nick.as_bytes()].concat()
}

fn seal_with(secret: &Secret, nick: &str, recipe: &Recipe, key: &Key, nonce: [u8; NONCE]) -> String {
    let mut out = Vec::with_capacity(SEALED);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&recipe.rounds.to_le_bytes());
    out.extend_from_slice(&recipe.salt);
    let locked = Aes256Gcm::new(&(*key).into())
        .encrypt(&nonce.into(), Payload { msg: secret, aad: &bound(&out, nick) })
        .expect("thirty-two bytes are never too many for AES-GCM");
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&locked);
    b64::encode(&out)
}

/// Seals a session secret, to be filed under `nick`. `key` is the passphrase
/// stretched with `recipe`, which goes along in the result. A fresh recipe each
/// time (and a nonce drawn here) means sealing the same thing twice never gives
/// the same result.
pub fn seal(secret: &Secret, nick: &str, recipe: &Recipe, key: &Key) -> String {
    let mut nonce = [0u8; NONCE];
    rng::fill(&mut nonce);
    seal_with(secret, nick, recipe, key, nonce)
}

/// Opens a sealed session, given the nickname it was filed under and its
/// passphrase stretched with its own recipe ([`Recipe::of`]).
pub fn open(sealed: &str, nick: &str, key: &Key) -> Result<Secret, KeepError> {
    let data = b64::decode(sealed).ok_or(KeepError::Damaged)?;
    let (_, head, nonce, locked) = parts(&data)?;
    let secret = Aes256Gcm::new(&(*key).into())
        .decrypt(&nonce.into(), Payload { msg: locked, aad: &bound(head, nick) })
        .map_err(|_| KeepError::WrongPass)?;
    secret.try_into().map_err(|_| KeepError::Damaged)
}

/// A nickname as it will be kept: no stray white space at the ends, single
/// spaces inside, nothing unprintable. `None` if that leaves nothing, or too much.
pub fn tidy_nick(raw: &str) -> Option<String> {
    let printable: String = raw.chars().filter(|c| c.is_whitespace() || !c.is_control()).collect();
    let nick = printable.split_whitespace().collect::<Vec<_>>().join(" ");
    (!nick.is_empty() && nick.chars().count() <= NICK_MAX).then_some(nick)
}

/// Whether two nicknames would be mistaken for each other ("Neja" and "neja").
pub fn same_nick(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// The remembered sessions as the text that goes into storage.
pub fn list_to_json(list: &[Kept]) -> String {
    let items = list
        .iter()
        .map(|kept| serde_json::json!({ "nick": kept.nick, "saved": kept.saved, "sealed": kept.sealed }))
        .collect();
    serde_json::Value::Array(items).to_string()
}

/// Reads the remembered sessions back out of storage. Anything that isn't one
/// (storage can be edited by hand) is left out rather than trusted, and so is a
/// second entry under a nickname already seen: a nickname names one session.
pub fn list_from_json(json: &str) -> Vec<Kept> {
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str(json) else {
        return Vec::new();
    };
    let mut list: Vec<Kept> = Vec::new();
    for item in &items {
        let (Some(nick), Some(sealed)) = (
            item.get("nick").and_then(serde_json::Value::as_str),
            item.get("sealed").and_then(serde_json::Value::as_str),
        ) else {
            continue;
        };
        if tidy_nick(nick).as_deref() != Some(nick) || list.iter().any(|kept| same_nick(&kept.nick, nick)) {
            continue;
        }
        list.push(Kept {
            nick: nick.to_owned(),
            saved: item.get("saved").and_then(serde_json::Value::as_u64).unwrap_or(0),
            sealed: sealed.to_owned(),
        });
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret() -> Secret {
        core::array::from_fn(|i| (i * 7 + 3) as u8)
    }

    /// A recipe light enough to run many times in a test.
    fn light() -> Recipe {
        Recipe { rounds: 3, ..Recipe::fresh() }
    }

    fn sealed_lightly(nick: &str, pass: &str) -> String {
        let recipe = light();
        seal(&secret(), nick, &recipe, &stretch(pass, &recipe))
    }

    /// The whole way round, as the page does it: read the recipe, stretch, open.
    fn open_with_pass(sealed: &str, nick: &str, pass: &str) -> Result<Secret, KeepError> {
        open(sealed, nick, &stretch(pass, &Recipe::of(sealed)?))
    }

    #[test]
    fn opens_with_its_passphrase_and_name_only() {
        let sealed = sealed_lightly("Neja", "correct horse");
        assert_eq!(open_with_pass(&sealed, "Neja", "correct horse"), Ok(secret()));
        assert_eq!(open_with_pass(&sealed, "Neja", "correct horsf"), Err(KeepError::WrongPass));
        assert_eq!(open_with_pass(&sealed, "Neja", ""), Err(KeepError::WrongPass));
        // Filed under another name, it is not the same thing.
        assert_eq!(open_with_pass(&sealed, "neja", "correct horse"), Err(KeepError::WrongPass));
    }

    #[test]
    fn never_seals_the_same_way_twice() {
        let (a, b) = (sealed_lightly("n", "passphrase"), sealed_lightly("n", "passphrase"));
        assert_ne!(a, b);
        assert_ne!(Recipe::of(&a).unwrap().salt, Recipe::of(&b).unwrap().salt);
        // Even with everything else the same, the nonce is new.
        let recipe = light();
        let key = stretch("passphrase", &recipe);
        assert_ne!(seal(&secret(), "n", &recipe, &key), seal(&secret(), "n", &recipe, &key));
        // And the secret is nowhere in it.
        let raw = b64::decode(&a).unwrap();
        assert_eq!(raw.len(), SEALED);
        assert!(!raw.windows(8).any(|w| secret().windows(8).any(|s| s == w)));
    }

    #[test]
    fn any_change_stops_it_opening() {
        let sealed = sealed_lightly("Neja", "correct horse");
        let raw = b64::decode(&sealed).unwrap();
        // The rounds, the salt, the nonce, the locked secret and the tag: flip a bit in each.
        for at in [4, HEAD - 1, HEAD, HEAD + NONCE, SEALED - 1] {
            let mut changed = raw.clone();
            changed[at] ^= 1;
            assert_eq!(
                open_with_pass(&b64::encode(&changed), "Neja", "correct horse"),
                Err(KeepError::WrongPass),
                "byte {at}"
            );
        }
        // Even under the right key, a head that was touched is refused: fewer
        // rounds can't be passed off on something sealed with more.
        let key = stretch("correct horse", &Recipe::of(&sealed).unwrap());
        let mut changed = raw.clone();
        changed[4] ^= 1;
        assert_eq!(open(&b64::encode(&changed), "Neja", &key), Err(KeepError::WrongPass));
        assert_eq!(open(&sealed, "Neja", &key), Ok(secret()));
    }

    #[test]
    fn refuses_what_is_not_a_sealed_session() {
        let raw = b64::decode(&sealed_lightly("Neja", "correct horse")).unwrap();
        let recipe_of = |raw: &[u8]| Recipe::of(&b64::encode(raw));
        assert!(recipe_of(&raw).is_ok());

        assert_eq!(Recipe::of(""), Err(KeepError::Damaged));
        assert_eq!(Recipe::of("not base64 !!"), Err(KeepError::Damaged));
        assert_eq!(recipe_of(&raw[..SEALED - 1]), Err(KeepError::Damaged));
        assert_eq!(recipe_of(&[&raw[..], &[0]].concat()), Err(KeepError::Damaged));

        let mut other_magic = raw.clone();
        other_magic[3] = b'2';
        assert_eq!(recipe_of(&other_magic), Err(KeepError::Damaged));

        // More rounds than any page should be asked to run, and none at all.
        for rounds in [MOST_ROUNDS + 1, u32::MAX, 0] {
            let mut doctored = raw.clone();
            doctored[4..8].copy_from_slice(&rounds.to_le_bytes());
            assert_eq!(recipe_of(&doctored), Err(KeepError::Damaged), "{rounds} rounds");
            assert_eq!(open(&b64::encode(&doctored), "Neja", &[0; 32]), Err(KeepError::Damaged));
        }
        let mut most = raw.clone();
        most[4..8].copy_from_slice(&MOST_ROUNDS.to_le_bytes());
        assert_eq!(recipe_of(&most).map(|r| r.rounds), Ok(MOST_ROUNDS));
    }

    /// A session sealed by the first version of this, at the real number of
    /// rounds. What people have remembered must keep opening, so this must never
    /// change. (Opened, when it was made, with Node's and OpenSSL's own PBKDF2 and
    /// AES-256-GCM: nothing about the format belongs to this code alone.)
    #[test]
    fn what_was_sealed_before_still_opens() {
        const SEALED_V1: &str = "aHVLMcAnCQAAAQIDBAUGBwgJCgsMDQ4PoKGio6SlpqeoqaqrvopDkrYPK4jtZV9D/Z3OnBm77pDLM53xFN3T4aqpOZ54HjYQkBlTtOLDDims05+3";
        let recipe = Recipe { rounds: ROUNDS, salt: core::array::from_fn(|i| i as u8) };
        assert_eq!(ROUNDS, 600_000);
        let key = stretch("open sesame, huh", &recipe);
        let nonce = core::array::from_fn(|i| 0xA0 + i as u8);
        assert_eq!(seal_with(&secret(), "Neja 💖", &recipe, &key, nonce), SEALED_V1);

        assert_eq!(Recipe::of(SEALED_V1), Ok(recipe));
        assert_eq!(open(SEALED_V1, "Neja 💖", &key), Ok(secret()));
        let raw = b64::decode(SEALED_V1).unwrap();
        assert_eq!(raw[..8], *b"huK1\xC0\x27\x09\x00"); // 600,000 rounds
    }

    #[test]
    fn stretches_as_the_standard_says() {
        // The page asks the browser for this same key, so it has to be plain
        // PBKDF2-HMAC-SHA256. The expected one is what OpenSSL and Node both give
        // for "passwd", the salt "salt" (padded with zeros to 16 bytes), 1 round.
        let mut salt = [0u8; SALT];
        salt[..4].copy_from_slice(b"salt");
        let key = stretch("passwd", &Recipe { rounds: 1, salt });
        assert_eq!(b64::encode(&key), "nqRC6nNgTm2JhCXFTC8uEIvNtguHgfSO481yk/sUj0Y=");
    }

    #[test]
    fn nicknames() {
        assert_eq!(tidy_nick("  Neja  "), Some("Neja".to_owned()));
        assert_eq!(tidy_nick("work \t  pal\n"), Some("work pal".to_owned()));
        assert_eq!(tidy_nick("a\u{0}b\u{7}"), Some("ab".to_owned()));
        assert_eq!(tidy_nick("නේජා 💖"), Some("නේජා 💖".to_owned()));
        assert_eq!(tidy_nick(""), None);
        assert_eq!(tidy_nick(" \n "), None);
        assert_eq!(tidy_nick(&"x".repeat(NICK_MAX)), Some("x".repeat(NICK_MAX)));
        assert_eq!(tidy_nick(&"💖".repeat(NICK_MAX)), Some("💖".repeat(NICK_MAX)));
        assert_eq!(tidy_nick(&"x".repeat(NICK_MAX + 1)), None);

        assert!(same_nick("Neja", "neja"));
        assert!(!same_nick("Neja", "Neja 2"));
    }

    #[test]
    fn the_list_survives_storage() {
        let list = vec![
            Kept { nick: "Neja 💖".into(), saved: 1_795_046_400_000, sealed: "aHVLMQ==".into() },
            Kept { nick: "say \"hi\"".into(), saved: 0, sealed: String::new() },
        ];
        assert_eq!(list_from_json(&list_to_json(&list)), list);
        assert_eq!(list_from_json(&list_to_json(&[])), Vec::new());
    }

    #[test]
    fn a_meddled_list_is_read_with_care() {
        assert_eq!(list_from_json(""), Vec::new());
        assert_eq!(list_from_json("{\"nick\":\"x\"}"), Vec::new());
        assert_eq!(list_from_json("[1, null, \"x\", {}]"), Vec::new());
        let json = r#"[
            {"nick": "ok", "sealed": "abc"},
            {"nick": "no seal"},
            {"nick": 7, "sealed": "abc"},
            {"nick": "  padded ", "sealed": "abc"},
            {"nick": "", "sealed": "abc"},
            {"nick": "dated", "saved": 12, "sealed": "abc", "extra": true},
            {"nick": "OK", "sealed": "a second one by the same name"}
        ]"#;
        assert_eq!(
            list_from_json(json),
            vec![
                Kept { nick: "ok".into(), saved: 0, sealed: "abc".into() },
                Kept { nick: "dated".into(), saved: 12, sealed: "abc".into() },
            ]
        );
    }
}
