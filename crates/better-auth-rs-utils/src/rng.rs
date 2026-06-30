//! OS-backed CSPRNG adapter. **Rust-only — no upstream `.ts`** (in JS the Web Crypto `SubtleCrypto`
//! seeds key generation internally; here we must hand the RustCrypto APIs an RNG).
//!
//! The RC asymmetric crates (`ecdsa`, `rsa`) take `&mut impl rand_core::CryptoRng` for key
//! generation and signing. rand_core 0.10 dropped the `os_rng` feature, so we adapt the OS CSPRNG
//! (`getrandom`) to its trait hierarchy: implementing infallible `TryRng`/`TryCryptoRng` makes
//! `Rng`/`CryptoRng` available via rand_core's blanket impls.

use core::convert::Infallible;

use rand_core::{TryCryptoRng, TryRng};

/// A stateless handle to the operating-system CSPRNG (`getrandom`).
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct OsCsprng;

impl TryRng for OsCsprng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        let mut b = [0u8; 4];
        self.try_fill_bytes(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        let mut b = [0u8; 8];
        self.try_fill_bytes(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }

    // The trait fixes `Error = Infallible`, so the only failure mode (the OS CSPRNG being
    // unavailable) must panic rather than return — matching `random.rs`'s policy.
    #[allow(clippy::expect_used)]
    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        getrandom::fill(dst).expect("OS CSPRNG unavailable");
        Ok(())
    }
}

impl TryCryptoRng for OsCsprng {}
