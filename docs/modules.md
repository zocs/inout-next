# Module interfaces

## Stable layers

- Domain: entry, relative path, permission, product and error models.
- Storage: asynchronous `FileStore` operations and neutral streaming readers; no HTTP/native UI dependencies.
- HTTP modules: descriptors, dependencies and route registration against shared auth/policy/storage state.
- Web modules: provider, view, toolbar, selection-associated panel and ordinary panel contributions; shared context and typed API client.
- Hosts: CLI/Tauri/native service configuration and shutdown; no duplicated file operation implementation.

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

Uploads use a module-owned sequential XHR queue with progress, cancellation, error, finalizing and saved states. Management owns its folder/operation forms and delete confirmation. Native host commands configure the enabled modules and consume the same HTTP lifecycle; they do not duplicate file operations or expose a JavaScript arbitrary-path setter.

This is composition through stable interfaces plus configurable availability. Native ABI plugin loading, hot-unloading active transfer modules and third-party executable plugins require a separate lifecycle/security design.

## Planned Store Compositions

[Store readiness](distribution/store-readiness.md) keeps mobile permissions/lifecycle, signing/build numbers, privacy disclosures and optional store integrations in native/distribution compositions. SAF and Apple security-scoped grants are storage/provider concerns; unsupported publication/move operations remain unavailable rather than bypassing containment/collision requirements. Android Service ownership and iOS finite/foreground sharing use the shared service contract with distinct platform lifetimes.

The default cross-channel composition should remain buildable as free software without proprietary Play/advertising/tracking SDKs. A future channel integration must declare its dependencies, data behavior and an independent free-software build where F-Droid is targeted. Store builds use compiled modules and packaged static assets; downloading executable feature plugins or remotely changing native features is excluded. New data flows, permissions and SDKs update the [privacy inventory](distribution/privacy-data-inventory.md) and channel gates before release.
