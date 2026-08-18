# Reproducible Builds

Aegis AI supports reproducible builds so that anyone can verify the released
binaries match the source code exactly. This document describes the
requirements and step-by-step instructions.

---

## Why Reproducible Builds?

Reproducible builds eliminate trust in the CI pipeline: if two independent
builders produce bit-for-bit identical artifacts from the same source, we can
be confident no malicious code was injected during the build process.

---

## Prerequisites

| Tool       | Version         | Notes                                  |
|------------|-----------------|----------------------------------------|
| Rust       | 1.97.1          | Pinned via `rust-toolchain.toml`       |
| Node.js    | 20.x            | LTS                                    |
| npm        | locked          | Via `package-lock.json`                |

---

## Pinning Dependencies

### Rust (Cargo.lock)

All transitive Rust dependencies are pinned in `Cargo.lock`, which is
committed to the repository. `cargo build` automatically uses this file,
so no extra step is needed. **Never run `cargo update` on a release
branch without review.**

### Node.js (package-lock.json)

All frontend dependencies are pinned in `package-lock.json`. Always use
`npm ci` (not `npm install`) in CI and local builds to ensure exact
dependency resolution.

---

## Deterministic Timestamps

Set the `SOURCE_DATE_EPOCH` environment variable to an epoch timestamp
derived from the git commit date to ensure all embedded timestamps are
deterministic:

```bash
export SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
```

This affects:
- Cargo's `CARGO_PKG_VERSION` and build timestamps
- Tauri's bundled metadata
- Zip/archive modification times

---

## Auditing Dependencies

### cargo supply-chain

[`cargo supply-chain`](https://github.com/marketplace/cargo-supply-chain)
audits the provenance and review status of every Rust dependency:

```bash
cargo install cargo-supply-chain
cargo supply-chain update
cargo supply-chain check
```

This checks each crate against the [cargo-audit](https://rustsec.github.io/)
advisory database and lists which dependencies have been reviewed by
trusted parties.

### cargo audit

```bash
cargo install cargo-audit
cargo audit
```

The advisory database configuration lives in `cargo-audit.toml` at the
project root.

### npm audit

```bash
npm audit
npm audit signatures  # verify registry signatures (npm 9+)
```

---

## Build Instructions

### Linux (x86_64)

```bash
# 1. Clone the exact release tag
git clone --branch=v0.6.0 https://github.com/hieulouisdev/Axiom.git
cd Axiom

# 2. Install system dependencies (Debian/Ubuntu)
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev

# 3. Set deterministic timestamp
export SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)

# 4. Install frontend deps (exact lockfile)
npm ci

# 5. Build
cargo tauri build --target x86_64-unknown-linux-gnu

# 6. Artifacts are in:
#    src-tauri/target/release/bundle/deb/*.deb
#    src-tauri/target/release/bundle/appimage/*.AppImage
#    src-tauri/target/release/bundle/rpm/*.rpm
```

### Windows (x86_64)

```powershell
# 1. Clone the exact release tag
git clone --branch=v0.6.0 https://github.com/hieulouisdev/Axiom.git
cd Axiom

# 2. Set deterministic timestamp (PowerShell)
$env:SOURCE_DATE_EPOCH = (git log -1 --format=%ct)

# 3. Install frontend deps (exact lockfile)
npm ci

# 4. Build
cargo tauri build --target x86_64-pc-windows-msvc

# 5. Artifacts are in:
#    src-tauri\target\release\bundle\msi\*.msi
#    src-tauri\target\release\bundle\nsis\*.exe
```

---

## Verifying a Release

1. Build from source using the instructions above.
2. Compute SHA-256 checksums of your local artifacts:
   ```bash
   sha256sum src-tauri/target/release/bundle/**/*.{deb,AppImage,rpm,msi,exe}
   ```
3. Compare against the checksums published in the GitHub release assets
   (`checksums-linux.txt`, `checksums-windows.txt`).
4. If all checksums match, the release is verified.

---

## CI Enforcement

The `.github/workflows/build-release.yml` workflow:

- Runs on every push to `main` and on version tag pushes (`v*`).
- Builds on both `ubuntu-latest` and `windows-latest`.
- Generates SHA-256 checksums and uploads them as workflow artifacts.
- On tag push, `tauri-apps/tauri-action` automatically creates a GitHub
  release with all bundled installers.
- The `concurrency` group ensures only one build per ref runs at a time.

---

## Future Improvements

- [ ] Sign release binaries with a GPG key or Sigstore.
- [ ] Enable `tauri-plugin-updater` pubkey verification (set `pubkey` in
      `tauri.conf.json` once signing is in place).
- [ ] Set up `diffoscope`-based reproducibility verification between
      independent builders.
- [ ] Add a `V=1` verbose cargo build mode for debugging non-reproducible
      artifacts.
