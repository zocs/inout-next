# Changelog

## 0.1.0 — Desktop preview (v0.1.0-preview.1)

- Add an independent Rust file service and CLI, static Next.js manager and Tauri desktop host.
- Separate product branding, stable installation identity, storage providers and compiled feature modules.
- Display the app as **inout**, preserving the `inout-next` repository/slug and stable installation identity; add a reusable dot-matrix wordmark and lightweight readiness-driven startup.
- Support read-only or keyed sharing, bearer/HttpOnly sessions, streamed upload/download, ranges and capability-aware file management.
- Use a measured 128 KiB default download buffer with validated 16–128 KiB host tuning, and position filesystem ranges on the original opened handle.
- Add hidden, keyboard-accessible native diagnostics: temporary buffer controls, bounded read-only loopback tests, cancellation and reset on close/Stop, with no HTTP control route or telemetry.
- Guard diagnostic replies/errors/finalizers across Stop/restart so obsolete actions cannot replace results or unlock newer work.
- Use current file-handle metadata in the Windows protected-upload alias regression so an open writer's cached directory size does not prevent the safety check.
- Separate macOS compilation from bundling and permit at most three total bundle attempts for DMG creation's transient resource-busy error, with retained logs and no recompilation; other packaging errors remain fatal.
- Derive startup removal from actual CSS transition timing with a one-second ceiling and immediate reduced-motion cleanup, preserving usable controls throughout the exit.
- Enforce root containment, portable path validation, collision-safe publication, immutable public resources and bounded shutdown.
- Add native folder selection, effective sharing settings, an isolated manager and shutdown independent of pending dialogs.
- Improve mobile layout, keyboard focus, navigation/error states, transfer progress/cancellation and accessibility.
- Provide CLI ZIPs and native Linux/Windows/macOS packages with serial target CI and resource/identity checks.
- Verify the buffer/startup/diagnostic checkpoint on all five serial desktop targets and inspect its five CLI ZIPs/six native packages against the tested 35-resource export.
- Document architecture, API contracts, design tokens, platform limits, signing reuse, early store/privacy requirements and measured core transfer/size costs.
- Define future default-address and QR requirements around explicit user choice, confirmed hotspots, recent network changes, stable recommendations and simple connection labels; these features are not implemented yet.

[The desktop prerelease](docs/distribution/preview.md) provides the verified five-target packages and checksums for testing. Android arm64 is the next build priority; iOS native adaptation is scheduled later. Mobile storage/service support, the new port-fallback and QR recommendation changes, durable resume, directory copy, Windows move/rename, older Linux compatibility, offline GUI packaging and installed/signing/store validation remain future work. No stable release is published yet.
