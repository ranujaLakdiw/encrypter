//! Huh encrypter.
//!
//! Crypto: AES with either a shared password or an ML-KEM-1024 session secret.
//! Output is "huH": base-3 text over the alphabet h / u / H, chunks joined with U.
//! Files and images go through the same pipe: their bytes (plus a small name/type
//! header) are encrypted and packed into a .huh. Text and files alike are squeezed
//! with LZMA2 before AES when that makes the huH shorter.
//!
//! Everything here except `app` is plain Rust with no browser in it, so it is
//! tested natively with `cargo test`. `app` is the page itself (Leptos), built
//! only for WebAssembly.

pub mod b64;
pub mod cipher;
pub mod huh;
pub mod kem;
pub mod locker;
pub mod payload;
pub mod rng;
pub mod squeeze;
pub mod words;

#[cfg(target_arch = "wasm32")]
mod app;
