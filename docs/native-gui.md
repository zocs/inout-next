# Native GUI evaluation

The planned application uses a Rust-rendered GUI. Next.js continues to export the HTTP file manager used by connecting browsers. The existing Tauri desktop and Android releases are transitional previews; no Rust-rendered application preview is released yet.

The first feasibility candidate is **egui/eframe**, with **Slint** retained as the alternative for declarative UI and its documented mobile integration. **Iced** remains a desktop fallback. This is an evaluation order, not final adoption or complete platform support.

## Candidate fit

Released source and official documentation were checked on 2026-11-15.

| Candidate | Strength for inout | Decision gates |
| --- | --- | --- |
| egui/eframe 0.37.0 | Direct Rust UI composition, selectable Glow/wgpu rendering, MIT OR Apache-2.0 distribution and a documented Android example | Android needs GameActivity plus explicit Android AccessKit integration for the intended input/accessibility path. iOS needs a separate winit/Metal host. Chinese fonts, IME, custom-control accessibility and each target must be verified. |
| Slint 1.18.1 | Declarative layout/animation, a stable 1.x API and documented Android/iOS setup | Its Android minimum is API26, above this project's API24 goal; the released Android backend forces Skia. Android accessibility wiring differs from desktop/iOS. Choose GPL, royalty-free or commercial distribution terms explicitly, including F-Droid and Apple channel obligations. |
| Iced 0.14.0 | Rust message/update/view architecture and a selectable tiny-skia desktop renderer | The project identifies itself as experimental. Native accessibility work remains open, and the examined release has no equivalent turnkey Android/iOS application route. No Iced performance result is available here. |

All three can render without WebView. Custom drawing does not imply stock OS widgets, complete mobile integration or an automatic performance advantage. Desktop renderer results do not predict Slint's mobile Skia footprint or egui's proposed iOS Metal host.

The workspace's Rust 1.99 requirement exceeds the examined package floors: Slint 1.92, Iced 1.88 and egui/eframe 1.95. Renderer features and companion versions must remain explicit rather than silently shipping multiple backends.

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

The native UI event loop owns widgets and rendering. An application-owned Rust controller owns service state, permissions, transfer observations and explicit shutdown. Tokio runs networking and asynchronous storage work; UI callbacks send bounded commands and receive snapshots without blocking the UI thread. Progress updates should be coalesced independently of transferred chunks, with a bounded refresh rate and no idle animation loop.

Provider grants, platform pickers and lifecycle adapters remain separate from the renderer. Surface recreation, language/theme changes and hiding a panel must not implicitly cancel admitted transfers. Explicit Stop/close/cancel flows require the intended user confirmation; OS termination and crash recovery need their own protocol. Android Service ownership and iOS foreground/finite background rules remain platform work whichever renderer is chosen.

Branding comes from `product.json`; native installation/signing identity remains in `config/native-identity.json`. The pixel wordmark and readiness-driven, reduced-motion startup are product requirements for the new renderer. Current CSS transitions and Tauri package checks do not establish native animation smoothness or upgrade compatibility.

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
