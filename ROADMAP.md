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

These foundations are implemented. [Desktop checkpoint c004d68](https://github.com/zocs/inout-next/actions/runs/43649949553) passes all five serial targets and inspection of five CLI ZIPs/six native packages against its tested export, alongside the recorded core comparison and streamed/relocated checks. The Android extension adds private-host, Kotlin/index, real package and emulator HTTP/browser/lifecycle coverage described below. Current0.1.0 is experimental; target/device/offline/store readiness remains separate.

The earlier unreleased localization/live-policy source passes [six-target serial CI at abeabd1](https://github.com/zocs/inout-next/actions/runs/43744797175). Its five server ZIPs, six transitional desktop packages and unsigned arm64 APK are inspected against the exact 36-resource export. That run precedes host extraction and the Slint trial; selected-renderer/device and release milestones below remain open.

## Native renderer migration

Next.js remains the HTTP/browser manager. The final app uses Rust native rendering; current Tauri packages remain transitional previews.

- Evaluate **Slint first, egui as the fallback** through the [native feasibility gates](docs/native-gui.md). Switch on an unresolved required-target, accessibility/distribution or matched-performance failure; an untested platform alone does not establish failure. Iced remains documented for comparison. No toolkit is adopted yet.
- The renderer-free `share-host` controller, diagnostics and session are extracted. The opt-in Slint desktop trial has real setup/addresses, current-run receipts, three languages/four themes, native branding and confirmed Stop/close. Its two-worker Tokio bridge bounds commands, coalesces progress and preserves command-completion identity.
- Complete selected-renderer target/device and distribution checks, platform picker/grant adapters, persistent settings/OS motion preferences, live-setting/diagnostic controls and matched sustained presentation/resource measurements. Keep storage/providers and native lifecycle independent of widgets.
- Prove selected-renderer Windows i686/real 32-bit OS, Android GameActivity or Slint API26 decision, Chinese input/TalkBack/16 KiB and a native iOS host/VoiceOver path.
- Preserve pixel branding, native localized themes and confirmed Stop/close/cancel, with actual presentation/input/idle/energy and relocated-package measurements.

## 2. Desktop distribution and compatibility

- Extend the actual Linux Slint trial checks to target-built packages, macOS/Windows native operation, installation and device validation.
- Windows32bit operating-system verification, installation/uninstall/upgrade and signing.
- Separate human-visible native installer/package labels from stable resource/install identities, with emitted-package and installed-upgrade checks before changing the retained `Inout Next` labels.
- Validate offline native Windows graphics/runtime packaging and actual 32-bit operation. Transitional Tauri WebView2 delivery has its own runtime/architecture boundary.
- Reduce Linux native runtime-packaging cost while retaining required libraries and truthful compatibility.
- Safe Windows move/rename and directory copy where storage guarantees permit them.
- LinuxARM64 runtime and an explicit Ubuntu18.04/glibc2.27 native graphics/font/runtime strategy, with transitional WebKitGTK limits recorded separately.

## Connection experience

- Preserve bounded consecutive-port fallback, actual-address display and the preferred port for the next start, with controlled app-owned shutdown.
- Add a modular [sharing-address recommendation and QR flow](docs/network-selection.md): explicit user choice, confirmed hotspots, recent network changes and successful remote transfers, with stable fallback and simple connection labels.
- Verify interface classification, network events and permission degradation independently on each platform, followed by real two-device scanning/transfers; new VPN/virtual interfaces must not displace ordinary LAN connections automatically.

## 3. Android integration

The first Android composition is [v0.1.0-preview.2](https://github.com/zocs/inout-next/releases/tag/v0.1.0-preview.2), with a [foreground-sharing guide](docs/distribution/android-preview.md), separate from the completed [desktop prerelease](docs/distribution/preview.md). It shares app-private files with another device's browser, retains the existing Rust authorization/storage modules and exposes capability-aware native controls. Home/root Back stops sharing; resume requires explicit Start. Raw packaged web resources are integrity-checked outside the shared root. Optimized arm64 package inspection and API36 x86_64/4 KiB emulator execution are separate from physical-device acceptance.

The signed arm64 delivery is 15.39MiB and comes from [six-target serial CI at 4dd4b51](https://github.com/zocs/inout-next/actions/runs/43704226914), with its own guide and download checksums. The release tag adds documentation without changing the tested application files.

Remaining Android milestones:

- Native Service ownership independent of Activity/WebView, extending the implemented Rust library/JNI host.
- User-started, visible and stoppable sharing with correct foreground-service prerequisites/types/timeouts.
- SAF directory selection and local import/export/manager UX, retaining the implemented app-private storage and truthful provider publication/move capabilities.
- API24 minimum-runtime goal, current Play submission target and version-aware local-network permissions.
- Physical arm64 and real16 KiB operation, two-device Wi-Fi, minimum/current OS and modern System WebView checks; AAB/store packaging beyond the inspected direct APK.
- Repeatable clean-source web/Rust/Gradle builds, explicit versionCode, permission revocation and signing-channel continuity.

### iOS/iPadOS preparation

- Adapt security-scoped documents and explicit foreground/allowed finite sharing, interruption, cancellation and manual restart.
- Prepare current Apple SDK/deployment, local-network/IPv6 and privacy-manifest requirements with stable identity and independent build numbers.
- Build/sign an archive and perform simulator, TestFlight and physical-device checks when an Apple build environment/device is available. No iOS artifact is currently released.

## 4. Transfer reliability and optional modules

Current source supplies atomic Rust permission/key updates, independent rename/delete authorization and bounded process-local transfer observations. Already-admitted bodies/provider work retain their policy. The HTTP manager and Slint trial have English/Simplified/Traditional catalogs and four themes; language/theme changes preserve task state. Slint supplies current-run receipts and confirmed Stop/close with cancellation that preserves sharing. Existing release packages precede these source updates.

- Integrate native live settings and browser ownership across permission/authentication changes without cancelling admitted transfers; retain the mounted-module availability boundary.
- Extend confirmation to individual transfer cancellation and browser close/authentication flows. Add durable native history/safe-open, single-file/common-directory sharing and configurable browser previews. Current-run Slint receipts do not establish persistent history; durable resume and crash recovery require persisted protocols beyond the process-local ledger.

- Extend matched core buffer/range measurements to physical LAN, slow clients, real disks and target devices; keep runtime tuning bounded and optional.
- Persistent resumable uploads, cancellation/retry/crash recovery and multi-GiB/slow-client/concurrency measurements.
- Archives, previews, editor and search through independently registered modules.
- Lightweight old-browser fallback and additional storage providers.

## 5. Store candidates

- Follow the dated [store gatesS1–S7](docs/distribution/store-readiness.md) and [privacy inventory](docs/distribution/privacy-data-inventory.md); recheck current policies before submission.
- Complete dependency/SDK/asset licensing and a real F-Droid source build; attempt reproducible upstream-signed APK verification early.
- Verify channel identities, signing versus upload-key roles, monotonic build numbers and real installed upgrades.
- Publish accurate privacy/support materials, required store disclosures, native manifests and transport/pairing/revocation behavior.
- Complete applicable Play testing, TestFlight, target-device/reviewer flows and store media.
- Build a separate Mac App Store sandboxed/signed app/pkg profile; direct DMGs remain a distinct channel.

## Release criteria

Relevant tests, independent review, actual CI/artifact verification, truthful platform limits and current product documentation precede a release. Store candidates additionally require their channel-specific evidence. Target builds and emulator operation do not establish physical arm64/16 KiB/Wi-Fi, old-Linux, offline Windows or store readiness.
