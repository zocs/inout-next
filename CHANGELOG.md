# Changelog

## 0.1.0 — Experimental foundation (unreleased)

- Add an independent Rust file service and CLI, static Next.js manager and Tauri desktop host.
- Separate product branding, stable installation identity, storage providers and compiled feature modules.
- Display the app as **inout**, preserving the `inout-next` repository/slug and stable installation identity; add a reusable dot-matrix wordmark and lightweight readiness-driven startup.
- Support read-only or keyed sharing, bearer/HttpOnly sessions, streamed upload/download, ranges and capability-aware file management.
- Use a measured 128 KiB default download buffer with validated 16–128 KiB host tuning, and position filesystem ranges on the original opened handle.
- Add hidden, keyboard-accessible native diagnostics: temporary buffer controls, bounded read-only loopback tests, cancellation and reset on close/Stop, with no HTTP control route or telemetry.
- Enforce root containment, portable path validation, collision-safe publication, immutable public resources and bounded shutdown.
- Add native folder selection, effective sharing settings, an isolated manager and shutdown independent of pending dialogs.
- Improve mobile layout, keyboard focus, navigation/error states, transfer progress/cancellation and accessibility.
- Provide CLI ZIPs and native Linux/Windows/macOS packages with serial target CI and resource/identity checks.
- Document architecture, API contracts, design tokens, platform limits, signing reuse and early store/privacy requirements.

Android/iOS native adaptation, durable resume, directory copy, Windows move/rename, older Linux compatibility, offline GUI packaging and installed/signing/store validation remain future work. This entry describes the experimental source preview; no release is published yet.
