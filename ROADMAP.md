# Roadmap

## 1. Independent modular foundation — experimental preview

- Rust domain, capability-based storage, HTTP modules and CLI with shared lifecycle.
- Static Next.js file manager with authentication, browsing, streamed uploads/downloads and capability-aware management.
- Replaceable display branding, stable native identity and compiled feature/provider interfaces.
- Single-word **inout** display name, reusable dot-matrix wordmark and readiness-driven startup with reduced-motion support.
- Tauri desktop host with native directory selection, isolated manager and controlled start/stop.
- Bounded 128 KiB default download streaming, same-handle filesystem ranges and temporary native-only diagnostic tuning/tests.
- Real filesystem/browser tests, transfer hashes, identity/resource verification and serial Linux/Windows/macOS CI.
- Public API, architecture, design-system and distribution documentation.

These foundations are implemented. The current0.1.0 is experimental; a successful build is separate from installed/device/offline/signing readiness.

## 2. Desktop distribution and compatibility

- Actual Linux/macOS GUI, installation and device verification after current native target builds.
- Windows32bit operating-system verification, installation/uninstall/upgrade and signing.
- Separate human-visible native installer/package labels from stable resource/install identities, with emitted-package and installed-upgrade checks before changing the retained `Inout Next` labels.
- Offline Windows WebView2 distribution with explicit runtime/architecture requirements.
- Reduce Linux native runtime-packaging cost while retaining required libraries and truthful compatibility.
- Safe Windows move/rename and directory copy where storage guarantees permit them.
- LinuxARM64 runtime and an explicit Ubuntu18.04/glibc2.27/WebKitGTK compatibility strategy.

## 3. Android integration

- Rust library/JNI and native Service ownership independent of Activity/WebView.
- User-started, visible and stoppable sharing with correct foreground-service prerequisites/types/timeouts.
- SAF/app-private storage and truthful provider publication/move capabilities.
- API24 minimum-runtime goal, current Play submission target and version-aware local-network permissions.
- Actual arm64APK/AAB,16KB native-library/package alignment and physical-device operation.
- Repeatable clean-source web/Rust/Gradle builds, explicit versionCode, permission revocation and signing-channel continuity.

## 4. Transfer reliability and optional modules

- Extend matched core buffer/range measurements to physical LAN, slow clients, real disks and target devices; keep runtime tuning bounded and optional.
- Persistent resumable uploads, cancellation/retry/crash recovery and multi-GiB/slow-client/concurrency measurements.
- Archives, previews, editor and search through independently registered modules.
- Lightweight old-browser fallback and additional storage providers.
- iOS foreground-sharing feasibility, security-scoped storage, IPv6 and interruption/restart behavior.

## 5. Store candidates

- Follow the dated [store gatesS1–S7](docs/distribution/store-readiness.md) and [privacy inventory](docs/distribution/privacy-data-inventory.md); recheck current policies before submission.
- Complete dependency/SDK/asset licensing and a real F-Droid source build; attempt reproducible upstream-signed APK verification early.
- Verify channel identities, signing versus upload-key roles, monotonic build numbers and real installed upgrades.
- Publish accurate privacy/support materials, required store disclosures, native manifests and transport/pairing/revocation behavior.
- Complete applicable Play testing, TestFlight, target-device/reviewer flows and store media.
- Build a separate Mac App Store sandboxed/signed app/pkg profile; direct DMGs remain a distinct channel.

## Release criteria

Relevant tests, independent review, actual CI/artifact verification, truthful platform limits and current product documentation precede a release. Store candidates additionally require their channel-specific evidence. Current CLI ZIPs and native target builds do not establish Android, old-Linux, offline Windows or store readiness.
