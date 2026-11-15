# Module interfaces

## Stable layers

- Domain: entry, relative path, permission, product and error models.
- Storage: asynchronous `FileStore` operations and neutral streaming readers; no HTTP/native UI dependencies.
- HTTP modules: descriptors, dependencies and route registration against shared auth/policy/storage state.
- Web modules: provider, view, toolbar, selection-associated panel and ordinary panel contributions; shared context and typed API client.
- Hosts: CLI/Tauri/native service configuration and shutdown; no duplicated file operation implementation.

Desktop and Android host composition share `HostController`. The desktop composition selects a folder; `new_app_private` validates and preselects an owned mobile root and refuses root replacement. Native status exposes `storageKind`, `chooseFolder`, `openManager` and `foregroundOnly`. These host capabilities control local setup and do not replace HTTP module manifests, permissions or storage move/copy capabilities. Older native clients/fixtures without the additive field retain desktop defaults.

The [Android preview.2 adapter](distribution/android-preview.md) materializes raw packaged web assets before Rust initialization, keeping them beside the private share rather than inside it. Foreground gating closes before asynchronous suspension cleanup; starts are checked around the awaited service start, pending stops drain before a restart, and resume only restores eligibility. Home/root Back retains a stopped host, while final destruction performs owned service cleanup. SAF, native foreground Service and local-manager/import UI remain separate host/provider work.

The six-field public `Product` remains a domain/API model. Stable native package/install values live in separate build configuration and do not enter feature state or HTTP models. Portable component validation also belongs to the domain boundary, including ASCII and superscript COM/LPT device names; modules continue to use the shared parser and typed errors.

## Initial features

| ID | Capability | Dependencies |
| --- | --- | --- |
| `files` | Directory browsing and content downloads | storage |
| `uploads` | Continuous streamed upload | files |
| `manage` | Directory creation, rename/move/file-copy/delete | files |

Disable uploads/manage by selecting only `files`. The API returns the available module IDs and effective permissions. The browser uses that manifest to render registered contributions. Server routes for disabled modules are not mounted, and permissions are still checked for mounted routes.

A new module supplies its manifest, routes and optional web contribution, then registers with the host. A replacement storage provider implements the storage trait. Neither requires modifying a file browser's internal state or introducing native paths into HTTP handlers.

Registration is compiled composition: add the descriptor/dependencies to the domain registry, route contribution to HTTP composition and optional entry to the web `featureRegistry`. Providers and contributions consume `ModuleRuntime` rather than another feature's private state. This is not runtime discovery of arbitrary binaries.

| Web contribution | Placement / ownership |
| --- | --- |
| `Provider` | Wraps descendants with module-owned state |
| `View` | Folder content area |
| `Toolbar` | Folder commands in registry order |
| `SelectionPanel` | Immediately after the selected-entry inspector; supports management forms even without a selection |
| `Panel` | General folder panels, including transfer history |

`SelectionPanel` is an additive optional slot. The shell chooses contribution positions, not module IDs; a long transfer history cannot separate management actions from their selected entry. Navigation failure must not retain actionable rows belonging to a different folder.

Selection focus belongs to the shared shell lifecycle. File selection and folder Details controls identify the initiating control; explicit inspector clearing returns focus there or to a stable current-folder control if it is unavailable. Passive refresh/reconciliation and management clearing do not restore selection focus. Modules use the shared selection contract rather than looking up another feature's DOM or private state.

Explicit management tasks use operation-owned focus independently of selection. Cancellation returns a connected feature-owned opener; successful operations use `ModuleRuntime.focusCurrentFolder()`, while failure keeps the relevant input/Cancel actionable. The small shared task-focus helper captures ownership before disabling/removal and abandons restoration if the user selects an unrelated focus destination while waiting. Upload history owns its Transfers heading destination; notice dismissal uses the shell callback. No passive update effect or private cross-module DOM query acquires focus.

The actual context includes permissions and `capabilities.file_move`, `directory_move` and `directory_copy`. Management renders rename/move/copy only when both policy and provider capability allow them; unsupported operations still fail on the server. Windows FsStore advertises no file/directory moves. Directory copy is currently unsupported on every provider.

`FileStore` uses neutral readers and typed metadata/errors for list/open/upload/mkdir/move/copy/remove. It reports move support and can return proven-isolated `WebAsset` snapshots; that asset method defaults to unsupported. Injection does not open `ServerConfig.root`. Providers must keep work owned by their futures so cancellation releases readers, unpublished stages and resources. FsStore closes a child inspection capability before parent-relative nonrecursive deletion, preserving Windows directory-handle semantics. Platform/provider limits are in [platforms](platforms.md).

An opened download exposes `FileReader`, which preserves optional seeking on its original handle. HTTP ranges call its positioning operation; an asynchronous stream-only provider can retain forward reading without implementing a filesystem path or reopening one. Upload readers keep their existing neutral streaming interface.

The shared HTTP `DownloadControl` validates 16–128 KiB buffers in 16 KiB steps, defaulting to 128 KiB. Hosts receive a clone from the running service. Each GET captures the setting once, so active transfers retain their starting buffer. This application-side control does not add a public HTTP tuning route or per-byte monitoring.

Uploads use a module-owned sequential XHR queue with progress, cancellation, error, finalizing and saved states. Management owns its folder/operation forms and delete confirmation. Native host commands configure the enabled modules and consume the same HTTP lifecycle; they do not duplicate file operations or expose a JavaScript arbitrary-path setter.

This is composition through stable interfaces plus configurable availability. Native ABI plugin loading, hot-unloading active transfer modules and third-party executable plugins require a separate lifecycle/security design.

The planned [network recommendation module](network-selection.md) separates platform snapshots/events, a deterministic address selector and QR rendering. It consumes the actual listener port and explicit user choice; it does not alter HTTP/file modules or running interface bindings. Missing platform metadata must produce conservative labels and recommendations. The current desktop composition only lists IPv4 LAN addresses; this selector and QR flow are not implemented.

## Native diagnostic extension

Diagnostics belong to the native host, outside the public `files`/`uploads`/`manage` registry. The optional header activation callback exists only in the native host; the HTTP manager has a plain brand. Its panel is lazy-loaded and consumes a typed IPC contract rather than shared host paths or another module's private state.

The Rust module owns temporary buffer validation, a private authentication handle and one bounded read-only loopback probe. It reuses the shared file API and the running service's `DownloadControl`. Cancellation and application-wide concurrency remain native lifecycle concerns; there is no public diagnostic HTTP route, saved test file or sampling when disabled. The panel owns each service session/action revision: old settings/probe replies, notices, errors and finalizers cannot alter a newer action. Stop/restart releases obsolete flags, while Cancel/Close retain their current ownership and focus behavior. Exact limits and data records are in [API v1](../contracts/api-v1.md).

## Brand and startup presentation

`Wordmark({name})` is a memoized presentation component used by the header, footer and startup. Its local 5×7 ASCII letter/digit glyphs support up to eight characters; unsupported or longer names fall back to wrapped text. It owns neither installation identity nor a diagnostic command. `Header` exposes an optional native-only activation callback with the initiating button, preserving keyboard and focus ownership independently of the rendered brand.

The page owns first readiness. `FileManager.onReady(product)` reports the first usable context, authentication or recovery screen; `NativeHost.onReady()` reports initial native status or error, including setup before a service runs. These optional callbacks are independent of file-feature state and `HostStatus.running`. `Startup({ready,name})` owns visual dismissal and computed CSS-duration/delay cleanup with a one-second ceiling; initial/live reduced motion completes it immediately at readiness. Later navigation and service changes do not reopen it, and controls do not depend on animation completion. Product substitution needs no change to storage, feature registries or lifecycle algorithms.

## Planned Store Compositions

[Store readiness](distribution/store-readiness.md) keeps mobile permissions/lifecycle, signing/build numbers, privacy disclosures and optional store integrations in native/distribution compositions. SAF and Apple security-scoped grants are storage/provider concerns; unsupported publication/move operations remain unavailable rather than bypassing containment/collision requirements. Android Service ownership and iOS finite/foreground sharing use the shared service contract with distinct platform lifetimes.

The default cross-channel composition should remain buildable as free software without proprietary Play/advertising/tracking SDKs. A future channel integration must declare its dependencies, data behavior and an independent free-software build where F-Droid is targeted. Store builds use compiled modules and packaged static assets; downloading executable feature plugins or remotely changing native features is excluded. New data flows, permissions and SDKs update the [privacy inventory](distribution/privacy-data-inventory.md) and channel gates before release.
