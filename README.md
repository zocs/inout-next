# inout

An independent Rust HTTP file service, a static Next.js file manager and an opt-in Slint native desktop trial. Existing Tauri desktop/Android packages remain transitional previews. Share a folder on desktop or app-private files in the Android foreground preview, then browse, upload, download or manage them from another device's browser. Product branding, storage providers, feature modules and native lifecycle have separate interfaces. The app is **inout**; the repository and distribution slug remain **inout-next**.

The final application uses a Rust-rendered native GUI. Next.js currently builds the HTTP/browser manager; a [planned Vite + React migration](ROADMAP.md#technology-direction) keeps its components and file-service contracts. **Slint is tried first; egui is the fallback** if required platform, accessibility, distribution or measured performance gates cannot be met. The functional desktop trial controls the real Rust service without WebView. [Native GUI evaluation](docs/native-gui.md) distinguishes this trial, exploratory comparisons and adoption; no replacement native release is published yet.

This is an experimental **0.1.0 preview**. Download [desktop preview.1](https://github.com/zocs/inout-next/releases/tag/v0.1.0-preview.1) or [Android arm64 preview.2](https://github.com/zocs/inout-next/releases/tag/v0.1.0-preview.2), with separate [desktop](docs/distribution/preview.md) and [Android](docs/distribution/android-preview.md) usage guides. [CI](https://github.com/zocs/inout-next/actions) covers Linux x64, Android arm64 packaging, Windows x86/x64 and macOS ARM64/x64 in one serial queue. Android runtime checks use an API 36 x86_64 emulator with 4 KiB pages; physical arm64, 16 KiB operation and two-device Wi-Fi remain unverified. Installation, offline distribution and store acceptance have separate milestones in the [platform matrix](docs/platforms.md). iOS adaptation is scheduled later. No stable release is published yet.

- [中文说明](README_zh.md)
- [Requirements](docs/requirements.md) · [Roadmap](ROADMAP.md) · [Platforms](docs/platforms.md)
- [Architecture](docs/architecture.md) · [API v1](contracts/api-v1.md) · [Modules](docs/modules.md)
- [Branding](docs/branding.md) · [Design system](DESIGN.md) · [Package size and runtime](docs/distribution/size-and-runtime.md)
- [Network addresses and QR](docs/network-selection.md)
- [Native GUI candidates and performance gates](docs/native-gui.md)
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

Add `--writable` to explicitly allow uploads, directory creation and supported rename/move/copy operations without requiring an access key. Published-file deletion remains disabled without a key. To protect all file access and enable deletion, create a private key outside the shared folder, read it locally, and enter it in the browser:

```sh
python3 scripts/create_key.py ./access.key
cargo run -p share-cli --locked -- \
  --root /path/to/shared-folder --web-dir apps/web/out \
  --listen 0.0.0.0:5000 --writable --key-file ./access.key
```

Other devices use the sharing computer's LAN address. An access key protects all file APIs when configured. HTTP does not encrypt the key or transferred files. Select `--modules files` to disable upload and management routes. Run `share-server --help` for transfer limits and alternate product configuration.

Current source includes English, Simplified Chinese and Traditional Chinese, system-language selection and four local themes. Changing language/theme preserves file and transfer state. These additions and the optional-key write policy are unreleased source changes; the existing preview downloads retain their earlier behavior.

The browser header labels the current permissions. Selecting a file opens an inline preview when downloading is allowed: text reads at most 256 KiB, images above 32 MiB wait for an explicit request, and audio/video use the browser's supported codecs. HTML, SVG, PDF and unknown formats remain downloads. Refresh reloads selected content even if its size and modification time are unchanged. Directory Details remains available with an independent rename or delete permission, and delayed management replies preserve a later selection or folder draft.

## Try the native desktop app

Build the static web export above, install the target's native graphics/font prerequisites, then:

```sh
cargo run --manifest-path apps/native-slint/Cargo.toml --locked
```

Choose a folder through the picker, common-folder shortcuts or a path beginning with `~`. Choose local/LAN access, port and one of three access presets: view/download, also upload, or also manage. A key is optional; deletion requires configured-key authentication. The connection panel labels actual addresses and draws a QR code containing the URL only. Linux/Android classify interfaces conservatively; Wi-Fi and wired candidates lead, virtual links follow, and loopback stays last. Select an address to copy it or open it in your system browser. Stop and closing a running host ask for confirmation; Keep sharing preserves the listener and admitted transfers. Current-run transfer receipts remain after Stop and reset at the next successful Start. Language/theme changes preserve sharing and drafts.

The trial supplies English, Simplified/Traditional Chinese, four accents with system/light/dark appearance, themed controls and a native dot-matrix wordmark. Language, appearance and setup drafts are saved in the per-user application configuration directory after presentation changes, Start and ordinary close; the access key is never saved. Stop retains the key draft in the current window's memory; ordinary close discards it. `INOUT_NATIVE_PREFERENCES` selects another file, and an empty value disables saving. `INOUT_NATIVE_REDUCE_MOTION=1` disables startup motion. Native OS motion detection, network-change subscriptions, hotspot classification, durable history/resume, individual cancellation and live-setting/diagnostic controls remain pending.

Current source prevents the key field from exporting text through Copy, Cut or PRIMARY selection, including after Show, while preserving selection, paste and composition. New Linux X11 checks verify clipboard preservation, authenticated access after Cut/replacement paste, ordinary address copying and Rime composition in folder/key fields. Show displays the key and exposes that displayed text to the tested assistive interface. Full screen-reader, broader font/IME and Windows/macOS/Wayland input behavior remain adoption gates; [native verification](docs/native-gui.md) records the scope.

Release GUI builds require a complete static web export and embed it in the executable. The new native ZIP needs no adjacent `web/` directory, so it can share a folder containing the application itself. Debug and renderer-free builds may omit the export and retain the development resource fallback. The browser pages remain external HTTP content; the native window uses no WebView.

For optimized native ZIP packaging, system dependencies and separate MIT/framework licence terms, see the [native trial guide](apps/native-slint/README.md). Build tools are not runtime dependencies. The trial archive does not establish installation, old-OS, mobile or store acceptance.

Windows source keeps FemtoVG preferred and adds software rendering when OpenGL initialization fails. The earlier x86/x64 trial passed real Windows10x64 startup, localized readback, CJK-root file operations and original-upload continuity through language/theme changes and canceled Stop/close, followed by normal cleanup. Those results precede the current authored-control redesign. [Windows verification](docs/native-gui.md#windows-default-renderer-and-input) records the scope; the redesigned controls, actual 32-bit OS, IME/screen readers, installed/offline behavior and presented-frame performance need their own checks.

[Earlier Slint trial CI artifacts at d42048d](https://github.com/zocs/inout-next/actions/runs/44239092778) include Linux x64, Windows x86/x64 and macOS ARM64/x64 ZIPs with the Windows fallback/value-write fixes. They precede the embedded export, QR and authored-control redesign and retain their original adjacent `web/` layout. Windows x86/x64 ZIPs are 6.95/6.99 MiB; their exact packaged executables passed startup, three-language/theme retention and normal-close probes. These remain trial artifacts, separate from the published previews. [Package measurements](docs/distribution/size-and-runtime.md#windows-fallback-and-input-ci-packages) retain that checkpoint and its runtime limits.

## Use the transitional Tauri host

Install the Tauri prerequisites for your platform, then:

```sh
npm ci --prefix apps/desktop
npm --prefix apps/desktop run dev
```

The desktop host selects a folder through the native dialog, configures local or LAN access and write permissions, generates an optional access key, starts/stops the shared Rust service and opens its file manager. Reloaded controls display the effective running settings; the status never returns the access key. Closing the manager stops sharing.

The Rust service library supports atomic runtime permission/key updates and bounded transfer observations. Already-admitted work retains its policy snapshot. Native live-setting controls and browser transfer ownership through authentication changes remain pending; the current host still changes access settings between service sessions.

The reusable dot-matrix wordmark follows the configured product name. Startup uses a light transition driven by actual readiness, with reduced-motion support and no minimum loading delay. Cleanup follows computed CSS timing with a one-second ceiling; ready controls remain usable throughout the exit.

For temporary local diagnostics, activate the app-name button five times within five seconds in the native host. The panel tunes download buffers from 16 to 128 KiB in 16 KiB steps (default 128 KiB) and runs one read-only loopback timing sample against a relative shared-file path. Tests use 1/2/4/8 connections, read at most the first 64 MiB and time out after 30 seconds. Choose a nonempty file with at least one byte per connection. Existing downloads keep their original buffer. Cancel stops a test; closing diagnostics or stopping sharing restores defaults. These measurements describe the sharing device, rather than another device's network speed. Native diagnostics are unavailable to the HTTP file-manager window.

Linux production compilation:

```sh
npm --prefix apps/desktop run tauri -- build --no-bundle -- --locked
```

## Use the Android preview

The Android host shares its own app storage with a browser on another device. Enable uploads/management and generate an access key, then start sharing and use a displayed local-network address. Uploaded files survive Stop, restart and Activity recreation; clearing app storage or uninstalling removes them. The native preview has no arbitrary-folder picker, file-import UI or local manager window.

Keep the app on screen. Home or system Back stops sharing and cancels incomplete transfers; returning leaves it stopped until you explicitly start again. A native foreground Service, SAF directory grants and notification controls remain later milestones. The preview uses the independent `io.github.zocs.inoutnext.preview` installation ID and a retained preview certificate; it installs separately from FileInfra and the future production app. See the [Android guide](docs/distribution/android-preview.md) for build, installation and test limits.

## Build and verify

```sh
python3 scripts/sync_product.py --check
python3 -m unittest discover -s tests -v
npm --prefix apps/web run typecheck
npm --prefix apps/web run build
cargo fmt --all -- --check
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check
cargo fmt --manifest-path apps/native-slint/Cargo.toml -- --check
cargo test --workspace --locked
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
cargo test --manifest-path apps/native-slint/Cargo.toml --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo clippy --manifest-path apps/native-slint/Cargo.toml --all-targets --locked -- -D warnings
cargo build -p share-cli --locked
npm --prefix apps/web run test:e2e
python3 scripts/verify_runtime.py --mib 128
```

Browser tests use Chromium and the actual Rust service/filesystem. Native command-boundary fixtures are separate from target-device tests. Install Chromium locally or run `npx playwright install --with-deps chromium` from `apps/web`; `PLAYWRIGHT_CHROMIUM_EXECUTABLE` selects another installed binary.

The latest [CI at da7a770](https://github.com/zocs/inout-next/actions/runs/45864654594) passes Linux, Android, Windows x86/x64 and macOS x64. Both Windows targets pass the original threaded and independent-process preference-save regressions, strict Clippy, builds, package inspection and packaged Slint startup/language/theme/normal-close checks. macOS arm64 was cancelled before acquiring a hosted runner because of capacity constraints, so the run ends in failure and complete six-target acceptance remains pending. These artifacts precede the current clipboard correction. [Platform status](docs/platforms.md#ci) records each target's actual scope.

The Android extension adds real asset/index, Kotlin filesystem, private-host and foreground-lifetime checks to the existing core/browser coverage. Actual release-APK tests cover authenticated HTTP operations, browser uploads/downloads, listener closure, restart persistence and Activity lifecycle. These emulator checks are separate from physical-device and store acceptance. The earlier checksum-verified core comparison measures the 128 KiB buffer and same-handle ranges against the earlier engine and FileInfra's shipped dufs. The [size/runtime report](docs/distribution/size-and-runtime.md) records download timings, concurrent-range memory costs and localhost sampling limits; it makes no GUI or physical-LAN performance claim.

[Desktop checkpoint c004d68](https://github.com/zocs/inout-next/actions/runs/43649949553) passes all five targets serially. Its five CLI ZIPs and six native packages match the expected executable architectures and all 35 tested web resources; emitted Debian and Windows installer identity checks also pass. Package verification remains separate from installed/device/offline/signing readiness.

The [GitHub workflow](.github/workflows/ci.yml) has one repository-wide queue: Linux verification, Android arm64, Windows x86/x64, then macOS arm64/x64; every matrix has `max-parallel: 1`. Each desktop target separately tests/lints/builds the Slint trial and packages its native ZIP alongside the CLI and transitional Tauri outputs. Android continues to build the Tauri foreground composition, reusing the Linux-tested web export with locked Cargo children and manifest/resource/ABI/ELF/native ZIP alignment checks. Direct preview signing follows artifact acceptance. macOS compiles once, then permits at most three total bundle attempts for the specific `hdiutil create` resource-busy error; other errors remain fatal. Offline Windows packages remain a separate milestone.

[CI 44239092778 at d42048d](https://github.com/zocs/inout-next/actions/runs/44239092778) passes all six jobs with no overlapping intervals. Every desktop target passes 15 Tauri composition and 14 Slint tests; core counts are Linux 88, Windows 80 each and macOS 86 each. Linux also passes 56 tool and 131 browser cases, streamed 128 MiB and relocated 8 MiB checks. Android passes seven Kotlin cases and two locked arm64 compilation children. All nine downloaded artifact digests and the five Slint ZIPs, five CLI ZIPs, six transitional desktop packages and unsigned Tauri APK are verified against this run and its exact 36-file / 1,084,607-byte export. Windows probes use the exact packaged Slint executables without starting sharing. macOS GUI operation, installed/offline behavior, signing, stores and final renderer adoption retain separate gates.

The earlier local Slint integration passed 88 core/host, 15 Tauri composition, 14 Slint and 56 tool cases, strict Clippy/fmt, and 131 browser cases. Its Linux checks included relocated transfer hashes, picker selection/cancellation, address copy/selection, browser navigation and confirmed Stop/normal-close cleanup with a paused upload. That optimized executable/ZIP measured 17.88/8.79 MiB with a separate 36-file export. [Size/runtime guidance](docs/distribution/size-and-runtime.md) keeps these historical measurements separate from the embedded redesign, CI and releases.

Android preview.2 comes from [CI 43704226914 at 4dd4b51](https://github.com/zocs/inout-next/actions/runs/43704226914), with all six targets passing serially. The signed arm64 APK is 15.39MiB; its [guide](docs/distribution/android-preview.md) records exact bytes/checksums, 32 native tests per desktop target, 53 tool/91 browser/seven Kotlin cases and the separate emulator runtime evidence.

The earlier localization/live-policy checkpoint passes [CI 43744797175 at abeabd1](https://github.com/zocs/inout-next/actions/runs/43744797175) across all six serial targets, before host extraction and Slint composition. Every desktop target passes 33 Tauri native tests; Linux also passes 53 tool and 131 browser cases, and Android passes seven Kotlin cases plus locked arm64 compilation and actual APK inspection. The five server ZIPs, six desktop packages and unsigned APK match that run's 36 tested web resources. [Platform status](docs/platforms.md) records core-test counts and remaining device gates; [package measurements](docs/distribution/size-and-runtime.md) distinguish this historical checkpoint from the published previews and current trial.

```sh
cargo build -p share-cli --release --locked
python3 scripts/package_portable.py --binary target/release/share-server \
  --target linux-x64 --output .local/packages
```

The ZIP contains the stable server binary, built `web/`, product metadata, license and usage instructions; it excludes private keys and build tools. Native bundles and portable CLI packages have separate runtime requirements.

## Store distribution preparation

Google Play, F-Droid, iOS/iPadOS App Store and Mac App Store have separate dated requirements and acceptance gates in the [store research](docs/distribution/store-readiness.md). Android preparation includes the current API 36 submission floor, native 16 KB operation, SAF directory grants, correctly owned foreground services and a real F-Droid source-build recipe. The foreground APK preview does not clear these store gates. Apple preparation separates foreground iOS sharing from Mac App Store sandboxing, signing and privacy requirements. Stable app identity, independent build numbers, signing continuity, dependency licences and truthful data/transport disclosures are planned from the start. iOS and store acceptance remain pending.

## Current boundaries

There is no dufs dependency, copied server or sidecar. Next.js exports static assets; Rust implements the file operations. Configuration and compiled module interfaces provide replaceable components; a dynamic third-party plugin ABI is not implemented. Display branding comes from `product.json`, while `config/native-identity.json` preserves native package/install names and the explicit WiX upgrade UUID. Window/browser names and portable filenames can change while OS installation labels remain stable; changing installation identity requires a deliberate migration.

Uploads default to a 2 GiB limit and stream continuously; resume and crash recovery are unfinished. Directory copying is unsupported. Windows file/folder move and rename are conservatively disabled until a validated exclusive implementation exists; uploads/copies require a filesystem supporting hard links there. Linux/Android/Apple publication and moves require native exclusive rename support. Root mutation, traversal, symlink sharing and implicit overwrite are rejected.

Empty reserved `INOUTTMP` directories persist physically after transfers to avoid races between concurrent uploads. Public listings hide them, and cancellation removes unpublished files. A process or OS crash can leave internal files; recovery is a later milestone.

Public UI assets are immutable bundles bounded to 128 MiB/10,000 files and depth 128. Directory-backed exports reject root overlap, symlinks, hard links into the share and Linux bind-mount aliases. A bounded Linux/Android mount-origin check avoids scanning ordinary singly linked shares; uncertain topology or hard links retain the 100,000-entry identity scan. Valid `nsfs` namespace-handle mounts outside both subtrees do not force a scan; opaque mounts within either subtree are rejected. Layered filesystems spanning different device identities are rejected because a scan cannot prove their backing content separate. Other platforms keep the bounded scan. Embedded native bundles do not open a resource directory. Startup can reject an unprovable or oversized directory-backed share; API-only CLI operation can omit `--web-dir`. Complete ETag/If-Range validation, durable resume, blocking-storage work and explicit HTTPS reverse-proxy origins remain milestones.
