use thiserror::Error;

/// All errors returned by this crate.
///
/// Variants are intentionally coarse — the variant name carries the diagnostic.
/// We do not expose cause chains or position info because they could leak
/// information about key material or input shape.
#[derive(Debug, Error)]
pub enum FilerCryptoError {
    #[error("decryption failed")]
    Decrypt,
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
