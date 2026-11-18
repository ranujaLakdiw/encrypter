//! Random bytes from the platform: `crypto.getRandomValues` in the browser, the
//! operating system everywhere else (which is only the test suite).

use core::convert::Infallible;
use rand_core::{TryCryptoRng, TryRng};

#[cfg(target_arch = "wasm32")]
pub fn fill(buf: &mut [u8]) {
    let crypto = web_sys::window()
        .and_then(|w| w.crypto().ok())
        .expect("this browser has no crypto.getRandomValues");
    // getRandomValues hands out at most 65536 bytes a call.
    for chunk in buf.chunks_mut(65_536) {
        crypto
            .get_random_values_with_u8_array(chunk)
            .expect("crypto.getRandomValues failed");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn fill(buf: &mut [u8]) {
    getrandom::fill(buf).expect("no system randomness");
}

/// The platform's randomness as a `rand_core` generator, for ML-KEM.
pub struct PlatformRng;

impl TryRng for PlatformRng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        let mut b = [0u8; 4];
        fill(&mut b);
        Ok(u32::from_le_bytes(b))
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        let mut b = [0u8; 8];
        fill(&mut b);
        Ok(u64::from_le_bytes(b))
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        fill(dst);
        Ok(())
    }
}

impl TryCryptoRng for PlatformRng {}
