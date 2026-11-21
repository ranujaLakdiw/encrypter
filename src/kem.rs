//! Session pairing with ML-KEM-1024 (FIPS 203).
//!
//! Both people run the same flow: each makes a keypair and sends the public key,
//! each encapsulates to the other's key (that ciphertext is the "reply"), and each
//! decapsulates the reply they receive. Both then hold the same two secrets, one
//! out of each reply.
//!
//! The session's secret is HKDF-SHA256 over those two secrets *and over
//! everything that was swapped to get them*: both public keys and both replies,
//! led by whichever public key is the smaller, so it comes out the same on both
//! sides whoever went first. Two things follow from hashing the lot:
//!
//! * A secret only ever comes out of a reply that was really opened. (The first
//!   version of the page XORed the two secrets together instead. Handed back its
//!   own key and its own reply it XORed a secret with itself, and the session's
//!   key was thirty-two zeros that anybody could use.)
//! * Both people end up with the same secret only if both saw the very same four
//!   codes. Change one on the way and the two secrets have nothing in common.
//!
//! Swapping codes keeps the secret from anyone who only listens. What it cannot
//! tell is *whose* codes arrived: someone able to change them in transit can
//! hand each person their own instead and sit in the middle, holding a session
//! with each. That is what the check number is for. It is cut from the secret
//! (one way: it gives nothing of the secret back), so two people in the same
//! session see the same forty digits, and two sessions with someone between
//! them see different ones.
//!
//! How much that proves depends on how much of it is compared. Whoever sits in
//! the middle sends her replies last, and can make as many as she likes before
//! choosing which to send. Getting two sessions to show the same *whole* number
//! would take some 2^66 fresh ML-KEM replies, which is out of anyone's reach;
//! getting the first ten digits to agree takes about a second. So it is all
//! forty or nothing, compared somewhere the codes did not travel.

use ml_kem::{
    kem::{Decapsulate, Encapsulate, FromSeed, KeyExport, TryKeyInit},
    DecapsulationKey, EncapsulationKey, MlKem1024,
};
use zeroize::Zeroize;

use crate::{
    kdf,
    rng::{self, PlatformRng},
};

pub const SECRET_LEN: usize = 32;
pub type Secret = [u8; SECRET_LEN];

/// How long a public key is in ML-KEM-1024, and a reply too.
const CODE_LEN: usize = 1568;

/// What HKDF is told each thing it makes is for.
const SESSION: &[u8] = b"huH session";
const NAME: &[u8] = b"huH session name";
const CHECK: &[u8] = b"huH session check";

/// The other person's key or reply was not a valid ML-KEM-1024 one.
#[derive(Debug, PartialEq, Eq)]
pub struct BadCode;

/// Why two swapped codes did not make a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairError {
    /// Their key or their reply is not a valid ML-KEM-1024 one.
    BadCode,
    /// "Their" key is my own.
    OwnKey,
    /// "Their" reply is the one I made.
    OwnReply,
    /// A key where their reply belongs: theirs, mine, anybody's. A key and a
    /// reply are the same length, so ML-KEM would open one without complaint,
    /// into a session nobody else is in.
    KeyForReply,
}

/// My half of a pairing. The private key never leaves this value.
pub struct KeyPair {
    dk: DecapsulationKey<MlKem1024>,
    public: Vec<u8>,
}

impl KeyPair {
    pub fn generate() -> Self {
        let mut seed = [0u8; 64];
        rng::fill(&mut seed);
        let (dk, ek) = MlKem1024::from_seed(&seed.into());
        seed.zeroize();
        Self { dk, public: ek.to_bytes().to_vec() }
    }

    /// My public key, to send to the other person.
    pub fn public(&self) -> &[u8] {
        &self.public
    }

    /// Whether a key that is supposed to be the other person's is in fact mine.
    pub fn is_mine(&self, key: &[u8]) -> bool {
        key == self.public
    }

    /// Opens the reply the other person made for my public key.
    pub fn decapsulate(&self, their_reply: &[u8]) -> Result<Secret, BadCode> {
        let shared = self.dk.decapsulate_slice(their_reply).map_err(|_| BadCode)?;
        Ok(shared.into())
    }

    /// Finishes a pairing: opens their reply and works out the session's secret
    /// from everything that was swapped. `my_reply` and `my_secret` are what
    /// [`encapsulate`] gave for `their_public`.
    pub fn pair(
        &self,
        their_public: &[u8],
        my_reply: &[u8],
        my_secret: &Secret,
        their_reply: &[u8],
    ) -> Result<Secret, PairError> {
        if my_reply.len() != CODE_LEN {
            return Err(PairError::BadCode);
        }
        if self.is_mine(their_public) {
            return Err(PairError::OwnKey);
        }
        // Their key has to be a real one (as it was when my reply was made for it).
        let is_a_key = |code: &[u8]| EncapsulationKey::<MlKem1024>::new_from_slice(code).is_ok();
        if !is_a_key(their_public) {
            return Err(PairError::BadCode);
        }
        if their_reply == my_reply {
            return Err(PairError::OwnReply);
        }
        // A key reads as a key and a reply does not (a real reply passing for
        // one is a chance of about one in 10^92), so this catches every key
        // there is, not only the two in this exchange.
        if is_a_key(their_reply) {
            return Err(PairError::KeyForReply);
        }
        let mut their_secret = self.decapsulate(their_reply).map_err(|_| PairError::BadCode)?;
        // Their reply was made for my key, and mine for theirs.
        let secret = session_secret(
            Side { public: &self.public, reply: their_reply, secret: &their_secret },
            Side { public: their_public, reply: my_reply, secret: my_secret },
        );
        their_secret.zeroize();
        Ok(secret)
    }
}

/// Makes my reply for the other person's public key: (reply to send, my shared secret).
pub fn encapsulate(their_public: &[u8]) -> Result<(Vec<u8>, Secret), BadCode> {
    let ek = EncapsulationKey::<MlKem1024>::new_from_slice(their_public).map_err(|_| BadCode)?;
    let (reply, shared) = ek.encapsulate_with_rng(&mut PlatformRng);
    Ok((reply.to_vec(), shared.into()))
}

/// One person's part of an exchange: their public key, the reply that was made
/// for it, and the secret inside that reply.
struct Side<'a> {
    public: &'a [u8],
    reply: &'a [u8],
    secret: &'a Secret,
}

/// The session's secret:
/// `HKDF-SHA256(the two secrets, info = "huH session" · SHA-256(the two public keys · the two replies))`,
/// each pair in the order of the public keys.
fn session_secret<'a>(a: Side<'a>, b: Side<'a>) -> Secret {
    let (first, second) = if a.public <= b.public { (a, b) } else { (b, a) };
    // Keys and replies are all the same length in ML-KEM-1024, so running them
    // together leaves no doubt where one ends and the next begins.
    let swapped = kdf::sha256(&[first.public, second.public, first.reply, second.reply]);
    let mut secrets = [0u8; 2 * SECRET_LEN];
    secrets[..SECRET_LEN].copy_from_slice(first.secret);
    secrets[SECRET_LEN..].copy_from_slice(second.secret);
    let secret = kdf::hkdf(&secrets, &[], &[SESSION, &swapped[..]].concat());
    secrets.zeroize();
    secret
}

const WORDS_A: [&str; 20] = [
    "amber", "brave", "crisp", "delta", "ember", "faint", "ghost", "haven", "ivory", "jewel",
    "kite", "lunar", "mist", "neon", "onyx", "pearl", "quill", "raven", "slate", "thorn",
];
const WORDS_B: [&str; 20] = [
    "arc", "bay", "cog", "dune", "echo", "floe", "gust", "haze", "isle", "jade", "knot", "lore",
    "maze", "nova", "orb", "pike", "reef", "sage", "tide", "vale",
];

/// A short name for a session, to tell one from another and to catch a code
/// that was pasted wrong. Both people see the same one. It is cut from the
/// secret one way, so showing it gives none of the secret away; and with forty
/// thousand names to go round, it is not proof of anything. That is
/// [`check_number`]'s job.
pub fn session_name(secret: &Secret) -> String {
    let cut: [u8; 3] = kdf::hkdf(secret, &[], NAME);
    let a = WORDS_A[cut[0] as usize % WORDS_A.len()];
    let b = WORDS_B[cut[1] as usize % WORDS_B.len()];
    format!("{a}-{b}-{:02}", cut[2] % 100)
}

/// How many five-digit groups a check number has. Eight of them are about 133
/// bits: finding two sessions whose numbers agree would take some 2^66 tries,
/// each one a fresh ML-KEM reply.
pub const CHECK_GROUPS: usize = 8;

/// The number two people compare to be sure they are in the same session with
/// nobody in the middle: eight groups of five digits, cut from the secret.
/// It only proves anything when all of it is compared, and somewhere the codes
/// did not travel (on a call, face to face), since whoever could change the
/// codes could change a number sent the same way.
pub fn check_number(secret: &Secret) -> String {
    let cut: [u8; CHECK_GROUPS * 5] = kdf::hkdf(secret, &[], CHECK);
    cut.chunks(5)
        .map(|five| {
            let n = five.iter().fold(0u64, |n, &byte| n << 8 | u64::from(byte));
            format!("{:05}", n % 100_000)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::b64;

    /// A pairing as two people do it: each replies to the other's key, then
    /// finishes with the reply they were sent.
    fn paired(alice: &KeyPair, bob: &KeyPair) -> (Secret, Secret) {
        let (alice_reply, alice_secret) = encapsulate(bob.public()).unwrap();
        let (bob_reply, bob_secret) = encapsulate(alice.public()).unwrap();
        assert_eq!(alice_reply.len(), 1568);
        (
            alice.pair(bob.public(), &alice_reply, &alice_secret, &bob_reply).unwrap(),
            bob.pair(alice.public(), &bob_reply, &bob_secret, &alice_reply).unwrap(),
        )
    }

    #[test]
    fn both_sides_agree() {
        let (alice, bob) = (KeyPair::generate(), KeyPair::generate());
        assert_eq!(alice.public().len(), 1568);
        let (alice_key, bob_key) = paired(&alice, &bob);
        assert_eq!(alice_key, bob_key);
        assert_eq!(session_name(&alice_key), session_name(&bob_key));
        assert_eq!(check_number(&alice_key), check_number(&bob_key));

        // The same two people pairing again get another session altogether.
        let (again, _) = paired(&alice, &bob);
        assert_ne!(again, alice_key);
        assert_ne!(check_number(&again), check_number(&alice_key));
    }

    #[test]
    fn refuses_its_own_codes() {
        let (me, them) = (KeyPair::generate(), KeyPair::generate());
        assert!(me.is_mine(me.public()) && !me.is_mine(them.public()));

        // Handed back my own key and my own reply: what gave a key of all zeros once.
        let (own_reply, own_secret) = encapsulate(me.public()).unwrap();
        assert_eq!(me.pair(me.public(), &own_reply, &own_secret, &own_reply), Err(PairError::OwnKey));

        // The right key, but my own reply pasted where theirs belongs.
        let (my_reply, my_secret) = encapsulate(them.public()).unwrap();
        assert_eq!(me.pair(them.public(), &my_reply, &my_secret, &my_reply), Err(PairError::OwnReply));

        // A key pasted where their reply belongs, theirs, mine or anyone's (one
        // left over from an earlier try, say): the same length as a reply, so
        // nothing else would stop it.
        let someone_else = KeyPair::generate();
        for key in [them.public(), me.public(), someone_else.public()] {
            assert_eq!(me.pair(them.public(), &my_reply, &my_secret, key), Err(PairError::KeyForReply));
        }
        // No real reply is taken for a key.
        for _ in 0..50 {
            let (their_reply, _) = encapsulate(me.public()).unwrap();
            assert!(me.pair(them.public(), &my_reply, &my_secret, &their_reply).is_ok());
        }
    }

    #[test]
    fn a_secret_only_comes_from_what_was_really_opened() {
        // Even with the refusals above out of the way, a pairing fed its own
        // codes makes a secret that takes the private key to work out: never
        // zeros, never the same twice.
        let reflected = || {
            let me = KeyPair::generate();
            let (reply, secret) = encapsulate(me.public()).unwrap();
            let opened = me.decapsulate(&reply).unwrap();
            assert_eq!(opened, secret);
            session_secret(
                Side { public: me.public(), reply: &reply, secret: &opened },
                Side { public: me.public(), reply: &reply, secret: &secret },
            )
        };
        let (one, other) = (reflected(), reflected());
        assert_ne!(one, [0u8; SECRET_LEN]);
        assert_ne!(one, other);
    }

    #[test]
    fn someone_in_the_middle_shows_in_the_check_number() {
        // Mallory hands Alice and Bob her own key and her own replies, and so
        // holds a session with each. They are two sessions, not one, and their
        // numbers differ.
        let (alice, bob, mallory) = (KeyPair::generate(), KeyPair::generate(), KeyPair::generate());
        let (alice_reply, alice_secret) = encapsulate(mallory.public()).unwrap();
        let (bob_reply, bob_secret) = encapsulate(mallory.public()).unwrap();
        let (to_alice, _) = encapsulate(alice.public()).unwrap();
        let (to_bob, _) = encapsulate(bob.public()).unwrap();
        let alice_key = alice.pair(mallory.public(), &alice_reply, &alice_secret, &to_alice).unwrap();
        let bob_key = bob.pair(mallory.public(), &bob_reply, &bob_secret, &to_bob).unwrap();
        assert_ne!(alice_key, bob_key);
        assert_ne!(check_number(&alice_key), check_number(&bob_key));
    }

    #[test]
    fn a_matching_name_proves_nothing() {
        // Mallory sends her replies last, so she can make as many as she likes
        // and pick. With forty thousand names to go round, a few hundred tries
        // on each side find two sessions that are called the same. Their
        // numbers still differ: making those agree as well is the 2^66 of work
        // nobody has, and why it is the number that gets compared.
        let (alice, bob, mallory) = (KeyPair::generate(), KeyPair::generate(), KeyPair::generate());
        let (alice_reply, _) = encapsulate(mallory.public()).unwrap();
        let (bob_reply, _) = encapsulate(mallory.public()).unwrap();
        let mut with_alice = std::collections::HashMap::new();
        let (alice_key, bob_key) = loop {
            let (to_alice, secret) = encapsulate(alice.public()).unwrap();
            let key = mallory.pair(alice.public(), &to_alice, &secret, &alice_reply).unwrap();
            with_alice.insert(session_name(&key), key);
            let (to_bob, secret) = encapsulate(bob.public()).unwrap();
            let key = mallory.pair(bob.public(), &to_bob, &secret, &bob_reply).unwrap();
            if let Some(same_name) = with_alice.get(&session_name(&key)) {
                break (*same_name, key);
            }
        };
        assert_eq!(session_name(&alice_key), session_name(&bob_key));
        assert_ne!(alice_key, bob_key);
        assert_ne!(check_number(&alice_key), check_number(&bob_key));
    }

    #[test]
    fn every_code_that_was_swapped_counts() {
        let (alice, bob) = (KeyPair::generate(), KeyPair::generate());
        let (alice_reply, alice_secret) = encapsulate(bob.public()).unwrap();
        let (bob_reply, bob_secret) = encapsulate(alice.public()).unwrap();
        let opened = alice.decapsulate(&bob_reply).unwrap();
        let secret_with = |alice_public: &[u8], bob_public: &[u8], to_alice: &[u8], to_bob: &[u8]| {
            session_secret(
                Side { public: alice_public, reply: to_alice, secret: &opened },
                Side { public: bob_public, reply: to_bob, secret: &alice_secret },
            )
        };
        let real = secret_with(alice.public(), bob.public(), &bob_reply, &alice_reply);
        assert_eq!(real, alice.pair(bob.public(), &alice_reply, &alice_secret, &bob_reply).unwrap());
        // It is the same whichever side is named first...
        assert_eq!(
            real,
            session_secret(
                Side { public: bob.public(), reply: &alice_reply, secret: &alice_secret },
                Side { public: alice.public(), reply: &bob_reply, secret: &opened },
            )
        );
        // ...and different if a single bit of any of the four codes is.
        let flipped = |code: &[u8]| {
            let mut code = code.to_vec();
            code[700] ^= 1;
            code
        };
        assert_ne!(real, secret_with(&flipped(alice.public()), bob.public(), &bob_reply, &alice_reply));
        assert_ne!(real, secret_with(alice.public(), &flipped(bob.public()), &bob_reply, &alice_reply));
        assert_ne!(real, secret_with(alice.public(), bob.public(), &flipped(&bob_reply), &alice_reply));
        assert_ne!(real, secret_with(alice.public(), bob.public(), &bob_reply, &flipped(&alice_reply)));
        // A reply that was meddled with on the way opens as something else
        // entirely (ML-KEM never says "no"), so the two sides part company.
        let meddled = alice.pair(bob.public(), &alice_reply, &alice_secret, &flipped(&bob_reply)).unwrap();
        assert_ne!(meddled, bob.pair(alice.public(), &bob_reply, &bob_secret, &alice_reply).unwrap());
    }

    /// The secret, the name and the number are plain SHA-256 and HKDF-SHA256 over
    /// what was swapped. These were worked out with Node's own crypto before this
    /// code had run, from made-up codes of the right sizes; both people have to
    /// keep getting the same answers, so they must never change.
    #[test]
    fn what_the_standard_functions_give() {
        let fill = |n: usize, f: fn(usize) -> usize| (0..n).map(|i| f(i) as u8).collect::<Vec<u8>>();
        let (low_key, high_key) = (fill(1568, |i| i * 3 + 1), fill(1568, |i| i * 5 + 2));
        // The reply made for each key, and the secret inside it.
        let (for_low, for_high) = (fill(1568, |i| i * 7 + 3), fill(1568, |i| i * 11 + 4));
        let in_low: Secret = core::array::from_fn(|i| i as u8 + 100);
        let in_high: Secret = core::array::from_fn(|i| 200 - i as u8);

        let secret = session_secret(
            Side { public: &high_key, reply: &for_high, secret: &in_high },
            Side { public: &low_key, reply: &for_low, secret: &in_low },
        );
        assert_eq!(b64::encode(&secret), "45BFbXynOoRE/3F//p4O1KzAElcqpYBPLH3wgYiOA2A=");
        assert_eq!(session_name(&secret), "slate-knot-85");
        assert_eq!(check_number(&secret), "03922 83853 97841 56876 63676 23024 36353 09380");
    }

    #[test]
    fn refuses_codes_of_the_wrong_shape() {
        assert_eq!(encapsulate(&[1, 2, 3]).err(), Some(BadCode));
        assert_eq!(encapsulate(&[0xff; 1568]).err(), Some(BadCode));
        let me = KeyPair::generate();
        assert_eq!(me.decapsulate(&[0; 10]).err(), Some(BadCode));

        let them = KeyPair::generate();
        let (my_reply, my_secret) = encapsulate(them.public()).unwrap();
        let (their_reply, _) = encapsulate(me.public()).unwrap();
        assert_eq!(me.pair(them.public(), &my_reply, &my_secret, &their_reply[..100]), Err(PairError::BadCode));
        assert_eq!(me.pair(&them.public()[..100], &my_reply, &my_secret, &their_reply), Err(PairError::BadCode));
        // The right length, but no key at all; and a reply of mine that isn't one.
        assert_eq!(me.pair(&[0xff; 1568], &my_reply, &my_secret, &their_reply), Err(PairError::BadCode));
        assert_eq!(me.pair(them.public(), &my_reply[..100], &my_secret, &their_reply), Err(PairError::BadCode));
        assert_eq!(me.pair(them.public(), &[], &my_secret, &[]), Err(PairError::BadCode));
        assert!(me.pair(them.public(), &my_reply, &my_secret, &their_reply).is_ok());
    }

    #[test]
    fn names_and_numbers() {
        let mut secret: Secret = core::array::from_fn(|i| (i * 7 + 3) as u8);
        assert_eq!(session_name(&secret), "raven-arc-69");
        assert_eq!(check_number(&secret), "83937 38204 37901 13168 48789 77017 60613 21738");
        // Neither is a window onto the secret: a session that was all zeros
        // (as a pairing gone wrong once was) looks like any other.
        assert_eq!(session_name(&[0; SECRET_LEN]), "ivory-tide-14");

        let number = check_number(&secret);
        assert_eq!(number.split(' ').count(), CHECK_GROUPS);
        assert!(number.split(' ').all(|group| group.len() == 5 && group.bytes().all(|b| b.is_ascii_digit())));
        // One bit of the secret, and the number is another number.
        secret[0] ^= 1;
        assert_ne!(check_number(&secret), number);
    }
}
