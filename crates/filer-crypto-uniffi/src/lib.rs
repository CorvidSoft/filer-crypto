//! UniFFI binding layer for filer-crypto.
//!
//! Wraps the public API of `filer-crypto` in UniFFI-compatible types and
//! glues them to the UDL definition in `filer_crypto.udl`.

// Empty for now — Task 10 fills this in with the actual wrapper types.
// The UDL file in the same directory has no scaffolding until Task 10 either.

uniffi::include_scaffolding!("filer_crypto");
