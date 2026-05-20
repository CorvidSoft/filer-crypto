use thiserror::Error;

/// All errors returned by this crate.
///
/// Variants are intentionally coarse — the variant name carries the diagnostic.
/// We do not expose cause chains or position info because they could leak
/// information about key material or input shape.
#[derive(Debug, Error)]
pub enum FilerCryptoError {
    /// AEAD encryption or decryption failed. The same variant covers both
    /// directions because the AEAD library returns the same opaque error type
    /// for either, and exposing more detail risks leaking timing or position
    /// information. Decryption failure is the common case (tag mismatch,
    /// wrong key, tampered ciphertext); encryption failure is rare (only
    /// reachable via the buffer-too-small condition that the Vec-based API
    /// never hits).
    #[error("AEAD operation failed")]
    Aead,
    #[error("invalid recovery phrase")]
    InvalidPhrase,
    #[error("invalid key length")]
    InvalidKeyLength,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("randomness source unavailable")]
    Randomness,
}

pub type Result<T> = std::result::Result<T, FilerCryptoError>;
