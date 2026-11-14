# inout

An independent Rust HTTP file service, a static Next.js file manager, and a Tauri desktop host. Share a folder from one device and browse, upload, download or manage it from another device's browser. Product branding, storage providers, feature modules and native lifecycle have separate interfaces. The app is **inout**; the repository and distribution slug remain **inout-next**.

This is an experimental **0.1.0 preview**. [CI](https://github.com/zocs/inout-next/actions) builds and tests Linux x64, Windows x86/x64 and macOS ARM64/x64, including real browser file operations and native packaging. Installation, signing, offline distribution and mobile support have separate milestones; see the [platform matrix](docs/platforms.md). There is no published release yet.

- [中文说明](README_zh.md)
- [Requirements](docs/requirements.md) · [Roadmap](ROADMAP.md) · [Platforms](docs/platforms.md)
- [Architecture](docs/architecture.md) · [API v1](contracts/api-v1.md) · [Modules](docs/modules.md)
- [Branding](docs/branding.md) · [Design system](DESIGN.md) · [Package size and runtime](docs/distribution/size-and-runtime.md)
- [App store preparation](docs/distribution/store-readiness.md) · [Privacy data inventory](docs/distribution/privacy-data-inventory.md)
- [Signing and application identity](docs/distribution/signing.md)

## Run the file service

Rust 1.99.1 and Node 26.13.1 are pinned for building. Python scripts generate configuration, package artifacts and run checks. The delivered CLI and exported pages require neither Node nor Python.

```sh
npm ci --prefix apps/web
npm --prefix apps/web run build
cargo run -p share-cli --locked -- \
  --root /path/to/shared-folder --web-dir apps/web/out \
  --listen 127.0.0.1:5000
```

Open `http://127.0.0.1:5000`. The selected folder must exist and must be separate from the exported UI assets. The default is read-only. Ctrl-C stops the service.

To allow uploads and management, create a private key outside the shared folder, read it locally, and enter it in the browser:

```sh
python3 scripts/create_key.py ./access.key
cargo run -p share-cli --locked -- \
  --root /path/to/shared-folder --web-dir apps/web/out \
  --listen 0.0.0.0:5000 --writable --key-file ./access.key
```

Other devices use the sharing computer's LAN address. An access key protects all file APIs when configured. HTTP does not encrypt the key or transferred files. Select `--modules files` to disable upload and management routes. Run `share-server --help` for transfer limits and alternate product configuration.

## Use the desktop host

Install the Tauri prerequisites for your platform, then:

```sh
npm ci --prefix apps/desktop
npm --prefix apps/desktop run dev
```

The host selects a folder through the native dialog, configures local or LAN access and write permissions, generates an optional access key, starts/stops the shared Rust service and opens its file manager. Reloaded controls display the effective running settings; the status never returns the access key. Closing the manager stops sharing. Android storage and foreground-service integration are later milestones.

The reusable dot-matrix wordmark follows the configured product name. Startup uses a light transition driven by actual readiness, with reduced-motion support and no minimum loading delay.

For temporary local diagnostics, activate the app-name button five times within five seconds in the native host. The panel tunes download buffers from 16 to 128 KiB in 16 KiB steps (default 128 KiB) and runs one read-only loopback timing sample against a relative shared-file path. Tests use 1/2/4/8 connections, read at most the first 64 MiB and time out after 30 seconds. Choose a nonempty file with at least one byte per connection. Existing downloads keep their original buffer. Cancel stops a test; closing diagnostics or stopping sharing restores defaults. These measurements describe the sharing device, rather than another device's network speed. Native diagnostics are unavailable to the HTTP file-manager window.

Linux production compilation:

```sh
npm --prefix apps/desktop run tauri -- build --no-bundle -- --locked
```

## Build and verify

```sh
python3 scripts/sync_product.py --check
python3 -m unittest discover -s tests -v
npm --prefix apps/web run typecheck
npm --prefix apps/web run build
cargo fmt --all -- --check
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check
cargo test --workspace --locked
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo build -p share-cli --locked
npm --prefix apps/web run test:e2e
python3 scripts/verify_runtime.py --mib 128
```

Browser tests use Chromium and the actual Rust service/filesystem. Native command-boundary fixtures are separate from target-device tests. Install Chromium locally or run `npx playwright install --with-deps chromium` from `apps/web`; `PLAYWRIGHT_CHROMIUM_EXECUTABLE` selects another installed binary.

The [GitHub workflow](.github/workflows/ci.yml) has one repository-wide queue. Linux verification precedes the Windows x86/x64 matrix, then the macOS arm64/x64 matrix; every matrix has `max-parallel: 1`. It creates CLI ZIPs and native bundles for implemented desktop targets. Android and offline Windows packages remain platform milestones.

```sh
cargo build -p share-cli --release --locked
python3 scripts/package_portable.py --binary target/release/share-server \
  --target linux-x64 --output .local/packages
```

The ZIP contains the stable server binary, built `web/`, product metadata, license and usage instructions; it excludes private keys and build tools. Native bundles and portable CLI packages have separate runtime requirements.

## Store distribution preparation

Google Play, F-Droid, iOS/iPadOS App Store and Mac App Store have separate dated requirements and acceptance gates in the [store research](docs/distribution/store-readiness.md). Android preparation includes the current API 36 submission floor, native 16 KB checks, SAF directory grants, correctly owned foreground services and a real F-Droid source-build recipe. Apple preparation separates foreground iOS sharing from Mac App Store sandboxing, signing and privacy requirements. Stable app identity, independent build numbers, signing continuity, dependency licences and truthful data/transport disclosures are planned from the start. No Android/iOS build or store acceptance is claimed by the desktop preview.

## Current boundaries

There is no dufs dependency, copied server or sidecar. Next.js exports static assets; Rust implements the file operations. Configuration and compiled module interfaces provide replaceable components; a dynamic third-party plugin ABI is not implemented. Display branding comes from `product.json`, while `config/native-identity.json` preserves native package/install names and the explicit WiX upgrade UUID. Window/browser names and portable filenames can change while OS installation labels remain stable; changing installation identity requires a deliberate migration.

Uploads default to a 2 GiB limit and stream continuously; resume and crash recovery are unfinished. Directory copying is unsupported. Windows file/folder move and rename are conservatively disabled until a validated exclusive implementation exists; uploads/copies require a filesystem supporting hard links there. Linux/Apple publication and moves require native exclusive rename support. Root mutation, traversal, symlink sharing and implicit overwrite are rejected.

Empty reserved `INOUTTMP` directories persist physically after transfers to avoid races between concurrent uploads. Public listings hide them, and cancellation removes unpublished files. A process or OS crash can leave internal files; recovery is a later milestone.

Public UI assets are provider-approved immutable snapshots, bounded to 128 MiB/10,000 files, depth 128, with up to 100,000 shared entries scanned for overlap/identity aliases. Startup can reject an unprovable or oversized share; API-only CLI operation can omit `--web-dir`. Filesystem ranges seek on the original opened handle; stream-only providers keep forward-reading support. Complete ETag/If-Range validation and durable resume are unfinished. Some filesystem work is synchronous, sessions are process-local, and HTTPS reverse proxies need a future explicit origin policy. See [platforms](docs/platforms.md) for support boundaries.
