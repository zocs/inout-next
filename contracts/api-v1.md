# API v1 contract

Prefix: `/api/v1`. JSON strings are UTF-8. Paths are validated root-relative strings; `""` denotes root for listing only. Query encoding is decoded once. Responses never contain the absolute shared root.

## Models

`Product`: `{name, slug, description, version, application_id, repository}`.

`Entry`: `{name: string, path: string, kind: "file" | "directory", size: number | null, modified: number | null}`. Modified time is UNIX seconds. Directory size is null, not an invented byte count.

`Permissions`: `{list: boolean, download: boolean, upload: boolean, manage: boolean}`.

`Error`: `{error: {code: string, message: string}}`. Codes include `unauthorized`, `forbidden`, `invalid_path`, `not_found`, `target_exists`, `too_large`, `not_empty`, `unsupported`, `invalid_range`, `conflict`, `io_error`.

## Endpoints

| Endpoint | Request / response |
| --- | --- |
| `GET /status` | Public `{product, auth_required}` |
| `GET /context` | Authorized `{product, root_label, modules: string[], permissions, capabilities: {file_move: boolean, directory_move: boolean, directory_copy: boolean}, limits: {max_upload_bytes}}` |
| `POST /session` | JSON `{key}`; 204 and HttpOnly SameSite=Strict cookie after verification |
| `DELETE /session` | 204, revoke session and clear cookie |
| `GET /entries?path=` | `{path, entries: Entry[]}` |
| `GET/HEAD /content?path=` | Attachment response with length; GET supports one byte range |
| `PUT /content?path=` | Raw octet-stream; new file only; 201 `{entry}` |
| `POST /directories` | JSON `{path}`; 201 `{entry}` |
| `POST /operations` | JSON `{action: "rename" | "move" | "copy", source, destination}`; 200 `{entry}` |
| `DELETE /entries?path=&recursive=false` | 204; root deletion forbidden; nonempty directories need explicit recursion |

Authentication may use `Authorization: Bearer <access-key>` or the issued session. Session-authenticated mutations require a same-origin request; JSON session creation checks any supplied Origin. All operations also enforce server permissions and enabled modules. Writable configuration requires a key. Missing, unauthorized or disabled capabilities do not become active because a client crafts a request.

Storage capabilities are separate from permissions. The initial filesystem provider supplies file and directory move/rename through exclusive native rename on Linux and Apple-family targets; Windows reports both move capabilities as false. Android source support is not native-device verification. Directory copy is not yet implemented. Clients hide unsupported file/directory actions while keeping the operations supported by the actual provider available. Providers must still reject unsupported operations server-side.

Range behavior: full 200; satisfiable single ranges 206 with `Content-Range` and correct `Content-Length`; unsatisfiable ranges 416 with `bytes */size`; HEAD returns full metadata and zero body. Multiple ranges may be ignored in this first version. User files are attachment responses with `nosniff`; active HTML/SVG are not served as same-origin previews.

The filesystem provider positions a range on the original opened handle; stream-only providers use a bounded forward-read fallback. Download buffers default to 128 KiB and can be tuned by a native host for subsequent requests. There is no HTTP tuning endpoint. Concurrent single-range requests are supported, but complete ETag/If-Range validation, multi-range responses and durable transfer resume are not promised.

Uploads reject collisions, publish only after successful completion, enforce the streamed byte limit and clean unfinished staging files. The protocol does not claim resumability.

## Native desktop command boundary

This is a Tauri IPC contract for the local `main` window, separate from HTTP API v1. The external file-manager webview receives no native command capability.

`HostStatus`: `{root: string | null, running: boolean, managerUrl: string | null, addresses: string[], activeSettings: ActiveSettings | null}`. Native controls can display the locally selected absolute root; HTTP responses still do not expose it.

`ActiveSettings`: `{network: boolean, port: number, uploads: boolean, manage: boolean, authRequired: boolean}`. These are sanitized effective settings. The port is the actual listener port, including an automatically assigned port. Settings exist only while running and are cleared before stopping. Status never returns a key.

| Command | Arguments / result |
| --- | --- |
| `host_status` | No arguments; `HostStatus` |
| `choose_folder` | No JavaScript path argument; real native picker then `HostStatus` |
| `start_service` | `{options: {network, port, uploads, manage, key: string | null}}`; `HostStatus` |
| `stop_service` | No arguments; `HostStatus` after controlled shutdown |
| `open_manager` | No arguments; open/reuse the exact loopback-origin manager |
| `create_access_key` | No arguments; fresh OS-random key, never added to status |

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

The test requests a nonempty root-relative file from the active service's IPv4 loopback endpoint using normal authentication and protected-file checks. The path is bounded to 4,096 UTF-8 bytes. It sends HEAD followed by single-range GETs, partitioning at most the first 64 MiB among the selected number of nonempty ranges; files shorter than the chosen connection count are rejected. Responses must match the expected status, range and length. Content is drained without saving it, one application-wide test permit prevents overlap across service restarts, and the whole operation is bounded to 30 seconds. Cancellation and caller drop release owned network work. The report is a local timing sample rather than a checksum, version-consistency guarantee or external network measurement.

The UI keeps stopped setup drafts separate from running settings. After a reload it displays the effective status and whether a key is configured; it does not recover or persist the key through browser storage. A pending folder dialog must not block application shutdown or apply a late root selection after exit begins.
