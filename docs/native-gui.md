# Native GUI evaluation

[简体中文](native-gui_zh.md)

The application direction is a Rust-rendered GUI. Next.js continues to export the HTTP file manager used by connecting browsers. An opt-in Slint desktop trial now controls the real file service without a WebView. The existing Tauri desktop and Android releases are transitional previews; the Slint trial has not replaced those releases.

The selected evaluation order is **Slint first, then egui/eframe if Slint cannot meet the requirements**. **Iced** remains a separately assessed desktop alternative. This order does not establish final adoption or complete platform support. The renderer-free service/session boundary allows another Rust renderer to reuse the same ownership and transfer contracts.

The native GUI can use its own layout, components and motion. The Next.js manager is an aesthetic reference; it does not prescribe the application's page structure. Favor native sharing tasks and platform interaction conventions, preserving configurable pixel branding, readable local languages and transfer/confirmation contracts. The current trial's palette and two-column layout are implemented choices, and may evolve independently of the browser manager.

## Candidate fit

Released source and official documentation were checked on 2026-11-15.

| Candidate | Strength for inout | Decision gates |
| --- | --- | --- |
| egui/eframe 0.37.0 | Direct Rust UI composition, selectable Glow/wgpu rendering, MIT OR Apache-2.0 distribution and a documented Android example | Android needs GameActivity plus explicit Android AccessKit integration for the intended input/accessibility path. iOS needs a separate winit/Metal host. Chinese fonts, IME, custom-control accessibility and each target must be verified. |
| Slint 1.18.1 | Declarative layout/animation, a stable 1.x API and documented Android/iOS setup | Its Android minimum is API26, above this project's API24 goal; the released Android backend forces Skia. Android accessibility wiring differs from desktop/iOS. Choose GPL, royalty-free or commercial distribution terms explicitly, including F-Droid and Apple channel obligations. |
| Iced 0.14.0 | Rust message/update/view architecture and a selectable tiny-skia desktop renderer | The project identifies itself as experimental. Native accessibility work remains open, and the examined release has no equivalent turnkey Android/iOS application route. No Iced performance result is available here. |

All three can render without WebView. Custom drawing does not imply stock OS widgets, complete mobile integration or an automatic performance advantage. Desktop renderer results do not predict Slint's mobile Skia footprint or egui's proposed iOS Metal host.

The workspace's Rust 1.99 requirement exceeds the examined package floors: Slint 1.92, Iced 1.88 and egui/eframe 1.95. Renderer features and companion versions must remain explicit rather than silently shipping multiple backends.

## Functional Slint desktop trial

[apps/native-slint](../apps/native-slint/README.md) is an independent workspace excluded from the core's default build, with locked Slint/slint-build 1.18.1, winit, FemtoVG and desktop accessibility. Its setup includes native folder selection, an optional masked access key, public upload/management choices, LAN or loopback access and an effective bound port. Actual addresses can be selected, copied and opened in the system browser. A failed browser handoff retains localized instructions to copy the address and open it manually. Product metadata and the compiled 5×7 wordmark remain independent of installation/signing identity.

English, Simplified Chinese and Traditional Chinese are typed local catalogs. Four themes recolor the authored surfaces. Language/theme changes preserve drafts, the listener and admitted transfers. The startup exit follows controller readiness; `INOUT_NATIVE_REDUCE_MOTION=1` disables its motion. Preferences are temporary, and automatic native OS motion-preference detection is pending.

Stop and closing a running app request confirmation before service cleanup. Keep sharing preserves the listener and admitted body. Confirmed stopping releases owned work, retains bounded current-run transfer receipts and leaves the next Start explicit. The next successful Start resets those receipts. Sent download bytes do not establish saving on the receiving device. Durable history/resume, individual cancellation, safe-open, QR/network recommendations and native live-setting/diagnostic editors remain separate work.

Local optimized Linux verification covers real relocated-package upload/download integrity, public deletion denial, paused-upload preservation through language/theme changes and canceled interruption, stopped receipt retention, keyboard confirmation and About behavior. The native suite has 14 cases; the separate HTTP browser manager has 131 passing Chromium cases. These checks do not establish mobile, installed-package or assistive-device acceptance.

The locked AccessKit AT-SPI bridge lacks EditableText and reports disabled buttons as enabled/sensitive. Direct reactive control state and application guards block both physical and forced semantic background activation, but the incorrect accessibility metadata is still an adoption gate. Full screen-reader operation, Chinese composing input and platform motion policy require independent checks.

The current local Linux trial executable is 18,752,320 bytes (17.88 MiB); its ZIP with the exact 36-file / 1,084,605-byte browser export and licence notices is 9,216,822 bytes (8.79 MiB). It requires the target OS graphics, font, display and desktop services; it does not bundle those libraries or require runtime Node/Python/WebView. These functional-package sizes are separate from the throwaway comparison below. Source remains MIT; the included Slint royalty-free desktop terms and attribution are documented in [THIRD_PARTY.md](../apps/native-slint/THIRD_PARTY.md). F-Droid/Apple licensing and mobile API26/Skia are unresolved independent gates.

## Desktop target CI

[CI 43844556784 at 3d0e656](https://github.com/zocs/inout-next/actions/runs/43844556784) passes Linux x64, Android arm64 packaging, Windows x86/x64 and macOS ARM64/x64 in six nonoverlapping jobs. Every desktop target passes 14 Slint and 15 Tauri composition tests, strict Clippy and optimized builds. [Platform status](platforms.md#ci) records the conditional core counts and Linux/browser/Android checks. Five Slint trial ZIPs, five CLI ZIPs, six transitional desktop packages and an unsigned Tauri APK match that run's exact 36-file / 1,084,607-byte browser export.

The CI Linux Slint executable is 18,714,064 bytes (17.85 MiB) and its complete trial ZIP is 9,008,964 bytes (8.59 MiB), with direct GLIBC imports through 2.35. [All target package measurements](distribution/size-and-runtime.md#slint-trial-serial-ci-packages) remain separate from the local functional package above and exploratory fixtures below. Actual PE/Mach-O architecture and notices are inspected; the CI does not launch Slint GUI windows on Windows/macOS or establish the full Linux runtime floor. Android still builds Tauri and has no Slint APK. Target operation, installed/offline behavior, input/accessibility, mobile/store licensing and matched sustained presentation measurements remain adoption gates.

## Exploratory Linux measurements

Two throwaway implementations used the same real Rust HTTP/storage backend, two-worker Tokio runtime, bounded messages, 10,000 filename rows including Simplified/Traditional Chinese and the same optimized release profile. The host was Linux x86_64 with X11/XWayland and accelerated Mesa Intel graphics. Fonts used installed system fallback, rather than a controlled set of identical faces. These fixtures are not native application previews.

| Metric | egui 0.37 / Glow | Slint 1.18.1 / FemtoVG |
| --- | ---: | ---: |
| Warm launch to populated render marker, median | 125.96 ms | 181.08 ms |
| Same marker, p95, 15 launches each | 167.71 ms | 207.35 ms |
| Stripped probe executable | 12.01 MiB | 16.84 MiB |
| Single-executable ZIP | 5.01 MiB | 7.85 MiB |
| Idle process PSS, median | 46.32 MiB | 48.09 MiB |
| Idle private memory, median | 36.69 MiB | 38.01 MiB |

The populated marker requires the service address and full filename model, but it occurs **before OS presentation and GPU completion**. These are warm process starts; OS/driver/font caches were not cleared. With 15 samples, nearest-rank p95 is the largest observed sample. Layout and model representations also differ, so the table measures these implementations rather than intrinsic minimum toolkit costs.

Thirty alternating startups and six 12-second workloads passed. Every workload streamed a 64 MiB file with verified length/hash and an inactive final backend ledger. Reads were deliberately throttled; this is transfer correctness during UI work, not maximum LAN throughput. Both implementations performed actual scrolling past 70% of their scroll extent. Their rendering cadences differ, so active CPU and toolkit-local callback timings are not ranked as equivalent frame work. The reported animation memory/CPU sample interval occurred after transfer completion.

Idle memory samples come from three short runs each. No CPU-tick increase was detected in the selected idle windows at 10 ms accounting resolution; sustained energy use remains unmeasured. PSS excludes complete GPU allocations. Executable/ZIP sizes exclude static Next resources, external OS libraries/fonts, distribution notices and installed runtime costs. These results do not establish Windows, mobile, Apple, offline-package or store acceptance. Existing delivery measurements remain separate in [package size and runtime](distribution/size-and-runtime.md).

## Runtime boundaries

The native UI event loop owns widgets and rendering. `share-host` owns the renderer-free controller and `HostSession`, including sanitized state, interruption confirmation, bounded receipts and cleanup. Tauri reexports the controller contracts; Slint composes the session through a two-worker Tokio bridge with 16 queued commands. Networking and asynchronous storage stay outside the UI thread. Transfer admission/finalization notify the observer; per-block progress remains atomic-only. Active/dirty observations use a 250 ms sampling interval, while idle observation waits. One queued latest-state delivery coalesces progress. FIFO command IDs and a persistent completion watermark prevent a passive sample from releasing an accepted action's busy/focus ownership.

Provider grants, platform pickers and lifecycle adapters remain separate from the renderer. Surface recreation, language/theme changes and hiding a panel must not implicitly cancel admitted transfers. Explicit Stop/close/cancel flows require the intended user confirmation; OS termination and crash recovery need their own protocol. Android Service ownership and iOS foreground/finite background rules remain platform work whichever renderer is chosen.

Branding comes from `product.json`; native installation/signing identity remains in `config/native-identity.json`. The Slint trial implements the pixel wordmark and readiness-driven startup with explicit reduced motion. Current CSS transitions, Slint render callbacks and Tauri package checks do not establish displayed-frame smoothness or upgrade compatibility.

Switch to egui if a reproducible required-target failure remains, an essential input/accessibility/distribution requirement cannot be met, or matched sustained measurements show an unacceptable resource cost. An untested target remains a verification task, rather than evidence that Slint has failed.

## Adoption and performance gates

1. **Prove the high-risk targets.** Compile and run the selected renderer on Windows i686, including a real 32-bit OS. Validate Android minimum/current OS, arm64, 16 KiB operation, Chinese composing input, Back, safe areas and TalkBack. Validate an iOS host, UIKit scene/input/VoiceOver integration and scoped documents. Cross-compilation is recorded separately from device operation.
2. **Declare font coverage.** Test English, Simplified/Traditional Chinese UI and arbitrary user filenames, rare glyphs, wrapping, selection, clipboard and password behavior. Measure cold font discovery/first-CJK lookup and warm atlas/cache costs. Compare identical licensed font assets separately from production system fallback; offline behavior cannot depend on downloading missing fonts.
3. **Normalize UI work.** Use the same viewport/DPI, visible rows, selected fonts, scroll distance, animation duration, progress-update cadence and backend workload. Measure at least 60 seconds of scroll/resize/input and a separate sustained idle period, with multiple cold and warm launches. Record sample counts, median/p95/p99 and actual presented/late frames. A proposed 60 Hz frame budget is 16.7 ms; 120 Hz is 8.3 ms. These are target budgets, not achieved results.
4. **Trace the actual boundary.** Use Linux process/compositor profiling; Windows ETW/WPA and PresentMon where the renderer is supported; Android Perfetto/FrameTimeline, simpleperf and memory inspection; Apple Instruments/Metal traces. Rust callback spans measure CPU work, while GPU queries measure supported GPU execution. Neither alone proves a displayed frame or input-to-display latency.
5. **Separate cost categories.** Record compressed and installed package bytes, service/UI/web/font resources, external runtime requirements, process-private/PSS memory, GPU/font-atlas memory, idle repaint count, CPU and sustained energy. Measure transfer throughput/CPU/peak memory independently with the same files, integrity checks, disks and 1/4/8 connections on real LAN and slow clients.
6. **Verify ownership and distribution.** Changes in permissions, view lifetime and foreground state follow the service contract; confirmed Stop cleans up owned work. Test relocated/offline packages, channel licensing/source builds, native identity, signing and applicable store rules before publishing a replacement application.

## Official references

- [egui/eframe 0.37 release](https://github.com/emilk/egui/releases/tag/0.37.0), [native framework](https://github.com/emilk/egui/blob/0.37.0/crates/eframe/README.md), [Android example](https://github.com/emilk/egui/blob/0.37.0/examples/hello_android/README.md), [system-font provider](https://github.com/emilk/egui/blob/0.37.0/crates/egui_system_font_provider/README.md).
- [Slint 1.18.1 release](https://github.com/slint-ui/slint/releases/tag/v1.18.1), [Android guide](https://github.com/slint-ui/slint/blob/v1.18.1/docs/astro/src/content/docs/guide/platforms/mobile/android.mdx), [iOS guide](https://github.com/slint-ui/slint/blob/v1.18.1/docs/astro/src/content/docs/guide/platforms/mobile/ios.mdx), [licensing](https://github.com/slint-ui/slint/blob/v1.18.1/LICENSE.md).
- [Iced 0.14 release](https://github.com/iced-rs/iced/releases/tag/0.14.0), [accessibility work](https://github.com/iced-rs/iced/issues/552).
- [android-activity integration](https://github.com/rust-mobile/android-activity), [AccessKit platform adapters](https://github.com/AccessKit/accesskit), [store preparation](distribution/store-readiness.md).
