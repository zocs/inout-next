# API v1 contract

Prefix: `/api/v1`. JSON strings are UTF-8. Paths are validated root-relative strings; `""` denotes root for listing only. Query encoding is decoded once. Responses never contain the absolute shared root.

## Models

`Product`: `{name, slug, description, version, application_id, repository}`.

`Entry`: `{name: string, path: string, kind: "file" | "directory", size: number | null, modified: number | null}`. Modified time is UNIX seconds. Directory size is null, not an invented byte count.

`Permissions`: `{list: boolean, download: boolean, upload: boolean, manage: boolean, rename: boolean, delete: boolean}`. General `manage` permits directory creation, move and copy. Rename is separately permissioned and stays within the same parent; deletion is independently permissioned and requires authenticated keyed admission. Rename/delete are additive fields; older server responses without them retain manage-based browser presentation behavior.

`Error`: `{error: {code: string, message: string}}`. Codes include `unauthorized`, `forbidden`, `invalid_path`, `not_found`, `target_exists`, `too_large`, `not_empty`, `unsupported`, `invalid_range`, `conflict`, `io_error`.

## Endpoints

| Endpoint | Request / response |
| --- | --- |
| `GET /status` | Public `{product, auth_required, policy_revision}` |
| `GET /context` | Authorized `{product, root_label, modules: string[], permissions, policy_revision, auth_required, capabilities: {file_move: boolean, directory_move: boolean, directory_copy: boolean, inline_preview: boolean}, limits: {max_upload_bytes}}` |
| `POST /session` | JSON `{key}`; 204 and HttpOnly SameSite=Strict cookie after verification |
| `DELETE /session` | 204, revoke session and clear cookie |
| `GET /entries?path=` | `{path, entries: Entry[]}` |
| `GET/HEAD /content?path=&inline=1` | Attachment by default; `inline=1` or `inline=true` selects an allow-listed preview type. GET supports one byte range |
| `PUT /content?path=` | Raw octet-stream; new file only; 201 `{entry}` |
| `POST /directories` | JSON `{path}`; 201 `{entry}` |
| `POST /operations` | JSON `{action: "rename" | "move" | "copy", source, destination}`; 200 `{entry}` |
| `DELETE /entries?path=&recursive=false` | 204; root deletion forbidden; nonempty directories need explicit recursion |

Authentication may use `Authorization: Bearer <access-key>` or the issued session. Mutations check any supplied browser Origin, including public writes; session-authenticated mutations require the same-origin boundary. JSON session creation checks supplied Origin. All operations enforce server permissions and enabled modules. Explicit upload/general-management/rename may run without a key; published-entry deletion requires a configured key and authenticated admission. Empty configured keys reject startup. Missing, unauthorized or disabled capabilities do not become active because a client crafts a request.

Storage capabilities are separate from permissions. The filesystem provider supplies file and directory move/rename through exclusive native rename on Linux, Android and Apple-family targets; Windows reports both move capabilities as false. [Android foreground preview.2](../docs/distribution/android-preview.md) release-APK operations are verified on an API36 x86_64/4 KiB emulator, separately from physical arm64 acceptance. Directory copy is not yet implemented. Clients hide unsupported file/directory actions while keeping the operations supported by the actual provider available. Providers must still reject unsupported operations server-side.

Range behavior: full 200; satisfiable single ranges 206 with `Content-Range` and correct `Content-Length`; unsatisfiable ranges 416 with `bytes */size`; HEAD returns full metadata and zero body. Multiple ranges may be ignored in this first version. User files default to octet-stream attachments with `nosniff`.

`capabilities.inline_preview` is additive; clients receiving no flag must retain downloads without issuing preview requests. With `inline=1|true`, a server-side extension allow-list serves supported raster images, audio/video and text-like files using `Content-Disposition: inline`, a specific media type (text is always `text/plain; charset=utf-8`) and `Content-Security-Policy: sandbox; default-src 'none'`. HTML/HTM/XHTML, SVG/SVGZ, PDF, extensionless and unknown files remain octet-stream attachments. The option preserves authentication, download permission, enabled modules, HEAD/range handling and transfer accounting. The browser bounds text reads to 256 KiB and requires explicit loading for images over 32 MiB; those UI limits do not alter the download API. Its optional `rev` query parameter only distinguishes refreshed content URLs and is not an ETag or a server consistency guarantee.

The filesystem provider positions a range on the original opened handle; stream-only providers use a bounded forward-read fallback. Download buffers default to 128 KiB and can be tuned by a native host for subsequent requests. There is no HTTP tuning endpoint. Concurrent single-range requests are supported, but complete ETag/If-Range validation, multi-range responses and durable transfer resume are not promised.

Uploads reject collisions, publish only after successful completion, enforce the streamed byte limit and clean unfinished staging files. The protocol does not claim resumability.

## Rust application controls

These are in-process Rust interfaces, not HTTP configuration/history endpoints or new Tauri IPC commands. `RunningServer::policy_control()` returns a `PolicyControl`; `router_with_store_and_policy` provides the same authority for an injected-provider router. Status returns `{revision, permissions, auth_required}`. `PolicyUpdate` contains `expected_revision`, complete permissions and `KeyUpdate::{Keep,Replace(String),Remove}`. Validation rejects stale revisions, blank keys, unkeyed deletion and enabling absent modules before atomically replacing the entire snapshot.

Permission-only updates retain existing sessions. Key replacement rejects old credentials for subsequent admissions. Already-admitted handlers and GET bodies retain their original immutable policy/auth snapshot and streamed work. Permission changes do not rebind a listener or stop requests; explicit service Stop still cancels/drains owned work. This backend contract does not establish uninterrupted browser upload ownership across authentication screens or implemented native live controls.

`RunningServer::transfer_control()` returns process-local `TransferControl` observations: `{active, records}`. Each record contains `{id, kind, path, policy_revision, total, bytes, state, started_ms, finished_ms}`; kind is upload/download/copy and state is active/published/served/interrupted/failed. Paths remain relative. Leases begin before pending provider work and span GET response-body lifetime; HEAD creates no transfer. At most 1,024 active and 200 terminal records are retained; capacity rejection never evicts active work. There is no background sampler.

Upload/download byte counts observe stream bodies, not network packet accounting. Published records follow successful storage publication; Served means body bytes passed downstream, not recipient saving or durable storage. Opaque copy receipts supply a final known published size when available, otherwise `total=null` and `bytes=0` mean unmeasured accounting. This history is not persisted and cannot supply durable resume/crash recovery. Native history/safe-open and live-setting integration remain pending.

## Native host command boundary

This is a Tauri IPC contract for the local desktop/Android `main` window, separate from HTTP API v1. The external file-manager webview/browser receives no native command capability.

`HostStatus`: `{root: string | null, running: boolean, managerUrl: string | null, addresses: string[], activeSettings: ActiveSettings | null, capabilities: HostCapabilities}`. Native controls can access the locally selected absolute root; HTTP responses still do not expose it. The Android UI presents app-storage copy instead of that private path.

`addresses` retains its existing string-array shape and now follows deterministic endpoint order: Wi-Fi, wired, unknown, down links, virtual, with loopback handled last in the Slint session. Duplicate addresses retain their best available link classification. Linux/Android labels use platform facts; Windows/macOS may remain unknown. Slint's `SessionSnapshot.endpoints` includes interface/link labels in process; it adds no Tauri IPC command or HTTP model. Ordering is a presentation recommendation, not proof that another device can reach an address.

`HostCapabilities`: `{storageKind: "folder" | "app-private", chooseFolder: boolean, openManager: boolean, foregroundOnly: boolean}`. Desktop values are `folder/true/true/false`; the Android foreground preview uses `app-private/false/false/true`. The field is additive; clients receiving older status without it use desktop defaults. These local host capabilities are distinct from `/context` storage capabilities.

`ActiveSettings`: `{network: boolean, port: number, uploads: boolean, manage: boolean, authRequired: boolean}`. These are sanitized effective settings. The port is the actual listener port, including an automatically assigned port. Settings exist only while running and are cleared before stopping. Status never returns a key.

| Command | Arguments / result |
| --- | --- |
| `host_status` | No arguments; `HostStatus` |
| `choose_folder` | No JavaScript path argument; real native picker then `HostStatus` |
| `start_service` | `{options: {network, port, uploads, manage, key: string | null}}`; `HostStatus` |
| `stop_service` | No arguments; `HostStatus` after controlled shutdown |
| `open_manager` | No arguments; open/reuse the exact loopback-origin manager |
| `create_access_key` | No arguments; fresh OS-random key, never added to status |

Android `choose_folder` and `open_manager` reject unsupported actions in native code. The private root is preselected and immutable to the UI. Start is rejected while suspended, during a pending background drain or after final shutdown begins; a suspension racing an awaited start stops that new listener. Home/root Back stops sharing, resume never starts it automatically, and saved files survive Activity recreation or a fresh process. The preview has no native foreground Service, SAF grant or local manager Activity.

The main native window also exposes temporary diagnostics. Five app-name activations within five seconds reveal its UI; the gesture does not authorize any command. Every command retains the main-window and active-application checks. No HTTP route or external manager capability grants these controls.

`DiagnosticReport`: `{bytes: number, durationMs: number, mibPerSecond: number, bufferKib: number, connections: number}`.

`DiagnosticStatus`: `{enabled: boolean, running: boolean, bufferKib: number, defaultBufferKib: number, probeState: "idle" | "running", result: DiagnosticReport | null}`. `running` and `probeState` describe the owned test; service state remains in `HostStatus`. These records contain no access key or absolute path.

| Diagnostic command | Arguments / result |
| --- | --- |
| `diagnostic_status` | No arguments; `DiagnosticStatus` |
| `set_diagnostic_mode` | `{enabled: boolean}`; `DiagnosticStatus` |
| `set_download_buffer` | `{kib: number}`; `DiagnosticStatus`, accepting 16–128 in steps of 16 |
| `reset_diagnostic_settings` | No arguments; `DiagnosticStatus`, restoring the default 128 KiB |
| `run_diagnostic_test` | `{path: string, connections: 1 | 2 | 4 | 8}`; `DiagnosticReport` |
| `cancel_diagnostic_test` | No arguments; `DiagnosticStatus` |

Settings require enabled diagnostics and a running service; invalid values preserve the current buffer. Changing settings during a test is rejected. Each download retains its buffer at request entry. Mode disable and service Stop cancel the probe, clear its result and restore defaults.

The native UI owns replies by service session/action. Stop/restart releases the old controls and invalidates pending initialization, settings/test replies, notices, errors and finalizers. An obsolete completion cannot restore an old result or unlock a newer action; Cancel/Close retain the current action's cancellation and focus ownership.

The test requests a nonempty root-relative file from the active service's IPv4 loopback endpoint using normal authentication and protected-file checks. The path is bounded to 4,096 UTF-8 bytes. It sends HEAD followed by single-range GETs, partitioning at most the first 64 MiB among the selected number of nonempty ranges; files shorter than the chosen connection count are rejected. Responses must match the expected status, range and length. Content is drained without saving it, one application-wide test permit prevents overlap across service restarts, and the whole operation is bounded to 30 seconds. Cancellation and caller drop release owned network work. The report is a local timing sample rather than a checksum, version-consistency guarantee or external network measurement.

The UI keeps stopped setup drafts separate from running settings. After a reload it displays the effective status and whether a key is configured; it does not recover or persist the key through browser storage. A pending folder dialog must not block application shutdown or apply a late root selection after exit begins.
