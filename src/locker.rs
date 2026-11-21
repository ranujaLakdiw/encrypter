//! Locking and unlocking, end to end: text or a file in, huH out, and back.
//!
//! Lock: payload → squeeze (LZMA2, when it helps) → envelope (AES-256-GCM) → huH.
//! Unlock reads everything the page has ever produced: today's huH, huHs in the
//! old envelope (AES-CBC with no check on it, see `cipher`), the first huH
//! format, and raw CryptoJS base64 from before huH existed. Nothing is locked the
//! old way any more.
//!
//! Each direction comes in two steps, so the page can show which one it is on:
//! locking is [`prepare_text`] / [`prepare_file`] (compress) and then one of
//! [`Prepared`]'s seals (encrypt); unlocking is [`read`] with one of [`Sealed`]'s
//! opens (decrypt) and then [`Opened::finish`] (decompress). [`lock_text`],
//! [`lock_file`] and [`unlock`] do all of it at once.
//!
//! A password has to be stretched before it locks or opens anything, and that is
//! slow on purpose. The page gives the stretching to the browser's own crypto,
//! so the seal and the open made for it take the password already stretched:
//! [`Prepared::seal_stretched`], [`Sealed::open_stretched`].

use core::fmt;

use crate::{
    b64, cipher,
    envelope::{self, Kind, Salt, Stretched, Unreadable},
    huh,
    kem::Secret,
    payload, squeeze,
};

/// Raw CryptoJS output (the pre-huH format) always starts with "Salted__" in base64.
const RAW_START: &str = "U2FsdGVk";

/// What a huH is locked with, or offered to open it.
#[derive(Clone, Copy)]
pub enum Key<'a> {
    /// A password both people know. For today's huHs it has to be in Unicode's
    /// composed form (see [`envelope::stretch`]); an old huH takes it exactly
    /// as it was typed.
    Password(&'a str),
    /// The secret of a paired session.
    Session(&'a Secret),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Unlocked {
    Text(String),
    File { name: String, mime: String, bytes: Vec<u8> },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnlockError {
    /// Not a huH at all.
    NotHuh,
    /// It didn't open: the wrong password or session, or a huH that is not
    /// exactly as it was made.
    WrongKey,
    /// It opened, so whoever locked it held the key, but what they locked into
    /// it is not a message or a file. (In an old huH, which checks nothing, it
    /// can also be damage.)
    Damaged,
    /// It starts the way a huH from a newer version of the page would. A slip
    /// in its first letters can look the same.
    Newer,
    /// It was locked in a session, and a password was offered.
    NeedsSession,
    /// It was locked with a password, and a session was offered.
    NeedsPassword,
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
pub struct Prepared(Vec<u8>);

impl Prepared {
    /// Whether squeezing helped (a two-word note or a JPEG goes in as it is).
    pub fn is_squeezed(&self) -> bool {
        squeeze::is_squeezed(&self.0)
    }

    /// Encrypts under a password and writes the result as huH. `stretched` is
    /// the password already stretched with `salt` ([`envelope::stretch`], or the
    /// browser's PBKDF2 doing the same sum), and `salt` a new one
    /// ([`envelope::fresh_salt`]).
    pub fn seal_stretched(self, salt: Salt, stretched: &Stretched) -> String {
        huh::encode(&envelope::seal_password(salt, stretched, &self.0))
    }

    /// Encrypts in a session and writes the result as huH.
    pub fn seal_session(self, secret: &Secret) -> String {
        huh::encode(&envelope::seal_session(envelope::fresh_salt(), secret, &self.0))
    }

    /// Encrypts under a password or in a session, stretching the password here.
    pub fn seal(self, key: Key) -> String {
        match key {
            Key::Password(pass) => {
                let salt = envelope::fresh_salt();
                self.seal_stretched(salt, &envelope::stretch(pass, &salt))
            }
            Key::Session(secret) => self.seal_session(secret),
        }
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
pub fn lock_text(msg: &str, key: Key) -> Result<String, LockError> {
    Ok(prepare_text(msg)?.seal(key))
}

/// Locks a file (its bytes, name and type) into huH.
pub fn lock_file(bytes: &[u8], name: &str, mime: &str, key: Key) -> Result<String, LockError> {
    Ok(prepare_file(bytes, name, mime)?.seal(key))
}

/// What a huH needs before it will open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Needs {
    /// An old huH, which doesn't say how it was locked: the password as it was
    /// typed, or the session's key as text.
    Old,
    /// A password, stretched with this salt for this many rounds of PBKDF2.
    Password { salt: Salt, rounds: u32 },
    /// The session it was locked in.
    Session,
}

/// A huH that has been read and not yet opened.
pub struct Sealed {
    bytes: Vec<u8>,
    needs: Needs,
}

/// Whether text is written the way locked things are: in huH letters, or as
/// the raw base64 from before huH. It says nothing of whether it will open.
pub fn looks_locked(msg: &str) -> bool {
    huh::is_huh(msg) || msg.starts_with(RAW_START)
}

/// The first step of unlocking: read the huH (already trimmed) and see how it
/// was locked.
pub fn read(msg: &str) -> Result<Sealed, UnlockError> {
    let bytes = if huh::is_huh(msg) {
        huh::decode(msg).map_err(|_| UnlockError::NotHuh)?
    } else if msg.starts_with(RAW_START) {
        b64::decode(msg).unwrap_or_default()
    } else {
        return Err(UnlockError::NotHuh);
    };
    let needs = match envelope::read(&bytes) {
        Ok((Kind::Password, salt)) => Needs::Password { salt, rounds: envelope::PASSWORD_ROUNDS },
        Ok((Kind::Session, _)) => Needs::Session,
        Err(Unreadable::Newer) => return Err(UnlockError::Newer),
        // Anything else is tried as an old huH, which is what it is if it is anything.
        Err(Unreadable::NotOne) => Needs::Old,
    };
    Ok(Sealed { bytes, needs })
}

/// How a huH was locked, from a glance at its two ends: enough to tell someone
/// they are in the wrong mode, or that this is no huH, before a password is
/// asked for or a long huH is read. `Ok(None)` for an old huH, which doesn't say.
/// (For a huH that is whole this agrees with [`read`]; for one that has been
/// knocked about, `read` is the one to believe.)
pub fn glance(msg: &str) -> Result<Option<Kind>, UnlockError> {
    if !looks_locked(msg) {
        return Err(UnlockError::NotHuh);
    }
    match huh::peek(msg).map(|(first, len)| envelope::kind_of(first, len)) {
        Some(Ok(kind)) => Ok(Some(kind)),
        Some(Err(Unreadable::Newer)) => Err(UnlockError::Newer),
        Some(Err(Unreadable::NotOne)) | None => Ok(None),
    }
}

impl Sealed {
    pub fn needs(&self) -> Needs {
        self.needs
    }

    /// Opens an old huH with the password as it was typed, or with a session's
    /// key written out as base64, which is what old session huHs were locked with.
    pub fn open_old(&self, pass: &str) -> Result<Opened, UnlockError> {
        if self.needs != Needs::Old {
            return Err(UnlockError::WrongKey);
        }
        cipher::open(&self.bytes, pass.as_bytes())
            .filter(|p| !p.is_empty())
            .map(|payload| Opened { payload, old: true })
            .ok_or(UnlockError::WrongKey)
    }

    /// Opens a password huH with the password already stretched as
    /// [`Needs::Password`] says.
    pub fn open_stretched(&self, stretched: &Stretched) -> Result<Opened, UnlockError> {
        self.opened(envelope::open_password(&self.bytes, stretched))
    }

    /// Opens a session huH with the session's secret.
    pub fn open_session(&self, secret: &Secret) -> Result<Opened, UnlockError> {
        self.opened(envelope::open_session(&self.bytes, secret))
    }

    fn opened(&self, payload: Option<Vec<u8>>) -> Result<Opened, UnlockError> {
        payload.map(|payload| Opened { payload, old: false }).ok_or(UnlockError::WrongKey)
    }

    /// Opens with a password or a session, stretching the password here.
    pub fn open(&self, key: Key) -> Result<Opened, UnlockError> {
        match (self.needs, key) {
            (Needs::Old, Key::Password(pass)) => self.open_old(pass),
            (Needs::Old, Key::Session(secret)) => self.open_old(&b64::encode(secret)),
            (Needs::Password { salt, rounds }, Key::Password(pass)) => {
                self.open_stretched(&crate::kdf::pbkdf2(pass, &salt, rounds))
            }
            (Needs::Session, Key::Session(secret)) => self.open_session(secret),
            (Needs::Password { .. }, Key::Session(_)) => Err(UnlockError::NeedsPassword),
            (Needs::Session, Key::Password(_)) => Err(UnlockError::NeedsSession),
        }
    }
}

/// What came out of the envelope: the payload, possibly still squeezed.
pub struct Opened {
    payload: Vec<u8>,
    old: bool,
}

impl Opened {
    /// Whether there is anything to decompress.
    pub fn is_squeezed(&self) -> bool {
        squeeze::is_squeezed(&self.payload)
    }

    /// Whether it came out of the old envelope, which had no way to tell if a
    /// huH was altered after it was made. What an old huH says is not proof of
    /// what was locked into it.
    pub fn is_old(&self) -> bool {
        self.old
    }

    /// The second step of unlocking: decompress if it was squeezed, and work out
    /// whether it is a file or a message.
    pub fn finish(self) -> Result<Unlocked, UnlockError> {
        let Opened { mut payload, old } = self;
        if squeeze::is_squeezed(&payload) {
            payload = squeeze::unsqueeze(&payload).map_err(|_| UnlockError::Damaged)?;
        }

        if let Some(file) = payload::unpack_file(&payload) {
            payload.drain(..file.data_start);
            return Ok(Unlocked::File { name: file.name, mime: file.mime, bytes: payload });
        }

        match String::from_utf8(payload) {
            Ok(text) if !text.is_empty() => Ok(Unlocked::Text(text)),
            // A wrong password now and then gets an old huH this far (its padding
            // happens to come out right) and hands over nonsense. Today's envelope
            // opens only for whoever holds its key, so nonsense in it was locked
            // into it.
            _ => Err(if old { UnlockError::WrongKey } else { UnlockError::Damaged }),
        }
    }
}

/// Unlocks a huH (already trimmed) with a password or in a session.
pub fn unlock(msg: &str, key: Key) -> Result<Unlocked, UnlockError> {
    read(msg)?.open(key)?.finish()
}

// What these hold is exactly what must not end up in a log, so they say how
// much of it there is and no more.
impl fmt::Debug for Key<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Key::Password(_) => "Key::Password(…)",
            Key::Session(_) => "Key::Session(…)",
        })
    }
}

impl fmt::Debug for Prepared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Prepared({} bytes)", self.0.len())
    }
}

impl fmt::Debug for Sealed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sealed({} bytes, needs {:?})", self.bytes.len(), self.needs)
    }
}

impl fmt::Debug for Opened {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Opened({} bytes{})", self.payload.len(), if self.old { ", old" } else { "" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret() -> Secret {
        core::array::from_fn(|i| (i * 7 + 3) as u8)
    }

    /// Stands in for a stretched password where the stretching itself isn't
    /// what is being tested (600,000 rounds a time adds up).
    fn stretched() -> Stretched {
        core::array::from_fn(|i| (i * 5 + 1) as u8)
    }

    fn in_session(secret: &Secret) -> Key<'_> {
        Key::Session(secret)
    }

    #[test]
    fn text_round_trip() {
        let (secret, other) = (secret(), [9u8; 32]);
        for msg in ["hi", "  spaces kept  \n", "සිංහල 🐸 \u{feff}bom", &"long message. ".repeat(500)] {
            let huh = lock_text(msg, in_session(&secret)).unwrap();
            assert!(huh::is_huh(&huh));
            assert_eq!(glance(&huh), Ok(Some(Kind::Session)));
            assert_eq!(unlock(&huh, in_session(&secret)), Ok(Unlocked::Text(msg.to_owned())));
            assert_eq!(unlock(&huh, in_session(&other)), Err(UnlockError::WrongKey));
            assert_eq!(unlock(&huh, Key::Password("a password")), Err(UnlockError::NeedsSession));
        }
    }

    #[test]
    fn with_a_password() {
        // The whole way round, stretching and all (once each way: it is slow on purpose).
        let huh = lock_text("the spare key is under the blue pot", Key::Password("correct horse")).unwrap();
        assert_eq!(glance(&huh), Ok(Some(Kind::Password)));
        assert_eq!(
            unlock(&huh, Key::Password("correct horse")),
            Ok(Unlocked::Text("the spare key is under the blue pot".to_owned()))
        );
        assert_eq!(unlock(&huh, in_session(&secret())), Err(UnlockError::NeedsPassword));
        // A wrong password is a wrong key, which is tried here without the wait.
        assert_eq!(read(&huh).unwrap().open_stretched(&stretched()).err(), Some(UnlockError::WrongKey));
    }

    #[test]
    fn in_two_steps() {
        // As the page does it: compress, then encrypt under a password stretched elsewhere.
        let msg = "Let me know when you get home. ".repeat(6);
        let prepared = prepare_text(&msg).unwrap();
        assert!(prepared.is_squeezed());
        let salt = envelope::fresh_salt();
        let huh = prepared.seal_stretched(salt, &stretched());

        let sealed = read(&huh).unwrap();
        assert_eq!(sealed.needs(), Needs::Password { salt, rounds: envelope::PASSWORD_ROUNDS });
        let opened = sealed.open_stretched(&stretched()).unwrap();
        assert!(opened.is_squeezed() && !opened.is_old());
        assert_eq!(opened.finish(), Ok(Unlocked::Text(msg)));
        assert_eq!(sealed.open_stretched(&[0; 32]).err(), Some(UnlockError::WrongKey));
        // The wrong kind of open is refused, even with the very same bytes for a key.
        assert_eq!(sealed.open_session(&stretched()).err(), Some(UnlockError::WrongKey));
        assert_eq!(sealed.open_old("pw").err(), Some(UnlockError::WrongKey));

        // A note too short to squeeze leaves nothing to decompress.
        let short = prepare_text("hi").unwrap();
        assert!(!short.is_squeezed());
        let secret = secret();
        let sealed = read(&short.seal_session(&secret)).unwrap();
        assert_eq!(sealed.needs(), Needs::Session);
        assert!(!sealed.open_session(&secret).unwrap().is_squeezed());
        assert_eq!(sealed.open_stretched(&secret).err(), Some(UnlockError::WrongKey));

        let file = prepare_file(&[7u8; 4000], "sevens.bin", "").unwrap();
        assert!(file.is_squeezed());
        assert_eq!(
            read(&file.seal_session(&secret)).unwrap().open_session(&secret).unwrap().finish(),
            Ok(Unlocked::File { name: "sevens.bin".into(), mime: String::new(), bytes: vec![7u8; 4000] })
        );
    }

    #[test]
    fn long_text_gets_shorter() {
        let msg = "The quick brown fox jumps over the lazy dog. ".repeat(20);
        let huh = lock_text(&msg, in_session(&secret())).unwrap();
        let sealed = huh::decode(&huh).unwrap();
        assert!(sealed.len() < envelope::sealed_len(Kind::Session, msg.len() / 4));
    }

    #[test]
    fn length_says_little() {
        // Whatever is locked, the huH's bytes come in whole steps of sixteen, so
        // short answers can't be told apart by counting.
        let secret = secret();
        let sealed_len = |msg: &str, key: Key| huh::decode(&lock_text(msg, key).unwrap()).unwrap().len();
        let answers = ["y", "no", "yes", "nope", "maybe", "I love you", "I hate you"];
        for answer in answers {
            assert_eq!(sealed_len(answer, in_session(&secret)), sealed_len("no", in_session(&secret)), "{answer}");
            assert_eq!(sealed_len(answer, in_session(&secret)), envelope::sealed_len(Kind::Session, 0));
        }
        // Squeezing is only kept when it saves a whole step, so it never gives a length away either.
        for msg in ["thank you so much", "see you tomorrow at the usual place", "Let me know when you get home."] {
            let prepared = prepare_text(msg).unwrap();
            assert!(envelope::padded(prepared.0.len()) <= envelope::padded(msg.len()));
            assert_eq!(prepared.is_squeezed(), envelope::padded(prepared.0.len()) < envelope::padded(msg.len()), "{msg}");
        }
    }

    #[test]
    fn file_round_trip() {
        let secret = secret();
        let compressible = "id,name,score\n1,someone,42\n".repeat(400).into_bytes();
        let mut noise = vec![0u8; 4000];
        crate::rng::fill(&mut noise);
        for (bytes, name, mime) in [
            (compressible, "scores.csv", "text/csv"),
            (noise, "රූපය 🐸.bin", ""),
            (Vec::new(), "empty", "application/octet-stream"),
        ] {
            let huh = lock_file(&bytes, name, mime, in_session(&secret)).unwrap();
            assert_eq!(
                unlock(&huh, in_session(&secret)),
                Ok(Unlocked::File { name: name.to_owned(), mime: mime.to_owned(), bytes })
            );
        }
    }

    #[test]
    fn not_a_huh() {
        let pw = Key::Password("pw");
        assert_eq!(unlock("hello there", pw), Err(UnlockError::NotHuh));
        assert_eq!(unlock("huHUhuHUhuH", pw), Err(UnlockError::WrongKey));
        assert_eq!(unlock("U2FsdGVkX1+++", pw), Err(UnlockError::WrongKey));
        assert_eq!(unlock("", pw), Err(UnlockError::NotHuh));

        // A glance says the same of what is no huH, and makes nothing of what
        // is only huH letters.
        for not_one in ["hello there", "", "huH!UhuH", "u2fsdGVk"] {
            assert_eq!(glance(not_one), Err(UnlockError::NotHuh), "{not_one:?}");
            assert!(!looks_locked(not_one));
        }
        for letters_only in ["huHUhuHUhuH", "U2FsdGVkX1+++", "U", "UUUUUUUUUUUUUUUU"] {
            assert_eq!(glance(letters_only), Ok(None), "{letters_only:?}");
            assert!(looks_locked(letters_only));
        }
    }

    #[test]
    fn a_huh_changed_on_the_way_does_not_open() {
        let secret = secret();
        let msg = "wire the 500 to the account ending 4417";
        let sealed = huh::decode(&lock_text(msg, in_session(&secret)).unwrap()).unwrap();
        assert_eq!(unlock(&huh::encode(&sealed), in_session(&secret)), Ok(Unlocked::Text(msg.to_owned())));
        // One bit, anywhere at all.
        for at in 0..sealed.len() {
            for bit in 0..8 {
                let mut changed = sealed.clone();
                changed[at] ^= 1 << bit;
                assert!(unlock(&huh::encode(&changed), in_session(&secret)).is_err(), "bit {bit} of byte {at}");
            }
        }
        // Cut short at any point (what a half-pasted huH is).
        for keep in 1..sealed.len() {
            assert!(unlock(&huh::encode(&sealed[..keep]), in_session(&secret)).is_err(), "cut to {keep}");
        }

        // The same holds for a file that doesn't squeeze, which the old envelope
        // let through with a byte of somebody else's choosing in it.
        let mut photo = vec![0u8; 3000];
        crate::rng::fill(&mut photo);
        let mut sealed = huh::decode(&lock_file(&photo, "photo.jpg", "image/jpeg", in_session(&secret)).unwrap()).unwrap();
        sealed[1500] ^= 0xff;
        assert_eq!(unlock(&huh::encode(&sealed), in_session(&secret)), Err(UnlockError::WrongKey));

        // And for a password huH, under its stretched password.
        let sealed = huh::decode(&prepare_text(msg).unwrap().seal_stretched(envelope::fresh_salt(), &stretched())).unwrap();
        assert!(read(&huh::encode(&sealed)).unwrap().open_stretched(&stretched()).is_ok());
        for at in 0..sealed.len() {
            let mut changed = sealed.clone();
            changed[at] ^= 0x10;
            let opened = read(&huh::encode(&changed)).and_then(|sealed| sealed.open_stretched(&stretched()));
            assert!(opened.is_err(), "byte {at}");
        }
    }

    #[test]
    fn a_broken_payload_says_so() {
        let secret = secret();
        let seal = |payload: &[u8]| huh::encode(&envelope::seal_session(envelope::fresh_salt(), &secret, payload));
        // A squeezed payload with its tail missing, sealed properly: right key, broken contents.
        let prepared = prepare_text(&"all work and no play makes jack a dull boy. ".repeat(40)).unwrap();
        assert!(prepared.is_squeezed());
        let cut = &prepared.0[..prepared.0.len() - 7];
        assert_eq!(unlock(&seal(cut), in_session(&secret)), Err(UnlockError::Damaged));
        // Nothing at all, and bytes that are no text, are not a message either.
        assert_eq!(unlock(&seal(b""), in_session(&secret)), Err(UnlockError::Damaged));
        assert_eq!(unlock(&seal(&[0xff, 0xfe, 0x80]), in_session(&secret)), Err(UnlockError::Damaged));
    }

    #[test]
    fn old_huhs_still_open_and_say_they_are_old() {
        // (A fixed salt, so what a wrong key makes of these is the same every run.)
        let msg = "locked before huHs had a check on them";
        let old = huh::encode(&cipher::seal_with_salt(msg.as_bytes(), b"pw", *b"old salt"));
        assert_eq!(glance(&old), Ok(None));
        let sealed = read(&old).unwrap();
        assert_eq!(sealed.needs(), Needs::Old);
        let opened = sealed.open_old("pw").unwrap();
        assert!(opened.is_old());
        assert_eq!(opened.finish(), Ok(Unlocked::Text(msg.to_owned())));
        assert_eq!(unlock(&old, Key::Password("pw")), Ok(Unlocked::Text(msg.to_owned())));
        assert_eq!(unlock(&old, Key::Password("pW")), Err(UnlockError::WrongKey));
        // An old huH has no key of today's kinds.
        assert_eq!(sealed.open_stretched(&stretched()).err(), Some(UnlockError::WrongKey));
        assert_eq!(sealed.open_session(&secret()).err(), Some(UnlockError::WrongKey));

        // An old session huH was locked with the session's key written as base64.
        let secret = secret();
        let old = huh::encode(&cipher::seal_with_salt(msg.as_bytes(), b64::encode(&secret).as_bytes(), *b"old salt"));
        assert_eq!(unlock(&old, in_session(&secret)), Ok(Unlocked::Text(msg.to_owned())));
        assert_eq!(unlock(&old, in_session(&[9; 32])), Err(UnlockError::WrongKey));
    }

    #[test]
    fn what_looks_newer_says_so() {
        let secret = secret();
        let mut sealed = huh::decode(&lock_text("hello from today", in_session(&secret)).unwrap()).unwrap();
        sealed[0] = 0x03;
        let from_later = huh::encode(&sealed);
        assert_eq!(read(&from_later).err(), Some(UnlockError::Newer));
        assert_eq!(unlock(&from_later, in_session(&secret)), Err(UnlockError::Newer));
        assert_eq!(glance(&from_later), Err(UnlockError::Newer));
    }

    #[test]
    fn nothing_secret_is_printed() {
        let secret = secret();
        let key = Key::Password("hunter2-hunter2");
        let prepared = prepare_text("a message nobody should see in a log").unwrap();
        let said = format!("{key:?} {:?} {prepared:?}", in_session(&secret));
        assert!(!said.contains("hunter2") && !said.contains("message") && !said.contains("3, 10"), "{said}");
        let sealed = read(&prepared.seal_session(&secret)).unwrap();
        let opened = sealed.open_session(&secret).unwrap();
        let said = format!("{sealed:?} {opened:?}");
        assert!(!said.contains("message") && !said.contains("109, 101"), "{said}");
    }
}
