# Package size and runtime

Measurements record experimental checkpoints from 2026-11-15–16, with desktop-package comparisons against [FileInfra 0.5.13](https://github.com/zocs/FileInfra/releases/tag/v0.5.13). Sizes use MiB (1,048,576 bytes). Archive size, uncompressed executable size, system dependencies and process memory are different quantities. Local builds and verified CI packages are identified separately below. [Desktop preview.1](preview.md) and [Android preview.2](android-preview.md) distribute separate verified CI packages; no stable release is published yet.

## Local desktop checkpoint

The Linux x64 desktop checkpoint uses Rust 1.99.1 and Node 26.13.1. Its export and executables were verified together, including a relocated CLI ZIP and a streamed 128 MiB runtime check. These measurements precede the Android adaptation; the Android APK and its own export are measured separately below.

| Item | Bytes | MiB |
| --- | ---: | ---: |
| Static Next export, 35 files | 1,050,698 | 1.00 |
| Uncompressed Rust CLI | 3,977,248 | 3.79 |
| Uncompressed Tauri native executable | 15,213,808 | 14.51 |
| CLI + static web ZIP | 1,721,866 | 1.64 |

The export grew by 22,310 bytes compared with the earlier 32-file export. The CLI and pages deliver no Python or Node runtime. The native executable excludes external webview libraries and is not an AppImage-size measurement. An older local native executable has insufficient build provenance for attributing growth specifically to diagnostics or branding; total native diagnostics memory and startup frame times remain unmeasured.

## Verified CLI delivery

[Desktop CI at c004d68](https://github.com/zocs/inout-next/actions/runs/43649949553) passed all five serial targets. These downloaded packages were checked against that run's own 35-file, 1,050,660-byte web export, including executable architecture, archive contents and hashes.

| Target | CLI + static web ZIP | Uncompressed server executable |
| --- | ---: | ---: |
| Windows x86 | 1.36MiB | 2.23MiB |
| Windows x64 | 1.45MiB | 2.76MiB |
| Linux x64 | 1.61MiB | 3.79MiB |
| macOS ARM64 | 1.50MiB | 3.27MiB |
| macOS Intel | 1.53MiB | 3.24MiB |

Each archive includes35exported web resources totalling1.00MiB before compression, product metadata, licence and instructions. It contains no Python scripts, Node executable or node_modules. The Rust service and static pages need neither Python nor Node at runtime.

For context, [dufs0.47.0](https://github.com/sigoden/dufs/releases/tag/v0.47.0) Windowsx64 ZIP is1.68MiB and Linuxx64-musl tar.gz is1.96MiB. Different compression formats, libc targets and feature sets prevent treating these sizes as an exact feature-parity comparison. A modern static Next page does not require delivering a Next server.

## Verified native packages

The six desktop packages come from the same c004d68 CI checkpoint. Extraction verifies the native CPU/subsystem and every packaged web resource; it is separate from installing or running the GUI on a device. Both macOS bundles succeeded on their first packaging attempt, so that run does not demonstrate an actual resource-busy recovery. The Android row comes from preview.2's separately accepted [CI at 4dd4b51](https://github.com/zocs/inout-next/actions/runs/43704226914), followed by direct preview signing.

| Platform / package | inout-next 0.1.0 checkpoint | FileInfra0.5.13 | Runtime distinction |
| --- | ---: | ---: | --- |
| Windows x64 installer | 2.90MiB | 11.39MiB | Tauri uses system WebView2; current NSIS downloads its bootstrapper. FileInfra includes Flutter runtime |
| Windows x86 installer | 2.72MiB | No corresponding asset | Earlier 32bit target operation on Windowsx64 is verified;32bit OS/installation/offline remain separate |
| Linux x64 deb | 4.42MiB | 9.63MiB | Tauri declares system WebKitGTK4.1/GTK3 dependencies; package size excludes their installation cost |
| Linux x64 AppImage | 79.06MiB | 22.41MiB | Current Tauri bundle includes a large WebKitGTK/JavaScriptCore/GTK/ICU library closure |
| macOS Intel native archive | 4.25MiB DMG | 20.03MiB ZIP | Different archive formats; Tauri uses Apple's system webview |
| macOS ARM64 DMG | 4.16MiB | 19.93MiB ZIP | Different archive formats and native runtimes |
| Android arm64 APK | 15.39MiB signed CI foreground preview | 11.14MiB | Different feature sets; Tauri uses Android System WebView, and physical arm64 is unverified |

FileInfra's Windowsx64 portable **GUI** ZIP is15.42MiB; it is not equivalent to the1.45MiB inout-next **CLI** ZIP. The former contains a20.29MiB uncompressed Flutter engine,6.72MiB Dart AOT application and5.23MiB dufs library, plus plugins/assets/runtime files. The inout-next uncompressed Windows native executable is11.44MiB forx64 and9.07MiB forx86, plus static resources and external WebView2 requirements. Extraction does not establish offline GUI portability.

That Linux AppImage's logical regular-file inventory totals approximately258.03MiB before SquashFS compression. Its largest libraries are WebKitGTK85.84MiB, JavaScriptCore32.20MiB and ICU data28.12MiB; the native executable is14.81MiB. The static Next export is1.00MiB. Library packaging, compression/deduplication and compatibility strategy need independent work; shrinking a package by omitting a required runtime does not establish a working portable artifact.

## Android foreground preview

The distributed optimized arm64 APK includes the private-storage, resource, ProGuard, inset and lifecycle corrections. [CI 43704226914](https://github.com/zocs/inout-next/actions/runs/43704226914) builds it from `4dd4b51` using that run's tested web export. Its measured signed size is **16,135,122 bytes (15.39 MiB)**, with one **13,216,272-byte Rust library** and **35 raw web resources totalling 1,052,299 bytes**. The APK uses Android System WebView and contains neither a Node nor Python runtime. Signing preserves the original archive payloads; exact package and certificate checksums are in the [Android guide](android-preview.md).

Separate local validation builds measure 16,151,507 bytes (15.40 MiB) for signed arm64 and 15,553,488 bytes (14.83 MiB) for signed x86_64, with their own 35-file / 1,052,337-byte export. The x86_64 APK is used for API36/4 KiB emulator execution with the same application code. ABI/compiler/package differences prevent treating it as the delivered arm64 APK or a physical-device memory/throughput measurement. All three APKs pass exact resource, single-ABI, manifest, retained-preview-signature and 16 KiB ELF/native ZIP alignment checks; these checks do not establish 16 KiB runtime operation. FileInfra0.5.13's arm64 APK is 11.14 MiB, but native/background/storage features differ and no matched mobile performance comparison has been performed.

## Current core measurements

Three rotating rounds used a fresh process for each of four engines on one Linux x64 host: 12 processes in total. Each used authenticated localhost HTTP, a 1 MiB warmup, 128 MiB upload/full download, a 37-byte suffix, and reassembled downloads over 1/4/8 single-range connections. Deterministic indexed SHAKE-256 blocks avoided repeated aligned contents; every uploaded file, full download, suffix and reassembled range body passed integrity checks. All test processes, listeners and temporary fixtures were removed. FileInfra's released `libdufs.so` ran in a minimal C FFI host; neither Flutter nor Tauri GUI was running.

| Median of 3 samples | Earlier 4 KiB engine | Buffer-only 128 KiB experiment | Current 128 KiB + same-handle seek | FileInfra bundled dufs |
| --- | ---: | ---: | ---: | ---: |
| 128 MiB upload, seconds | 0.098348 | 0.107092 | 0.107404 | 0.452953 |
| 128 MiB full GET, seconds | 0.411646 | 0.086644 | 0.063696 | 0.094227 |
| Full GET server CPU, seconds | 0.50 | 0.08 | 0.07 | 0.10 |
| 37-byte suffix, seconds | 0.271843 | 0.247098 | 0.012311 | 0.012731 |
| Warmed server RSS, MiB | 7.05 | 7.38 | 7.38 | 7.13 |
| Sampled peak RSS, MiB | 8.83 | 12.57 | 14.88 | 10.06 |

| Reassembled 128 MiB range download, seconds | Earlier engine | Buffer-only experiment | Current engine | FileInfra bundled dufs |
| --- | ---: | ---: | ---: | ---: |
| 1 connection | 0.526513 | 0.073184 | 0.068066 | 0.463366 |
| 4 connections | 0.335631 | 0.259625 | 0.048639 | 0.169106 |
| 8 connections | 0.411770 | 0.278825 | 0.036337 | 0.172940 |

The smaller earlier read batches increased download work. Raising capacity improved full GET, but a buffer-only change still read and discarded high-offset prefixes. Positioning the original filesystem handle removes that prefix work and improves suffix/parallel ranges without reopening a pathname. Stream-only providers retain bounded forward reading. A separate 45-process sweep evaluated 16–128 KiB in 16 KiB steps plus the 4 KiB baseline before selecting the default.

These are cache-warm localhost samples, not universal speed guarantees. The earlier engine is an Ubuntu 22.04 CI build; the experiment and current engine use the same local compiler/profile. Authentication and filesystem/publication semantics differ between projects. Sampled peak RSS covers the workload including eight concurrent ranges and is higher in the current engine: larger per-request buffers have a real memory cost. Matching warmed RSS for the experiment and current CLI assesses the core control path, not the native diagnostics panel's total overhead.

The built-in native test separately caps reads at 64 MiB and provides a local timing sample, not this checksum-verified comparison. Tauri GUI startup, total application/child-webview memory, native IPC, physical LAN, cold disk, slow clients and mobile performance still need matched target measurements. FileInfra's server already uses Rust; replacing its Flutter shell does not itself accelerate file transfers. ETag/If-Range validation, durable resume and feature/platform parity remain incomplete.

## Current source web export

The locally verified localization/permission integration exports 36 files totalling 1,084,605 bytes. JavaScript totals 1,011,194 raw bytes and 312,634 gzip bytes, measured separately per file. The same export passes 131 Chromium cases, including locale/theme changes during a real upload and public-write/action-visibility checks. These figures describe the current local source export; the earlier release-package measurements above retain their own exports and provenance.

## Historical localization/admission CI packages

[CI 43744797175 at abeabd1](https://github.com/zocs/inout-next/actions/runs/43744797175) completes all six serial targets on 2026-11-16. Its own tested export has 36 files / 1,084,607 bytes, including 1,011,196 raw JavaScript bytes and 312,635 gzip bytes measured separately per file. The source passes 131 Chromium cases, with no skipped, unexpected or flaky results. Downloaded packages are checked against this export, separately from the local export and earlier releases above.

| Target | CLI + static web ZIP | Transitional native package |
| --- | ---: | --- |
| Linux x64 | 1.64MiB | 4.46MiB deb / 79.10MiB AppImage |
| Windows x86 | 1.39MiB | 2.74MiB NSIS installer |
| Windows x64 | 1.49MiB | 2.94MiB NSIS installer |
| macOS ARM64 | 1.53MiB | 4.20MiB DMG |
| macOS Intel | 1.55MiB | 4.28MiB DMG |
| Android arm64 | — | 15.35MiB unsigned APK |

The unsigned APK is **16,099,144 bytes**, with one 13,254,104-byte arm64 Rust library and the same 36 indexed web resources. Exact manifest, single-ABI, ELF/native ZIP alignment and two locked compilation children pass inspection; signature/install/device/16 KiB runtime acceptance is separate. The desktop packages still use Tauri and their platform webviews, with the same dependency/offline/old-Linux boundaries above. These CI artifacts are an unreleased source checkpoint; the separately signed 15.39MiB Android preview.2 and published desktop preview.1 retain their own package provenance.

## Functional Slint desktop trial

The current opt-in Slint desktop client controls the real service without a WebView. It is separate from the published Tauri previews and the throwaway toolkit fixtures below. Local optimized Linux packaging uses the trial's own exact browser export and source/framework notices, and is verified by running outside the checkout with an unrelated working directory.

| Local trial item | Bytes | MiB |
| --- | ---: | ---: |
| Slint native executable | 18,752,320 | 17.88 |
| Exact static HTTP export, 36 files | 1,084,605 | 1.03 |
| Native executable + web + notices ZIP | 9,216,822 | 8.79 |

The normal Linux dependency graph selects winit/FemtoVG and contains no Tauri/Wry/WebKit, Skia or software-renderer runtime. Loaded mappings of the tested process contain no embedded browser, Node or Python runtime. The executable directly needs fontconfig, libc, libm and the GCC/runtime loader; OpenGL/GLX, X11, xkbcommon and FreeType are also loaded through OS services. These libraries, graphics drivers, fonts, desktop dialog/clipboard services and the external browser are not bundled by the ZIP. The local executable imports GLIBC symbols through **2.43**, so this local archive is not an Ubuntu22.04/18.04 compatibility artifact. A target build and its complete dependency/runtime floor must be inspected separately.

One traced launch reaches host-ready at 118.270 ms and first/ready render callbacks at 276.133/276.155 ms. These are individual callback markers before confirmed display, not cold-start, median, FPS or input-to-display results. A preceding functional Slint checkpoint was sampled for 65.34 seconds with 66 observations, a running loopback service, completed receipts, warm font/renderer caches and no GUI/transfer/accessibility observation during sampling. Median RSS/PSS/private memory was 99.45/88.24/83.32 MiB with 13 threads. No CPU-tick increase was detected at 10 ms accounting resolution. That earlier checkpoint remains separate from the current completion-correction package; neither interval establishes zero actual CPU, complete GPU memory, energy cost or an intrinsic toolkit ranking.

Native interaction checks cover byte-exact relocated transfers, optional-key/public-write policy, actual folder selection/cancellation, multiple address selection/copy, external browser navigation and confirmed Stop/normal-close cleanup. Current-run receipts and canceled interruption preserve the listener/admitted body; durable history/resume and individual cancellation remain pending. Three languages/four themes and explicit startup reduced motion are implemented. Native OS motion policy, assistive/CJK input, Windows32/offline/installed operation, mobile/Apple/F-Droid distribution and matched sustained presentation measurements remain independent adoption gates. See [native evaluation](../native-gui.md) and [trial usage/licensing](../../apps/native-slint/README.md).

## Exploratory Rust GUI comparison

The [native GUI evaluation](../native-gui.md) records a separate exploratory Linux x86_64/X11/Intel comparison of egui 0.37/Glow and Slint 1.18.1/FemtoVG. Both throwaway fixtures use the real Rust service, the same optimized profile and 10,000 filename rows with system-font fallback. Thirty warm launches and six checksum-verified, deliberately throttled 64 MiB workloads passed.

Populated render-callback medians are 125.96/181.08 ms, before OS presentation. Single-executable ZIPs are 5.01/7.85 MiB and idle process PSS is 46.32/48.09 MiB. These are concrete probe measurements, excluding static Next assets, external libraries/fonts, installed runtime cost and complete GPU allocations. Selected font faces/layouts and active render cadence differ; there is no normalized FPS/CPU ranking, cold-start result or mobile/Windows/Apple package acceptance. The reported animation sample slice is post-transfer. No Rust-rendered application preview is released; existing Tauri package sizes above remain separate.

## Python and repository language percentages

Python supports product/configuration synchronization, key/icon generation, packaging, identity/runtime checks and tool tests. These files are development tools, not HTTP request handlers. Node builds the static Next assets. Transitional Tauri uses local assets with its platform webview; Slint renders its own native controls and serves the packaged export to external browsers.

GitHub's language percentages describe detected source-code volume. They do not measure executable composition, memory, CPU time or request latency. Moving a build helper from Python to Rust would not accelerate a download path that never executes that helper. Keep runtime profiling and archive contents as the evidence for performance/size decisions.
