# Publish FilerCrypto.swift as a Release Asset — Design

**Date:** 2026-06-08
**Status:** Approved, implementing directly (small change, spec-review gate skipped per maintainer)
**Tracks:** [filer-crypto#3](https://github.com/CorvidSoft/filer-crypto/issues/3)
**Prior spec:** [2026-05-24 distribution design](./2026-05-24-filer-crypto-distribution-design.md)

## 1. Goal

Make the generated high-level Swift API, `FilerCrypto.swift`, a published artifact
of the release pipeline so the Filer iOS app can consume it.

## 2. The gap

The release pipeline publishes `FilerCryptoFFI.xcframework.zip`, which contains the
compiled staticlib plus the C header and modulemap (inside each slice's `Headers/`).
It does **not** contain `FilerCrypto.swift` — the UniFFI-generated high-level Swift
wrapper. That file lives only as committed package source at
`Sources/FilerCrypto/FilerCrypto.swift` and today reaches consumers through the SPM
git checkout at the tag.

The Filer iOS app does not consume via an SPM git checkout. Its Expo
`with-crypto-core` config plugin downloads the GitHub Release **assets** directly.
It therefore picks up the XCFramework but never sees `FilerCrypto.swift`, so the
high-level Swift API is missing on the app side.

## 3. Decision

Publish `Sources/FilerCrypto/FilerCrypto.swift` as a standalone raw release asset,
with its sha256 recorded in the release notes for parity with the XCFramework.

- **Just `FilerCrypto.swift`.** The C header and modulemap are already inside the
  XCFramework; only the Swift wrapper is missing.
- **Raw `.swift` file**, not zipped — one file, no unzip step, integrity covered by
  the per-file sha256.
- **sha256 in release notes**, matching how the XCFramework checksum is published.

## 4. Why publishing the committed file is safe (the pairing invariant)

`scripts/build-xcframework.sh` runs a bindings-drift check during the release: it
regenerates the Swift bindings into a temp dir and **aborts the release** if the
committed `Sources/FilerCrypto/FilerCrypto.swift` differs from what the current Rust
source produces. So by the time the release is created, the committed file is proven
to be the correct pair for the XCFramework built in the same run. No separate
generation step is needed, and the published asset always matches the repo source at
the tag commit.

This is why approach A (attach the committed file) is preferred over regenerating the
file in the release job — regeneration would duplicate a guarantee that already
exists and risk publishing bytes that diverge from the tag's committed source.

## 5. Changes

### 5.1 `.github/workflows/release.yml`

- **"Compute checksums" step** (`id: sum`): additionally compute the sha256 of
  `Sources/FilerCrypto/FilerCrypto.swift` and expose it as a step output
  `swift_sha256`.
- **"Create GitHub Release" step**: add `Sources/FilerCrypto/FilerCrypto.swift` to
  the `gh release create` asset list (uploaded as asset `FilerCrypto.swift`, basename
  default) and pass both checksums to `release-notes.sh`.

### 5.2 `scripts/release-notes.sh`

- Accept an optional second positional arg: the `FilerCrypto.swift` sha256.
- When present, the "Artifacts" section lists `FilerCrypto.swift` with its predictable
  download URL (`…/releases/download/<tag>/FilerCrypto.swift`) and sha256, alongside
  the XCFramework entry and the existing `.binaryTarget` snippet.
- The second arg is **optional** so the manual-recovery invocations documented in
  `docs/VERSIONING.md` keep working when only the XCFramework sha is supplied.

### 5.3 `docs/VERSIONING.md`

- Update the partial-failure recovery table so the manual `gh release create` /
  `release-notes.sh` commands also attach `FilerCrypto.swift` and pass its sha — a
  recovery run then produces the same asset set as the happy path.

## 6. Out of scope

- No change to `.github/workflows/ci.yml`: the bindings-freshness check already lives
  there and is what makes publishing the committed file safe.
- No change to `Package.swift`: SPM consumers still get `FilerCrypto.swift` via the
  source target; this asset serves the asset-download consumption path.
- No standalone header/modulemap assets: they are bundled inside the XCFramework.

## 7. Verification

- Run `./scripts/release-notes.sh <xcf-sha> <swift-sha>` and confirm the Artifacts
  section lists both files with correct URLs and checksums.
- Run `./scripts/release-notes.sh <xcf-sha>` (one arg) and confirm it still emits
  valid notes without the Swift entry (recovery path).
- Lint the workflow YAML (`actionlint` if available) and shell scripts
  (`shellcheck`/`bash -n`).
- First real validation is the next `gh release` run, where the asset appears on the
  release and the plugin can fetch it.
