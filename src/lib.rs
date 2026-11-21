//! Huh encrypter.
//!
//! Crypto: AES-256-GCM, under a key made from either a shared password
//! (PBKDF2-HMAC-SHA256) or the secret of a session paired with ML-KEM-1024.
//! Output is "huH": base-3 text over the alphabet h / u / H, chunks joined with U.
//! Files and images go through the same pipe: their bytes (plus a small name/type
//! header) are encrypted and packed into a .huh. Text and files alike are squeezed
//! with LZMA2 before AES when that makes the huH shorter. A session can be kept on
//! the device between visits, sealed under a passphrase (`keep`).
//!
//! `envelope` is how huHs are sealed today; `cipher` is how they were sealed
//! before, and is only here to open those.
//!
//! Everything here except `app` is plain Rust with no browser in it, so it is
//! tested natively with `cargo test`. `app` is the page itself (Leptos), built
//! only for WebAssembly.

pub mod b64;
pub mod cipher;
pub mod envelope;
pub mod huh;
pub mod kdf;
pub mod keep;
pub mod kem;
pub mod locker;
pub mod payload;
pub mod rng;
pub mod squeeze;
pub mod words;

#[cfg(target_arch = "wasm32")]
mod app;
