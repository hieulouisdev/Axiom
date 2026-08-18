# Reproducible Builds

How to build Aegis AI from source and verify it matches released binaries.

---

## Why Reproducible Builds?

If two independent builders produce bit-for-bit identical artifacts from the same source, we can be confident no malicious code was injected during CI.

---

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | 1.97.1 | `rust-toolchain.toml` |
| Node.js | 20.x | LTS |
| npm | locked | `package-lock.json` |

---

## Pinning

- **Rust**: `Cargo.lock` committed — never `cargo update` on release branch without review
- **Node.js**: always `npm ci` (not `npm install`) in CI

---

## Deterministic Timestamps

```bash
export SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
```

Affects: `CARGO_PKG_VERSION`, Tauri metadata, archive modification times.

---

## Auditing

```bash
cargo install cargo-audit && cargo audit           # Rust vulnerabilities
npm audit && npm audit signatures                   # Node.js vulnerabilities
cargo install cargo-supply-chain && cargo supply-chain check  # provenance
```

---

## Build Instructions

### Linux (x86_64)

```bash
git clone --branch=v0.9.0 https://github.com/hieulouisdev/Axiom.git
cd Axiom
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
export SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
npm ci
cargo tauri build --target x86_64-unknown-linux-gnu
# Output: src-tauri/target/release/bundle/{deb,appimage,rpm}/
```

### Windows (x86_64)

```powershell
git clone --branch=v0.9.0 https://github.com/hieulouisdev/Axiom.git
cd Axiom
$env:SOURCE_DATE_EPOCH = (git log -1 --format=%ct)
npm ci
cargo tauri build --target x86_64-pc-windows-msvc
# Output: src-tauri\target\release\bundle\{msi,nsis}/
```

---

## Verifying a Release

1. Build from source using instructions above
2. `sha256sum src-tauri/target/release/bundle/**/*.{deb,AppImage,rpm,msi,exe}`
3. Compare against checksums published in GitHub release assets
4. Match = verified

---

## CI Enforcement

`.github/workflows/build-release.yml` runs on push to `main` and version tags (`v*`). Builds on Linux + Windows, generates SHA-256 checksums, creates GitHub release on tag push.

---

## Future

- [ ] Sign binaries with GPG or Sigstore
- [ ] `tauri-plugin-updater` pubkey verification
- [ ] `diffoscope`-based reproducibility verification
