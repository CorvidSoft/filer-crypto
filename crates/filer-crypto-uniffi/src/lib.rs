//! UniFFI binding layer for filer-crypto.
//!
//! Each UDL type is mirrored here as a thin Rust type. The `Vault` interface
//! becomes a struct holding the core `filer_crypto::Vault` behind a Mutex.
//! UniFFI interfaces require `Send + Sync`; the core Vault is already both,
//! but the Mutex insulates us if any future addition to the core introduces
//! interior mutability that breaks Sync. Lock contention is negligible
//! because crypto operations are short.
//!
//! Byte arrays cross the FFI as `Vec<u8>`. We validate fixed-length inputs
//! (32-byte secrets, 32-byte public keys, 64-byte signatures) inside the
//! wrapper and return `FilerCryptoError::InvalidKeyLength` on mismatch.

use std::sync::Mutex;

use filer_crypto::{
    recovery, DeviceSignature as CoreDeviceSignature, EncryptedBlob as CoreEncryptedBlob,
    EncryptedField as CoreEncryptedField, FilerCryptoError as CoreError, Vault as CoreVault,
};

// ---- Error type -------------------------------------------------------
//
// FilerCryptoError is declared HERE (not imported from the core crate) so
// that `uniffi::include_scaffolding!` can apply `udl_derive(Error)` to the
// local type name without violating Rust's orphan rules.

/// All errors returned across the FFI boundary.
///
/// Variants mirror `filer_crypto::FilerCryptoError` exactly, so a
/// `From` impl can convert with no loss of information.
#[derive(Debug, thiserror::Error)]
pub enum FilerCryptoError {
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

impl From<CoreError> for FilerCryptoError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::Aead => Self::Aead,
            CoreError::InvalidPhrase => Self::InvalidPhrase,
            CoreError::InvalidKeyLength => Self::InvalidKeyLength,
            CoreError::InvalidSignature => Self::InvalidSignature,
            CoreError::Randomness => Self::Randomness,
        }
    }
}

type Result<T> = std::result::Result<T, FilerCryptoError>;

// ---- Dictionary types -------------------------------------------------
//
// EncryptedBlob, EncryptedField, DeviceSignature are declared here so that
// include_scaffolding! can apply udl_derive(Record) to the local names.
// We keep the iv field as Vec<u8> at the FFI boundary (UDL sequence<u8>)
// and validate the fixed 12-byte length when converting back to core types.

#[derive(Debug, Clone)]
pub struct EncryptedBlob {
    pub ciphertext: Vec<u8>,
    pub iv: Vec<u8>,
    pub wrapped_key: Vec<u8>,
}

impl From<CoreEncryptedBlob> for EncryptedBlob {
    fn from(b: CoreEncryptedBlob) -> Self {
        Self {
            ciphertext: b.ciphertext,
            iv: b.iv.to_vec(),
            wrapped_key: b.wrapped_key,
        }
    }
}

impl TryFrom<EncryptedBlob> for CoreEncryptedBlob {
    type Error = FilerCryptoError;
    fn try_from(b: EncryptedBlob) -> Result<Self> {
        let iv: [u8; 12] =
            b.iv.try_into()
                .map_err(|_| FilerCryptoError::InvalidKeyLength)?;
        Ok(CoreEncryptedBlob {
            ciphertext: b.ciphertext,
            iv,
            wrapped_key: b.wrapped_key,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedField {
    pub ciphertext: Vec<u8>,
    pub iv: Vec<u8>,
}

impl From<CoreEncryptedField> for EncryptedField {
    fn from(f: CoreEncryptedField) -> Self {
        Self {
            ciphertext: f.ciphertext,
            iv: f.iv.to_vec(),
        }
    }
}

impl TryFrom<EncryptedField> for CoreEncryptedField {
    type Error = FilerCryptoError;
    fn try_from(f: EncryptedField) -> Result<Self> {
        let iv: [u8; 12] =
            f.iv.try_into()
                .map_err(|_| FilerCryptoError::InvalidKeyLength)?;
        Ok(CoreEncryptedField {
            ciphertext: f.ciphertext,
            iv,
        })
    }
}

/// An Ed25519 signature produced by `Vault::sign_challenge`.
///
/// No `Debug` derive: the bytes are raw signature material that we
/// intentionally never print.
#[derive(Clone)]
pub struct DeviceSignature {
    pub bytes: Vec<u8>,
}

impl From<CoreDeviceSignature> for DeviceSignature {
    fn from(s: CoreDeviceSignature) -> Self {
        Self {
            bytes: s.bytes.to_vec(),
        }
    }
}

// ---- Vault interface --------------------------------------------------
//
// `Vault` is declared here so include_scaffolding! can apply udl_derive(Object)
// to the local type. The struct holds the core Vault behind a Mutex so that
// UniFFI's Arc<Vault> sharing across threads remains safe.

pub struct Vault {
    inner: Mutex<CoreVault>,
}

// ---- Include scaffolding ----------------------------------------------
//
// MUST come after all type declarations above; the scaffolding's
// #[udl_derive(...)] macros reference the names declared above.

uniffi::include_scaffolding!("filer_crypto");

// ---- Top-level function implementations -------------------------------

fn generate_master_secret() -> Vec<u8> {
    // recovery::generate_master_secret returns Result<[u8;32]>. The UDL signature
    // is sequence<u8> with no [Throws] — if the OS CSPRNG truly fails we have no
    // recovery path, so panic with a clear message. This matches Apple's iOS
    // semantics where SecRandomCopyBytes failing is a system-level fault.
    recovery::generate_master_secret()
        .expect("OS CSPRNG unavailable")
        .to_vec()
}

fn secret_to_phrase(secret: Vec<u8>) -> Result<String> {
    let array: [u8; 32] = secret
        .try_into()
        .map_err(|_| FilerCryptoError::InvalidKeyLength)?;
    recovery::secret_to_phrase(&array).map_err(Into::into)
}

fn phrase_to_secret(phrase: String) -> Result<Vec<u8>> {
    recovery::phrase_to_secret(&phrase)
        .map(|s| s.to_vec())
        .map_err(Into::into)
}

fn verify_signature(public_key: Vec<u8>, nonce: Vec<u8>, signature: Vec<u8>) -> Result<()> {
    let pk: [u8; 32] = public_key
        .try_into()
        .map_err(|_| FilerCryptoError::InvalidKeyLength)?;
    let sig: [u8; 64] = signature
        .try_into()
        .map_err(|_| FilerCryptoError::InvalidKeyLength)?;
    filer_crypto::verify_signature(&pk, &nonce, &sig).map_err(Into::into)
}

// ---- Vault method implementations -------------------------------------

impl Vault {
    pub fn open(master_secret: Vec<u8>) -> Result<Self> {
        let array: [u8; 32] = master_secret
            .try_into()
            .map_err(|_| FilerCryptoError::InvalidKeyLength)?;
        let core = CoreVault::open(&array).map_err(FilerCryptoError::from)?;
        Ok(Self {
            inner: Mutex::new(core),
        })
    }

    pub fn from_recovery_phrase(phrase: String) -> Result<Self> {
        let core = CoreVault::from_recovery_phrase(&phrase).map_err(FilerCryptoError::from)?;
        Ok(Self {
            inner: Mutex::new(core),
        })
    }

    pub fn encrypt_blob(&self, plaintext: Vec<u8>) -> Result<EncryptedBlob> {
        let core_blob = self
            .inner
            .lock()
            .unwrap()
            .encrypt_blob(&plaintext)
            .map_err(FilerCryptoError::from)?;
        Ok(core_blob.into())
    }

    pub fn decrypt_blob(&self, blob: EncryptedBlob) -> Result<Vec<u8>> {
        let core_blob: CoreEncryptedBlob = blob.try_into()?;
        self.inner
            .lock()
            .unwrap()
            .decrypt_blob(&core_blob)
            .map_err(FilerCryptoError::from)
    }

    pub fn encrypt_metadata_field(&self, plaintext: Vec<u8>) -> Result<EncryptedField> {
        let core_field = self
            .inner
            .lock()
            .unwrap()
            .encrypt_metadata_field(&plaintext)
            .map_err(FilerCryptoError::from)?;
        Ok(core_field.into())
    }

    pub fn decrypt_metadata_field(&self, field: EncryptedField) -> Result<Vec<u8>> {
        let core_field: CoreEncryptedField = field.try_into()?;
        self.inner
            .lock()
            .unwrap()
            .decrypt_metadata_field(&core_field)
            .map_err(FilerCryptoError::from)
    }

    pub fn sign_challenge(&self, nonce: Vec<u8>) -> DeviceSignature {
        self.inner.lock().unwrap().sign_challenge(&nonce).into()
    }

    pub fn device_public_key(&self) -> Vec<u8> {
        self.inner.lock().unwrap().device_public_key().to_vec()
    }
}
