# Requirements

Status: experimental0.1.0, 2026-11-15; [desktop preview.1](distribution/preview.md) and [Android foreground preview.2](distribution/android-preview.md) have separate packages and usage scopes. App display name: **inout**; repository/distribution slug: **inout-next**. Display branding is configured independently in `product.json`. Current build/runtime readiness is recorded in [platforms](platforms.md), delivery costs in [package size and runtime](distribution/size-and-runtime.md). Android physical arm64/16 KiB/network acceptance and iOS adaptation remain pending. No stable release is published yet.

## Product goal

One device starts a Rust HTTP file service for a chosen desktop directory or the Android preview's app-private storage. Other devices browse and manage its files through a modern Next.js interface without installing an application. The same core is used by a CLI, Tauri desktop shell and Android foreground adapter.

## Experience carried forward

FileInfra supplies experience in native lifecycle, authentication, offline tools, Android notifications/tiles, signing, portable packaging and honest compatibility verification. The archived inout-tauri shows why bundled executable discovery, writable-directory execution and incomplete Android storage integration need redesigned boundaries. The new project implements its own service instead of retaining dufs.

Retain a configurable dot-matrix product wordmark and a smooth, lightweight startup presentation. Performance and cross-platform behavior guide the module contracts: readiness releases controls immediately, reduced motion is respected, and branding introduces no downloaded font, per-dot animation loop or artificial startup delay. Display names can change while package, resource, upgrade and signing identities remain explicit.

Initial readiness includes usable authentication, native setup and connection-error recovery, rather than requiring sharing to be active. Escape can dismiss the waiting presentation without stopping the service. Navigation and Start/Stop must not replay it, and cleanup must not retain startup timers/listeners. Visual removal follows computed CSS transition durations/delays with a one-second ceiling, never blocking ready controls; initial/live reduced motion ends cleanup immediately at readiness. Measure real native frame times before asserting device-level smoothness.

Planned connection improvements retain automatic consecutive-port fallback and improve the default sharing address. [Network and QR requirements](network-selection.md) combine explicit user choice, confirmed hotspot state, recent physical-network changes and actual successful use, with conservative handling of VPN/virtual/unknown interfaces. Recommendations change presentation rather than the running listener, use its actual port and require separate platform and two-device validation. The current host has an address list; QR generation and recommendation are not implemented.

## Initial functional requirements

- Browse directories with relative paths, names, file sizes and modified times; do not expose the absolute host root.
- Stream file downloads through native browser links. Provide HEAD, full GET, single byte ranges, suffix/open-ended ranges, empty files and unsatisfiable ranges.
- Stream bounded uploads to staged files. Publish only on completion, clean abandoned staging files, and reject destination collisions without overwriting existing data.
- Create directories; rename/move entries where the provider supplies exclusive moves; copy files; delete files and explicitly confirmed recursive directories. The UI must follow the actual file/directory move/copy capabilities.
- Provide clear server errors for authorization, invalid paths, collisions, missing files, unsupported moves, limits and I/O failures.
- Allow read-only operation. Writable operation must be explicitly enabled and protected by an access key. Support same-origin HttpOnly sessions for native download links and bearer authorization for API clients.
- Show a responsive file manager, keyboard-accessible controls, actual permissions and an upload queue with cancel/error/finalizing states. Keep all delivered assets local.
- The HTTP manager supports English, Simplified Chinese and Traditional Chinese, with a system-language preference and evergreen, tangerine, ocean and berry themes. Validated preferences use product-independent browser storage keys and remain usable if storage is denied. Changes preserve folder navigation, search, selection, management inputs, dialogs and queued/in-flight transfers. Known API errors are localized; unknown provider details, file names, paths and custom product descriptions remain data.
- Failed folder navigation must show an explicit recoverable failure without actionable rows from another path. Selection and sharing changes need accessible state acknowledgment and preserved user-action focus. Configurable long names must fit beside mobile access controls, and form boundaries need at least 3:1 adjacent-surface contrast.
- Explicit Clear selection must restore focus to the control that selected the file or folder, with a stable current-folder fallback if that control is unavailable. Passive listing reconciliation and management-driven clearing must not move focus to an earlier selection control.
- Explicit inline/dialog cancellation must return keyboard focus to a surviving opener. Successful management operations, notice dismissal and finished-history clearing need an appropriate surviving task destination. Failed operations keep actionable forms, and an async completion must preserve unrelated focus that the user deliberately chose while waiting.
- Native reload must display actual running network/port/permission settings and whether a key is configured, without returning or persisting that key. Selection-associated actions must remain adjacent to the inspector even with long transfer history.
- A hidden native diagnostic entry uses five app-name activations within five seconds and remains keyboard accessible. It adds temporary validated download-buffer tuning and a bounded read-only local test. Hidden discovery does not grant authority: every diagnostic command remains restricted to the active application's main native window, with no HTTP tuning endpoint.
- Android's first preview shares only app-private files. Materialized raw web assets and their bounded integrity index stay separate from the shared root. Uploads from another device persist across stop/restart and Activity recreation; clearing app data/uninstalling removes them. Folder picking, local file import and a local manager are unavailable.
- Mobile host capabilities explicitly describe private storage, unsupported folder/manager actions and foreground-only lifetime; native code enforces them. Home/root Back suspends sharing, closes the listener and cancels incomplete requests. Resume never automatically restarts sharing. Configuration recreation preserves a usable stopped host; final destruction cleans up the owned service before process termination.

## Modularity and renaming

- `product.json` retains six public fields: display name, distribution slug, description, version, application ID and repository URL. A sync command updates generated frontend/desktop/distribution metadata. Stable code identifiers do not contain the brand.
- `config/native-identity.json` separately owns the associated application ID, stable native installation name and explicit WiX upgrade UUID. Tauri `productName` must use that installation value; window titles, browser/API labels and portable archive names follow display branding. A display-name change preserves Debian package/resource and NSIS install/registry identity. System installation labels remain stable until an explicit migration.
- The native identity must match the product application ID; invalid/missing/mismatched configuration must fail before changing generated consumers. Installation-name/application-ID/upgrade-code changes and signing continuity require a deliberate migration with platform verification.
- Domain models have no Axum or native-platform dependencies. Storage is accessed through an interface; the first provider uses filesystem directory capabilities.
- HTTP features register descriptors and routes. The initial modules are `files`, `uploads` and `manage`; enabling/disabling them controls actual routes and reported UI capabilities.
- The web application uses a module registry and slots for views, toolbar contributions and panels. Module code consumes shared contracts, not another module's private implementation.
- The CLI and Tauri shell consume the same service lifecycle interface. Android storage/background adapters belong behind native/provider interfaces.

## Data and transfer constraints

Reject root mutation, parent traversal, absolute/drive/UNC paths, NUL/control bytes, invalid portable names and access to internal staging names. Windows reserved COM/LPT device stems include ASCII 1–9 and superscript ¹/²/³, also with extensions, mixed case or nested components; ordinary Unicode and COM10/LPT10 remain valid. Contain symlinks with directory capabilities and reject symlink sharing in the initial provider. Source and destination are both checked. Writes do not implicitly replace existing files. Cross-volume moves return an explicit unsupported/error response; file copies use staged publication.

The filesystem provider reserves case-insensitive `INOUTTMP`, legacy `.inout-stage-*` names and conservative DOS short-name alias patterns. Empty internal containers persist so finishing one upload cannot detach another upload's acquired directory. Cancellation/error removes unpublished files; public listings hide the reserved containers. An otherwise-empty user folder remains removable nonrecursively. Process/OS crash recovery is a separate requirement.

Public web resources must be isolated from the actual provider's shared storage, including filesystem aliases, and remain immutable for the server lifetime. The initial filesystem snapshot is bounded to 128 MiB, 10,000 files and depth 128, with 100,000 shared entries scanned. Unknown providers reject web assets by default. Provider injection must never open the configuration's advisory `root` path.

Stopping cancels the whole request future, including body extraction, provider calls, copies and range positioning. Explicit stop drains tracked requests within two seconds or returns a shutdown error. Providers own their operations and release them when futures are dropped; detached writes and permanently blocking polls violate that contract.

Download buffers accept 16–128 KiB in 16 KiB increments and default to 128 KiB after a bounded buffer comparison. Capture the setting once per request; existing downloads retain their starting buffer. Filesystem ranges position on the original opened object without reopening a pathname, while asynchronous read-only providers retain a bounded forward-read fallback.

Diagnostic tests use only the active service's IPv4 loopback address and actual port, normal authorization and protected-file checks. A HEAD plus 1/2/4/8 nonempty single ranges reads at most the first 64 MiB without saving content. Permit one test at a time, validate response ranges/lengths, reject settings changes during a test and bound the whole test to 30 seconds. Cancel, disable, Stop, exit and dropping the future release owned connection/request work without holding lifecycle locks across network awaits. Close/Stop restore defaults. Diagnostic status/results contain no key or absolute path; no telemetry, background sampler or Node/Python runtime is added.

Diagnostic UI replies, notices, errors and finalizers must belong to the current service session/action. Stop/restart releases obsolete controls; a late completed reply cannot restore an old buffer/sample or unlock a newer action. Cancel/Close must preserve the current action's cancellation and focus behavior.

Default upload limit is 2 GiB, configurable. The initial release promises continuous streamed upload, not resumable sessions. Resume requires a later persisted-offset/session protocol with concurrency and crash-recovery verification. Preview, archives, WebDAV, recursive indexing and arbitrary native plugin loading are separate milestones.

## Platform targets

- Windows x86/x64: core and Tauri builds; portable delivery needs correct-architecture WebView2 and actual moved-folder/offline tests.
- Linux x64/arm64: standalone service and desktop packages. Ubuntu 18.04/glibc 2.27 compatibility is an independent headless target; Tauri GUI needs a separate WebKitGTK compatibility decision.
- macOS Intel/Apple Silicon: core and Tauri, followed by real packaging, signing and device checks.
- Android API24+ is the declared minimum, with compile/target36. The initial arm64 APK uses the Rust library, private storage and foreground-only host. API36 x86_64/4 KiB emulator execution is verified separately from physical arm64, actual16 KiB operation and API24 runtime. SAF, native foreground Service, notifications/tiles and production/store signing remain milestones.
- iOS remains a later feasibility/adapter milestone.
- Modern Next browser requirements apply. A basic page for old Chromium/WebViews is a separate compatibility milestone.

## Store distribution requirements from the start

Google Play, F-Droid, iOS/iPadOS App Store and Mac App Store are planned distribution channels with separate [dated requirements and gates S1–S7](distribution/store-readiness.md). The current 2026-11-14 official-source record is a baseline to recheck before a candidate; it is not store acceptance.

- Preserve stable application/Bundle IDs independently of display branding. Android `versionCode` and Apple build numbers must be explicit and monotonic, including same-version previews and any ABI/channel variants; they belong to native/distribution configuration, preserving the six-field public Product model.
- Keep app-signing keys, Play upload keys, aliases, certificates, API keys and provisioning profiles independent of names/slugs. Evaluate reproducible upstream-signed F-Droid APKs before choosing cross-channel signing; default F-Droid signatures do not prove same-ID upgrade compatibility. Store signing credentials and generated bundles stay out of Git.
- Android uses SAF/user-selected directories and app-private storage by default. Required permissions, safe publication and move/copy capabilities must reflect actual provider guarantees. A SAF URI is not converted into a permanent unrestricted host path.
- Android native Service owns sharing independently of Activity/WebView, with user initiation, visibility, Stop and actual foreground-service type/prerequisites/timeout handling. The current Play target floor is API36; API24 remains a separate minimum-runtime goal. Android17+/target37 local-network permission behavior needs a version-aware adapter, without requesting that new permission prematurely on target36.
- Validate every Android native library's ABI and 16 KB ELF/RELRO/package alignment plus actual 16 KB device operation; SDK/NDK version or a successful Tauri build alone is insufficient.
- iOS sharing follows allowed foreground/finite lifecycle behavior; do not promise an indefinite generic HTTP server in the background. Use security-scoped document grants, IPv6-only verification and current Apple SDK/deployment requirements.
- Mac App Store has a separate signed/sandboxed profile with selected-directory/bookmark scope, necessary network entitlements and actual `.app`/`.pkg` verification. A direct-distribution DMG does not establish sandbox/store compliance.
- Build static Next assets and Rust/native code from the same public source/locks in the F-Droid recipe. Commit required native source/generator inputs, document accepted build-tool origins, complete licence/SDK/asset obligations and run a real clean-environment fdroidserver build before proposing inclusion. Attempt APK reproducibility and retain actual byte differences; it is encouraged rather than universally required by F-Droid.
- Maintain the [privacy data inventory](distribution/privacy-data-inventory.md), publicly accessible policy/contact and in-app access, applicable store forms and accurate Apple privacy manifests/SDK signatures. Local sharing and user-initiated transfer do not automatically imply identical no-collection answers across stores. Current HTTP is unencrypted; complete appropriate transport/pairing/revocation work before claiming secure store distribution.
- Store modules use compiled composition and bundled static assets; no downloaded executable plugins/service binaries or remote scripts that change native functionality. Store-specific integrations stay outside domain/storage/HTTP layers, with a complete freedom-preserving build for F-Droid.
- Complete applicable account verification, Play closed-test requirements, TestFlight, real device/reviewer flows and store media. Record each separately from CI; keep every channel's future builds in the existing single queue.

## Acceptance and maintenance

The first runnable milestone must pass domain/storage/API tests, real streamed hash verification, actual browser CRUD/upload/download interaction, brand substitution, disabled-module behavior, and clean production builds. CI must test supported host systems and build declared artifacts. Native and compatibility status is tracked independently.

Current implementation includes the CLI, real storage/HTTP modules, static file manager, desktop controller and Android private-storage foreground host. Android checks use actual Kotlin/filesystem resources, locked target builds, package inspection and installed release-APK HTTP/browser/lifecycle tests on an API36 x86_64/4 KiB emulator. The existing desktop target CI and matched core comparison remain separately recorded; [measured results](distribution/size-and-runtime.md) include memory costs and sampling limits. Windows native directory selection, manager, supported writable operations and shutdown have bounded earlier runtime validation. Store gates, directory copy, Windows moves, SAF/foreground Service, physical Android/16 KiB/Wi-Fi, iOS, resume, offline Windows WebView2 and Ubuntu18.04 remain milestones; [platforms](platforms.md) distinguishes compilation, execution and installed/device/signing readiness.

Every major update must update affected public product documentation and retain detailed handoff/validation/review records locally under ignored paths. Public docs contain usage, architecture, current limits, roadmap and concise measured facts. All status claims require evidence; a scaffold or cross-compilation alone does not establish production readiness.
