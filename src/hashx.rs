//! SHA-256 hex digests — the single source for every hash lean-md writes or
//! compares (the `lean-md.lock` provenance values and the pack-drift manifest).
//! Two definitions of "how we hash" would be exactly the drift this module exists
//! to make loud. Output matches coreutils `sha256sum` byte for byte, so a user can
//! re-check any value we emit without trusting us.

use sha2::{Digest, Sha256};

/// Lowercase 64-char hex digest of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_known_sha256_of_abc() {
        // NIST FIPS 180-4 test vector — proves we produce what `sha256sum` produces.
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn empty_input_has_the_canonical_digest() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
