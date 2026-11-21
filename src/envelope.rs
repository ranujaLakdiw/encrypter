//! The envelope every huH is sealed in today: AES-256-GCM, under a key made for
//! that one huH and never used again.
//!
//! * in a session: `0x02 · salt (16 bytes) · the payload, padded and encrypted · tag (16 bytes)`
//! * with a password: `0x01 · salt (16 bytes) · key check (8 bytes) · the payload, padded and encrypted · tag (16 bytes)`
//!
//! The first byte is the kind, which says where the key comes from; with the
//! salt it is everything (besides the secret itself) needed to make that key again.
//!
//! * **A session**: the key is HKDF-SHA256 of the session's secret and the salt.
//!   That secret is 32 random bytes already, so there is nothing to stretch.
//! * **A password** is first stretched: PBKDF2-HMAC-SHA256 of the password and
//!   the salt, 600,000 rounds. A password is only as hard to find as it is to
//!   guess, so every guess is made to cost real time. (The old envelope,
//!   `cipher`, asked for one round of MD5, and a laptop could try twenty million
//!   a second.) HKDF-SHA256 then turns what was stretched into two things: the
//!   AES key, and the eight bytes of key check that travel in the head.
//!
//! A salt of its own for every huH means a key of its own for every huH: locking
//! the same thing twice never gives the same result, and no key ever sees two
//! messages. The first twelve bytes of the salt serve as the GCM nonce.
//!
//! **The tag** is what the old envelope never had. Whoever doesn't hold the key
//! can't change a byte of a huH (the head included: it is authenticated along
//! with the payload) and have it still open, nor make one of their own that
//! does. What the tag can't say is *who* made a huH or *when*: both people in a
//! session hold the same secret, so a huH posted a second time, or one's own
//! handed back, opens like any other.
//!
//! **The key check** is there because of what passwords are. AES-GCM by itself
//! doesn't tie a ciphertext to one key: with some algebra, a single forged
//! ciphertext can be made that passes the tag under thousands of keys at once.
//! With keys that come from guessable passwords, that turns anybody who will
//! try a huH and say what happened into a way of testing thousands of guesses
//! with one paste. The check comes out of the same stretching as the key (it is
//! the eight bytes HKDF gives after it), and an envelope opens only under a
//! password whose check is the one in its head. A forgery good for thousands of
//! guesses would need thousands of passwords that share those eight bytes under
//! one salt, and nobody can find them: even two that do would take billions of
//! stretches to come across, to get two arbitrary guesses for the price of one.
//! A session's secret can't be guessed at, so its envelope carries no check.
//!
//! **The padding** is so that a huH's length says little about what is in it.
//! The payload is filled out to the next whole 16 bytes, a whole 16 more if it
//! was there already, each filling byte being how many there are (as PKCS#7 does
//! it; the old envelope's AES-CBC padded the same way). Without it "yes" and
//! "no" would differ by a byte anyone could count.
//!
//! People's huHs are written in this layout from now on, so it must not change.
//! Anything stronger one day gets a kind of its own (those up to `0x0f` are kept
//! for that), which this build answers with "that may be newer than I am".

use aes_gcm::{aead::inout::InOutBuf, AeadInOut, Aes256Gcm, KeyInit};
use zeroize::Zeroize;

use crate::{kdf, kem::Secret, rng};

pub const SALT: usize = 16;
const NONCE: usize = 12;
const TAG: usize = 16;
const CHECK: usize = 8;
/// Payloads are padded to a whole number of these.
const STEP: usize = 16;

/// The last kind kept for envelopes to come.
const KINDS_END: u8 = 0x0f;

/// How many rounds of PBKDF2 a password goes through (OWASP's figure for
/// HMAC-SHA256). It belongs to kind `0x01`: more rounds would be a new kind.
pub const PASSWORD_ROUNDS: u32 = 600_000;

/// The shortest password the page locks with, in characters. Stretching makes
/// each guess slow; it cannot make a short password hard to guess.
pub const PASS_MIN: usize = 8;

/// What HKDF is told each thing it makes is for.
const MESSAGE: &[u8] = b"huH message";
const PASSWORD: &[u8] = b"huH password";

pub type Salt = [u8; SALT];
type Key = kdf::Key;

/// A password after its 600,000 rounds with one huH's salt: what
/// [`stretch`] gives, or the browser's own PBKDF2 doing the same sum.
pub type Stretched = [u8; 32];

/// Where an envelope's key comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Password,
    Session,
}

impl Kind {
    const fn byte(self) -> u8 {
        match self {
            Kind::Password => 0x01,
            Kind::Session => 0x02,
        }
    }

    /// Everything ahead of the encrypted payload: the kind, the salt, and for a
    /// password the key check.
    const fn head(self) -> usize {
        match self {
            Kind::Password => 1 + SALT + CHECK,
            Kind::Session => 1 + SALT,
        }
    }

    /// The least an envelope of this kind can be: its head, one step of padding
    /// around nothing, and the tag.
    const fn least(self) -> usize {
        self.head() + STEP + TAG
    }
}

/// Why some bytes can't be read as an envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// Not one of these: an old huH, or nothing of ours at all.
    NotOne,
    /// It starts the way an envelope of a kind this build has never heard of
    /// would. A newer build may have made it; so may a slip in the first letters.
    Newer,
}

/// The length a payload of `n` bytes is padded to: the next whole 16, and a
/// whole 16 more if it was there already.
pub const fn padded(n: usize) -> usize {
    (n / STEP + 1) * STEP
}

/// How long the envelope around a payload of `n` bytes is.
pub const fn sealed_len(kind: Kind, n: usize) -> usize {
    kind.head() + padded(n) + TAG
}

/// A salt for a new huH.
pub fn fresh_salt() -> Salt {
    let mut salt = [0u8; SALT];
    rng::fill(&mut salt);
    salt
}

/// Stretches a password for the huH with this salt. This is the slow part by
/// design; the page hands it to the browser's own PBKDF2 where it can, which
/// gives the same bytes many times faster.
///
/// `pass` is the password in Unicode's composed form (NFC), which is how the
/// page writes whatever was typed: the same word must make the same key
/// whichever way a keyboard put its accents together. There is no normalising
/// here, so a caller with decomposed text has to compose it first.
pub fn stretch(pass: &str, salt: &Salt) -> Stretched {
    kdf::pbkdf2(pass, salt, PASSWORD_ROUNDS)
}

/// The AES key a stretched password makes, and the check that says which
/// stretched password an envelope is for.
fn password_keys(stretched: &Stretched) -> (Key, [u8; CHECK]) {
    let mut both: [u8; 32 + CHECK] = kdf::hkdf(stretched, &[], PASSWORD);
    let key = both[..32].try_into().expect("the first 32 bytes");
    let check = both[32..].try_into().expect("the last 8 bytes");
    both.zeroize();
    (key, check)
}

/// The AES key a session's secret makes for the huH with this salt.
fn session_key(secret: &Secret, salt: &Salt) -> Key {
    kdf::hkdf(secret, salt, MESSAGE)
}

fn nonce(salt: &[u8]) -> [u8; NONCE] {
    salt[..NONCE].try_into().expect("a salt is longer than a nonce")
}

/// Seals a payload behind `head`, which is authenticated with it.
fn seal(mut out: Vec<u8>, salt: &Salt, mut key: Key, payload: &[u8]) -> Vec<u8> {
    let head = out.len();
    let fill = padded(payload.len()) - payload.len();
    out.reserve_exact(padded(payload.len()) + TAG);
    out.extend_from_slice(payload);
    out.resize(out.len() + fill, fill as u8);
    let (ahead, body) = out.split_at_mut(head);
    let tag = Aes256Gcm::new(&key.into())
        .encrypt_inout_detached(&nonce(salt).into(), ahead, InOutBuf::from(body))
        .expect("nothing this page handles is too long for AES-GCM");
    key.zeroize();
    out.extend_from_slice(&tag);
    out
}

/// Seals a payload under a password already stretched with `salt` ([`stretch`]).
/// `salt` has to be one that was never used before ([`fresh_salt`]).
pub fn seal_password(salt: Salt, stretched: &Stretched, payload: &[u8]) -> Vec<u8> {
    let (key, check) = password_keys(stretched);
    let mut head = Vec::with_capacity(sealed_len(Kind::Password, payload.len()));
    head.push(Kind::Password.byte());
    head.extend_from_slice(&salt);
    head.extend_from_slice(&check);
    seal(head, &salt, key, payload)
}

/// Seals a payload in a session. `salt` has to be one that was never used
/// before ([`fresh_salt`]).
pub fn seal_session(salt: Salt, secret: &Secret, payload: &[u8]) -> Vec<u8> {
    let mut head = Vec::with_capacity(sealed_len(Kind::Session, payload.len()));
    head.push(Kind::Session.byte());
    head.extend_from_slice(&salt);
    seal(head, &salt, session_key(secret, &salt), payload)
}

/// What the first byte of something says it is, given how long the whole is.
pub fn kind_of(first: u8, len: usize) -> Result<Kind, Unreadable> {
    let kind = match first {
        0x01 => Kind::Password,
        0x02 => Kind::Session,
        // Too short to be anything is not "newer", whatever it starts with.
        0x03..=KINDS_END if len >= Kind::Session.least() => return Err(Unreadable::Newer),
        _ => return Err(Unreadable::NotOne),
    };
    if len < kind.least() {
        return Err(Unreadable::NotOne);
    }
    Ok(kind)
}

/// The kind and the salt of an envelope, read off the front of it.
pub fn read(sealed: &[u8]) -> Result<(Kind, Salt), Unreadable> {
    let kind = kind_of(*sealed.first().ok_or(Unreadable::NotOne)?, sealed.len())?;
    let salt = sealed[1..1 + SALT].try_into().expect("the head holds a whole salt");
    Ok((kind, salt))
}

/// Opens what is behind an envelope's head with the AES key, and takes the
/// padding off.
fn open(sealed: &[u8], kind: Kind, mut key: Key) -> Option<Vec<u8>> {
    let (head, rest) = sealed.split_at(kind.head());
    let (body, tag) = rest.split_at(rest.len() - TAG);
    // A whole number of steps, and at least the one that padding always adds.
    if body.is_empty() || body.len() % STEP != 0 {
        return None;
    }
    let tag: [u8; TAG] = tag.try_into().ok()?;
    let mut payload = body.to_vec();
    let opened = Aes256Gcm::new(&key.into()).decrypt_inout_detached(
        &nonce(&head[1..]).into(),
        head,
        InOutBuf::from(&mut payload[..]),
        &tag.into(),
    );
    key.zeroize();
    opened.ok()?;

    // Whoever sealed it held the key, so this only fails for something sealed wrongly.
    let fill = *payload.last()? as usize;
    if fill == 0 || fill > STEP || payload[payload.len() - fill..].iter().any(|&b| b as usize != fill) {
        return None;
    }
    payload.truncate(payload.len() - fill);
    Some(payload)
}

/// Opens a password envelope with the password stretched with its salt
/// ([`read`] gives the salt, [`stretch`] does the stretching). `None` for a
/// wrong password, for anything that was changed after it was sealed, and for
/// anything that isn't a password envelope.
pub fn open_password(sealed: &[u8], stretched: &Stretched) -> Option<Vec<u8>> {
    if !matches!(read(sealed), Ok((Kind::Password, _))) {
        return None;
    }
    let (key, check) = password_keys(stretched);
    // Not a password this envelope was sealed under: no need to try the lock.
    // (Compared without stopping at the first byte that differs, so that how
    // long it takes says nothing about how nearly a guess's check matched.)
    let differing = sealed[1 + SALT..Kind::Password.head()].iter().zip(check).fold(0u8, |all, (a, b)| all | (a ^ b));
    if differing != 0 {
        return None;
    }
    open(sealed, Kind::Password, key)
}

/// Opens a session envelope with the session's secret. `None` for another
/// session's, for anything that was changed after it was sealed, and for
/// anything that isn't a session envelope.
pub fn open_session(sealed: &[u8], secret: &Secret) -> Option<Vec<u8>> {
    let Ok((Kind::Session, salt)) = read(sealed) else { return None };
    open(sealed, Kind::Session, session_key(secret, &salt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::b64;

    fn secret() -> Secret {
        core::array::from_fn(|i| (i * 7 + 3) as u8)
    }

    fn fixed_salt() -> Salt {
        core::array::from_fn(|i| 0xA0 + i as u8)
    }

    /// Stands in for a stretched password where the stretching itself isn't
    /// what is being tested (600,000 rounds a time adds up).
    fn stretched() -> Stretched {
        core::array::from_fn(|i| (i * 5 + 1) as u8)
    }

    #[test]
    fn round_trip_at_every_size() {
        for len in [0usize, 1, 15, 16, 17, 31, 32, 33, 1000, 70_000] {
            let payload: Vec<u8> = (0..len).map(|i| (i * 31) as u8).collect();

            let salt = fresh_salt();
            let sealed = seal_password(salt, &stretched(), &payload);
            assert_eq!(sealed.len(), sealed_len(Kind::Password, len));
            assert_eq!(sealed.len(), 1 + 16 + 8 + padded(len) + 16);
            assert_eq!(read(&sealed), Ok((Kind::Password, salt)));
            assert_eq!(open_password(&sealed, &stretched()), Some(payload.clone()), "password {len}");

            let salt = fresh_salt();
            let sealed = seal_session(salt, &secret(), &payload);
            assert_eq!(sealed.len(), sealed_len(Kind::Session, len));
            assert_eq!(sealed.len(), 1 + 16 + padded(len) + 16);
            assert_eq!(read(&sealed), Ok((Kind::Session, salt)));
            assert_eq!(open_session(&sealed, &secret()), Some(payload.clone()), "session {len}");
        }
    }

    #[test]
    fn padded_to_whole_steps() {
        assert_eq!([0, 1, 15, 16, 17, 31, 32].map(padded), [16, 16, 16, 32, 32, 32, 48]);
        // Short answers all come to the same length: nothing to count.
        let lengths: Vec<usize> = ["y", "no", "yes", "nope", "I love you", "I hate you"]
            .iter()
            .map(|answer| seal_session(fresh_salt(), &secret(), answer.as_bytes()).len())
            .collect();
        assert!(lengths.iter().all(|&len| len == lengths[0]), "{lengths:?}");
    }

    #[test]
    fn only_its_own_key_opens_it() {
        let sealed = seal_session(fresh_salt(), &secret(), b"between us");
        assert_eq!(open_session(&sealed, &secret()).as_deref(), Some(&b"between us"[..]));
        let mut other = secret();
        other[31] ^= 1;
        assert_eq!(open_session(&sealed, &other), None);
        // A session's envelope is not a password's, whatever is offered.
        assert_eq!(open_password(&sealed, &secret()), None);

        let sealed = seal_password(fresh_salt(), &stretched(), b"between us");
        assert_eq!(open_password(&sealed, &stretched()).as_deref(), Some(&b"between us"[..]));
        let mut other = stretched();
        other[0] ^= 1;
        assert_eq!(open_password(&sealed, &other), None);
        assert_eq!(open_session(&sealed, &stretched()), None);
    }

    #[test]
    fn never_seals_the_same_way_twice() {
        let (a, b) = (
            seal_session(fresh_salt(), &secret(), b"the very same words"),
            seal_session(fresh_salt(), &secret(), b"the very same words"),
        );
        assert_ne!(a[1..17], b[1..17]);
        // Not one byte of the encrypted part or the tag in common is too much to
        // ask of chance, but the two must not share a run of them.
        assert!(!a[17..].windows(8).any(|w| b[17..].windows(8).any(|v| v == w)));
        // And what was sealed is nowhere in it.
        assert!(!a.windows(8).any(|w| w == b"the very"));

        // The same password under another salt is stretched to something else,
        // so the key check gives nothing away from one huH to the next either.
        let (a, b) = (
            seal_password(fresh_salt(), &stretch_lightly("pw", 1), b"x"),
            seal_password(fresh_salt(), &stretch_lightly("pw", 2), b"x"),
        );
        assert_ne!(a[17..25], b[17..25]);
    }

    /// A stand-in for [`stretch`] that is quick: what matters to these tests is
    /// only that different salts give different results.
    fn stretch_lightly(pass: &str, salt: u8) -> Stretched {
        kdf::pbkdf2(pass, &[salt; SALT], 3)
    }

    #[test]
    fn any_change_stops_it_opening() {
        let payload = b"pay Neja back the forty you owe her".to_vec();
        for password in [true, false] {
            let sealed = if password {
                seal_password(fresh_salt(), &stretched(), &payload)
            } else {
                seal_session(fresh_salt(), &secret(), &payload)
            };
            let opened = |sealed: &[u8]| {
                if password {
                    open_password(sealed, &stretched())
                } else {
                    open_session(sealed, &secret())
                }
            };
            assert_eq!(opened(&sealed), Some(payload.clone()));

            // Every single bit: the kind, the salt, the key check, the encrypted
            // payload and its padding, the tag.
            for at in 0..sealed.len() {
                for bit in 0..8 {
                    let mut changed = sealed.clone();
                    changed[at] ^= 1 << bit;
                    assert_eq!(opened(&changed), None, "bit {bit} of byte {at}");
                }
            }
            // Cut short anywhere, or with something added.
            for keep in 0..sealed.len() {
                assert_eq!(opened(&sealed[..keep]), None, "cut to {keep}");
            }
            for extra in [1usize, 15, 16, 17] {
                assert_eq!(opened(&[&sealed[..], &vec![0u8; extra][..]].concat()), None, "{extra} more");
            }
        }
    }

    #[test]
    fn the_key_check_names_one_stretched_password() {
        let sealed = seal_password(fixed_salt(), &stretched(), b"which password is this for?");
        let (_, check) = password_keys(&stretched());
        assert_eq!(sealed[17..25], check);
        // It says nothing about the AES key itself.
        let (key, _) = password_keys(&stretched());
        assert!(!key.windows(4).any(|w| check.windows(4).any(|c| c == w)));

        // A password that isn't the one is turned away by the check alone,
        // before the lock is tried: with the check made to match, the same
        // wrong key gets as far as the tag, and no further.
        let mut other = stretched();
        other[5] ^= 0x40;
        assert_ne!(password_keys(&other).1, check);
        assert_eq!(open_password(&sealed, &other), None);
        let mut forged = sealed.clone();
        forged[17..25].copy_from_slice(&password_keys(&other).1);
        assert_eq!(open_password(&forged, &other), None);
        // And the right password no longer opens it once the check is not its own.
        assert_eq!(open_password(&forged, &stretched()), None);
    }

    #[test]
    fn padding_that_is_wrong_is_refused() {
        // Sealed properly in every other way: the right key, a good tag, but a
        // payload whose padding is not padding. (Only someone holding the key
        // could make one; it must still not open as anything.)
        let salt = fresh_salt();
        let seal_raw = |body: &[u8]| {
            let mut out = vec![Kind::Session.byte()];
            out.extend_from_slice(&salt);
            out.extend_from_slice(body);
            let (head, body) = out.split_at_mut(17);
            let tag = Aes256Gcm::new(&session_key(&secret(), &salt).into())
                .encrypt_inout_detached(&nonce(&salt).into(), head, InOutBuf::from(body))
                .unwrap();
            out.extend_from_slice(&tag);
            out
        };
        let good: Vec<u8> = [&b"twelve bytes"[..], &[4; 4]].concat();
        assert_eq!(open_session(&seal_raw(&good), &secret()).as_deref(), Some(&b"twelve bytes"[..]));
        for bad in [
            [&b"twelve bytes"[..], &[0, 0, 0, 0]].concat(),  // ends in zero
            [&b"twelve bytes"[..], &[4, 4, 3, 4]].concat(),  // not all the same
            [&b"twelve bytes"[..], &[17; 4]].concat(),       // more than a step
            [&b"twelve bytes ..."[..], &[32; 16]].concat(),  // more than a step, twice over
            vec![16; 15],                                    // not a whole step
            Vec::new(),                                      // nothing at all
        ] {
            assert_eq!(open_session(&seal_raw(&bad), &secret()), None, "{bad:?}");
        }
        // A whole step of padding around nothing is the empty payload.
        assert_eq!(open_session(&seal_raw(&[16; 16]), &secret()), Some(Vec::new()));
    }

    #[test]
    fn what_is_not_an_envelope() {
        let sealed = seal_session(fresh_salt(), &secret(), b"x");
        assert_eq!(sealed.len(), Kind::Session.least());
        assert!(read(&sealed).is_ok());
        assert_eq!(read(&[]), Err(Unreadable::NotOne));
        assert_eq!(read(&sealed[..sealed.len() - 1]), Err(Unreadable::NotOne));
        // A password envelope is longer at its least, by the key check.
        let as_password: Vec<u8> = [&[0x01][..], &sealed[1..]].concat();
        assert_eq!(read(&as_password), Err(Unreadable::NotOne));
        assert_eq!(read(&[&as_password[..], &[0; 8]].concat()).map(|(kind, _)| kind), Ok(Kind::Password));
        // The old envelope starts with "Salted__".
        assert_eq!(
            read(&crate::cipher::seal(b"an old huH, long enough to count as one of these", b"pw")),
            Err(Unreadable::NotOne)
        );

        let mut odd = sealed.clone();
        for (first, want) in [
            (0x00, Unreadable::NotOne),
            (0x03, Unreadable::Newer),
            (KINDS_END, Unreadable::Newer),
            (KINDS_END + 1, Unreadable::NotOne),
            (b'S', Unreadable::NotOne),
            (0xff, Unreadable::NotOne),
        ] {
            odd[0] = first;
            assert_eq!(read(&odd), Err(want), "first byte {first:#04x}");
            assert_eq!(open_session(&odd, &secret()), None);
            assert_eq!(open_password(&odd, &stretched()), None);
        }
        // Too short to be anything is not "newer", whatever it starts with.
        assert_eq!(read(&[0x03; 48]), Err(Unreadable::NotOne));
        assert_eq!(kind_of(0x03, 49), Err(Unreadable::Newer));
    }

    /// Envelopes sealed by the first version of this, one of each kind, the
    /// password one at the real number of rounds. What people lock from here on
    /// has to keep opening, so these must never change. (They were made by Node's
    /// own PBKDF2, HKDF and AES-256-GCM before this code had run once: nothing
    /// about the layout belongs to this program alone.)
    #[test]
    fn what_was_sealed_before_still_opens() {
        const PASSWORD_V1: &str = "AaChoqOkpaanqKmqq6ytrq9dvWMpieh3OR0KG8josn6Yx4xUzc9qSVwFxu+hTMiazXxkIdkbYUas+mYvYRdWHICMv3FP1ETR9A==";
        const SESSION_V1: &str = "AqChoqOkpaanqKmqq6ytrq9z9och3GrapofVleXs73eVqTyLqejQSCK0By7H18qUL6aZPEztQgRFc2gS865CtcU=";
        let payload = "huH, forever \u{1f438}".as_bytes();
        assert_eq!(PASSWORD_ROUNDS, 600_000);

        let stretched = stretch("open sesame, huh", &fixed_salt());
        assert_eq!(b64::encode(&seal_password(fixed_salt(), &stretched, payload)), PASSWORD_V1);
        let sealed = b64::decode(PASSWORD_V1).unwrap();
        assert_eq!(read(&sealed), Ok((Kind::Password, fixed_salt())));
        assert_eq!(open_password(&sealed, &stretched).as_deref(), Some(payload));
        assert_eq!(sealed[0], 0x01);

        assert_eq!(b64::encode(&seal_session(fixed_salt(), &secret(), payload)), SESSION_V1);
        let sealed = b64::decode(SESSION_V1).unwrap();
        assert_eq!(read(&sealed), Ok((Kind::Session, fixed_salt())));
        assert_eq!(open_session(&sealed, &secret()).as_deref(), Some(payload));
        assert_eq!(sealed[0], 0x02);
    }
}
