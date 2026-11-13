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
| `GET /context` | Authorized `{product, root_label, modules: string[], permissions, limits: {max_upload_bytes}}` |
| `POST /session` | JSON `{key}`; 204 and HttpOnly SameSite=Strict cookie after verification |
| `DELETE /session` | 204, revoke session and clear cookie |
| `GET /entries?path=` | `{path, entries: Entry[]}` |
| `GET/HEAD /content?path=` | Attachment response with length; GET supports one byte range |
| `PUT /content?path=` | Raw octet-stream; new file only; 201 `{entry}` |
| `POST /directories` | JSON `{path}`; 201 `{entry}` |
| `POST /operations` | JSON `{action: "rename" | "move" | "copy", source, destination}`; 200 `{entry}` |
| `DELETE /entries?path=&recursive=false` | 204; root deletion forbidden; nonempty directories need explicit recursion |

Authentication may use `Authorization: Bearer <access-key>` or the issued session. Session-authenticated mutations require a same-origin request; JSON session creation checks any supplied Origin. All operations also enforce server permissions and enabled modules. Writable configuration requires a key. Missing, unauthorized or disabled capabilities do not become active because a client crafts a request.

Range behavior: full 200; satisfiable single ranges 206 with `Content-Range` and correct `Content-Length`; unsatisfiable ranges 416 with `bytes */size`; HEAD returns full metadata and zero body. Multiple ranges may be ignored in this first version. User files are attachment responses with `nosniff`; active HTML/SVG are not served as same-origin previews.

Uploads reject collisions, publish only after successful completion, enforce the streamed byte limit and clean unfinished staging files. The protocol does not claim resumability.
