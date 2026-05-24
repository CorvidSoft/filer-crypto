# Versioning

filer-crypto follows semver (MAJOR.MINOR.PATCH).

## The one rule that overrides everything else

Anything that changes the bytes a consumer's existing vault depends on
is a MAJOR bump. Existing vaults stop decrypting on a MAJOR bump — that
is the entire reason MAJOR exists in this crate.

## Classification

### MAJOR — existing vaults stop decrypting

- Envelope struct changes: field name / order / length in `EncryptedBlob`,
  `EncryptedField`, or `DeviceSignature`.
- Wrapped-key layout change (currently `IV(12) || GCM ciphertext+tag`).
- HKDF context strings (`WRAP_CTX`, `METADATA_CTX`, `SIGN_CTX` in `kdf.rs`).
  The `v1` in `filer-crypto/v1/...` exists so that v2 context strings can
  be added later without rotating the v1 ones — but adding a v2 context
  that an existing `Vault` produces is itself a MAJOR change to the
  produced envelopes. See `CLAUDE.md` invariant #8.
- Switching AEAD, KDF, signature scheme, or recovery-phrase wordlist.
- Removing or renaming any `pub` method on `Vault` or any `pub` free
  function exported through the UDL.

### MINOR — additive only

- New methods on `Vault` that don't change the meaning of existing ones.
- New free functions in `recovery.rs` or new modules.
- New error variants on `FilerCryptoError`. Adding variants is
  source-breaking for `match` consumers without a wildcard; we tolerate
  this as MINOR because the variants are intentionally coarse and
  external matchers should use a wildcard.
- New UDL surface that exposes already-public Rust API to Swift.

### PATCH — internal only

- Bug fixes that don't change envelope bytes or the public API.
- Dependency bumps within semver-compatible ranges.
- Documentation, CI, test-only changes.
- Performance improvements with byte-for-byte equivalent output.

## XCFramework + Swift Package versioning

The Swift Package version tracks the Rust crate version. A `v0.2.0` tag
produces a `v0.2.0` GitHub Release with an XCFramework artifact;
`Package.swift`'s `.binaryTarget` URL on `main` points at the latest
published release.

A pre-1.0 MAJOR bump (e.g. `0.1.0 → 0.2.0`) carries the same
break-the-vault implications as a post-1.0 MAJOR. Pre-1.0 does not mean
"we can break vaults silently" — it means "we haven't promised forward
compatibility yet."

## Release procedure

1. Bump `workspace.package.version` in `Cargo.toml`.
2. Update `Package.swift`: `url:` to the new tag, `checksum:` to the
   expected sha256. The first release uses a `-rc1` tag to learn the
   actual checksum; subsequent releases compute it locally first by
   running `./scripts/build-xcframework.sh` and reading the output.
3. Commit (`chore: release v<X.Y.Z>`), tag (`git tag v<X.Y.Z>`), push the
   tag. CI builds the XCFramework on macOS, computes the checksum,
   creates the GitHub Release, and uploads the asset.
4. If the post-publish checksum doesn't match what's in `Package.swift`,
   delete the tag (`git push --delete origin v<X.Y.Z>`) and release
   (`gh release delete v<X.Y.Z>`), fix the checksum, re-tag.

## When in doubt

If a change *might* alter envelope bytes for any plausible input, run
the cross-language fixture tests (`Tests/FilerCryptoTests/Fixtures/`).
If they fail after your change, that's a MAJOR.
