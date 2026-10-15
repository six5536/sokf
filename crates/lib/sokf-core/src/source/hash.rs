//! The tool uses one hash, 64-bit Fowler–Noll–Vo (FNV-1a), because it gives the same value on
//! every Rust version, so a store and a harness record stay valid after the
//! toolchain changes (CLI-3_AC-53, CACHE-1_AC-6).

/// `Fnv` computes a 64-bit FNV-1a hash over bytes fed to it in pieces.
#[derive(Debug, Clone, Copy)]
pub struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self::new()
    }
}

impl Fnv {
    /// `new` starts a hash at the FNV-1a offset basis, the hash of no bytes.
    pub fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }

    /// `update` adds `bytes` to the hash.
    pub fn update(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    /// `finish` returns the hash of every byte fed so far.
    pub fn finish(&self) -> u64 {
        self.0
    }
}

/// `fnv1a64` returns the 64-bit FNV-1a hash of `bytes`.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = Fnv::new();
    h.update(bytes);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_is_fnv1a() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        let mut h = Fnv::default();
        h.update(b"a");
        assert_eq!(h.finish(), fnv1a64(b"a"));
    }
}
