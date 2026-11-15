# Package size and runtime

Measured on 2026-11-15 for the experimental source, with desktop-package comparisons against [FileInfra 0.5.13](https://github.com/zocs/FileInfra/releases/tag/v0.5.13). Sizes use MiB (1,048,576 bytes). Archive size, uncompressed executable size, system dependencies and process memory are different quantities. Local builds and verified CI packages are identified separately below. [The desktop prerelease](preview.md) distributes the verified CI packages; no stable release is published yet.

## Current local build

The current Linux x64 release build uses Rust 1.99.1 and Node 26.13.1. Its export and executables were verified together, including a relocated CLI ZIP and a streamed 128 MiB runtime check.

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

These six native packages come from the same c004d68 CI checkpoint. Extraction verifies the native CPU/subsystem and every packaged web resource; it is separate from installing or running the GUI on a device. Both macOS bundles succeeded on their first packaging attempt, so this run does not demonstrate an actual resource-busy recovery.

| Platform / package | inout-next 0.1.0 checkpoint | FileInfra0.5.13 | Runtime distinction |
| --- | ---: | ---: | --- |
| Windows x64 installer | 2.90MiB | 11.39MiB | Tauri uses system WebView2; current NSIS downloads its bootstrapper. FileInfra includes Flutter runtime |
| Windows x86 installer | 2.72MiB | No corresponding asset | Earlier 32bit target operation on Windowsx64 is verified;32bit OS/installation/offline remain separate |
| Linux x64 deb | 4.42MiB | 9.63MiB | Tauri declares system WebKitGTK4.1/GTK3 dependencies; package size excludes their installation cost |
| Linux x64 AppImage | 79.06MiB | 22.41MiB | Current Tauri bundle includes a large WebKitGTK/JavaScriptCore/GTK/ICU library closure |
| macOS Intel native archive | 4.25MiB DMG | 20.03MiB ZIP | Different archive formats; Tauri uses Apple's system webview |
| macOS ARM64 DMG | 4.16MiB | 19.93MiB ZIP | Different archive formats and native runtimes |
| Android arm64 APK | Not built | 11.14MiB | No inout-next mobile-size or device claim yet |

FileInfra's Windowsx64 portable **GUI** ZIP is15.42MiB; it is not equivalent to the1.45MiB inout-next **CLI** ZIP. The former contains a20.29MiB uncompressed Flutter engine,6.72MiB Dart AOT application and5.23MiB dufs library, plus plugins/assets/runtime files. The inout-next uncompressed Windows native executable is11.44MiB forx64 and9.07MiB forx86, plus static resources and external WebView2 requirements. Extraction does not establish offline GUI portability.

That Linux AppImage's logical regular-file inventory totals approximately258.03MiB before SquashFS compression. Its largest libraries are WebKitGTK85.84MiB, JavaScriptCore32.20MiB and ICU data28.12MiB; the native executable is14.81MiB. The static Next export is1.00MiB. Library packaging, compression/deduplication and compatibility strategy need independent work; shrinking a package by omitting a required runtime does not establish a working portable artifact.

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

The built-in native test separately caps reads at 64 MiB and provides a local timing sample, not this checksum-verified comparison. GUI startup, total application/child-webview memory, native IPC, physical LAN, cold disk, slow clients and mobile performance still need matched target measurements. FileInfra's server already uses Rust; replacing its Flutter shell does not itself accelerate file transfers. ETag/If-Range validation, durable resume and feature/platform parity remain incomplete.

## Python and repository language percentages

Python supports product/configuration synchronization, key/icon generation, packaging, identity/runtime checks and tool tests. These files are development tools, not HTTP request handlers. Node builds the static Next assets. Tauri uses the resulting local assets with its platform webview.

GitHub's language percentages describe detected source-code volume. They do not measure executable composition, memory, CPU time or request latency. Moving a build helper from Python to Rust would not accelerate a download path that never executes that helper. Keep runtime profiling and archive contents as the evidence for performance/size decisions.
