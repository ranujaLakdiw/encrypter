//! Session pairing with ML-KEM-1024 (FIPS 203).
//!
//! Both people run the same flow: each makes a keypair and sends the public key,
//! each encapsulates to the other's key (that ciphertext is the "reply"), and each
//! decapsulates the reply they receive. The session key is the XOR of the two
//! shared secrets, which is the same on both sides whoever went first.

use ml_kem::{
    kem::{Decapsulate, Encapsulate, FromSeed, KeyExport, TryKeyInit},
    DecapsulationKey, EncapsulationKey, MlKem1024,
};

use crate::rng::{self, PlatformRng};

pub const SECRET_LEN: usize = 32;
pub type Secret = [u8; SECRET_LEN];

/// The other person's key or reply was not a valid ML-KEM-1024 one.
#[derive(Debug, PartialEq, Eq)]
pub struct BadCode;

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
        seed.fill(0);
        Self { dk, public: ek.to_bytes().to_vec() }
    }

    /// My public key, to send to the other person.
    pub fn public(&self) -> &[u8] {
        &self.public
    }

    /// Opens the reply the other person made for my public key.
    pub fn decapsulate(&self, their_reply: &[u8]) -> Result<Secret, BadCode> {
        let shared = self.dk.decapsulate_slice(their_reply).map_err(|_| BadCode)?;
        Ok(shared.into())
    }
}

/// Makes my reply for the other person's public key: (reply to send, my shared secret).
pub fn encapsulate(their_public: &[u8]) -> Result<(Vec<u8>, Secret), BadCode> {
    let ek = EncapsulationKey::<MlKem1024>::new_from_slice(their_public).map_err(|_| BadCode)?;
    let (reply, shared) = ek.encapsulate_with_rng(&mut PlatformRng);
    Ok((reply.to_vec(), shared.into()))
}

/// The session's secret: both shared secrets XORed together.
pub fn combine(mine: &Secret, theirs: &Secret) -> Secret {
    let mut out = [0u8; SECRET_LEN];
    for (o, (a, b)) in out.iter_mut().zip(mine.iter().zip(theirs)) {
        *o = a ^ b;
    }
    out
}

const WORDS_A: [&str; 20] = [
    "amber", "brave", "crisp", "delta", "ember", "faint", "ghost", "haven", "ivory", "jewel",
    "kite", "lunar", "mist", "neon", "onyx", "pearl", "quill", "raven", "slate", "thorn",
];
const WORDS_B: [&str; 20] = [
    "arc", "bay", "cog", "dune", "echo", "floe", "gust", "haze", "isle", "jade", "knot", "lore",
    "maze", "nova", "orb", "pike", "reef", "sage", "tide", "vale",
];

/// A human-readable name for a session secret. Both people see the same one, which
/// confirms the exchange worked.
pub fn session_name(secret: &Secret) -> String {
    let a = WORDS_A[secret[0] as usize % WORDS_A.len()];
    let b = WORDS_B[secret[1] as usize % WORDS_B.len()];
    format!("{a}-{b}-{:02}", secret[2] % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sides_agree() {
        let (alice, bob) = (KeyPair::generate(), KeyPair::generate());
        assert_eq!(alice.public().len(), 1568);
        let (reply_a, secret_a) = encapsulate(bob.public()).unwrap();
        let (reply_b, secret_b) = encapsulate(alice.public()).unwrap();
        assert_eq!(reply_a.len(), 1568);
        let alice_key = combine(&secret_a, &alice.decapsulate(&reply_b).unwrap());
        let bob_key = combine(&secret_b, &bob.decapsulate(&reply_a).unwrap());
        assert_eq!(alice_key, bob_key);
        assert_eq!(session_name(&alice_key), session_name(&bob_key));
    }

    #[test]
    fn refuses_codes_of_the_wrong_shape() {
        assert_eq!(encapsulate(&[1, 2, 3]).err(), Some(BadCode));
        assert_eq!(encapsulate(&[0xff; 1568]).err(), Some(BadCode));
        assert_eq!(KeyPair::generate().decapsulate(&[0; 10]).err(), Some(BadCode));
    }

    #[test]
    fn names() {
        let mut s = [0u8; 32];
        assert_eq!(session_name(&s), "amber-arc-00");
        s[..3].copy_from_slice(&[21, 39, 207]);
        assert_eq!(session_name(&s), "brave-vale-07");
    }
}
