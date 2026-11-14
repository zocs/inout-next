# Package size and runtime

Baseline measured on 2026-11-14 from earlier 0.1.0 experimental CI artifacts and [FileInfra 0.5.13 release](https://github.com/zocs/FileInfra/releases/tag/v0.5.13), before the download-buffer, diagnostic and wordmark extension. Sizes use MiB (1,048,576 bytes). Archive size, uncompressed executable size, system dependencies and process memory are different quantities. The figures below describe that baseline, rather than the refreshed extension's packages.

## Baseline CLI delivery

| Target | CLI + static web ZIP | Uncompressed server executable |
| --- | ---: | ---: |
| Windows x86 | 1.35MiB | 2.23MiB |
| Windows x64 | 1.44MiB | 2.75MiB |
| Linux x64 | 1.60MiB | 3.79MiB |
| macOS ARM64 | 1.49MiB | 3.26MiB |
| macOS Intel | 1.51MiB | 3.24MiB |

Each archive includes32exported web resources totalling0.98MiB before compression, product metadata, licence and instructions. It contains no Python scripts, Node executable or node_modules. The Rust service and static pages need neither Python nor Node at runtime.

For context, [dufs0.47.0](https://github.com/sigoden/dufs/releases/tag/v0.47.0) Windowsx64 ZIP is1.68MiB and Linuxx64-musl tar.gz is1.96MiB. Different compression formats, libc targets and feature sets prevent treating these sizes as an exact feature-parity comparison. A modern static Next page does not require delivering a Next server.

## Baseline native packages

| Platform / package | Earlier inout-next 0.1.0 | FileInfra0.5.13 | Runtime distinction |
| --- | ---: | ---: | --- |
| Windows x64 installer | 2.81MiB | 11.39MiB | Tauri uses system WebView2; current NSIS downloads its bootstrapper. FileInfra includes Flutter runtime |
| Windows x86 installer | 2.63MiB | No corresponding asset | Earlier 32bit target operation on Windowsx64 is verified;32bit OS/installation/offline remain separate |
| Linux x64 deb | 4.26MiB | 9.63MiB | Tauri declares system WebKitGTK4.1/GTK3 dependencies; package size excludes their installation cost |
| Linux x64 AppImage | 78.92MiB | 22.41MiB | Current Tauri bundle includes a large WebKitGTK/JavaScriptCore/GTK/ICU library closure |
| macOS Intel native archive | 4.11MiB DMG | 20.03MiB ZIP | Different archive formats; Tauri uses Apple's system webview |
| macOS ARM64 DMG | 4.03MiB | 19.93MiB ZIP | Different archive formats and native runtimes |
| Android arm64 APK | Not built | 11.14MiB | No inout-next mobile-size or device claim yet |

FileInfra's Windowsx64 portable **GUI** ZIP is15.42MiB; it is not equivalent to the1.44MiB inout-next **CLI** ZIP. The former contains a20.29MiB uncompressed Flutter engine,6.72MiB Dart AOT application and5.23MiB dufs library, plus plugins/assets/runtime files. The inout-next uncompressed Windows native executable is11.04MiB forx64 and8.76MiB forx86, plus static resources and external WebView2 requirements. An extracted native launch does not establish offline GUI portability.

That Linux AppImage's earlier content-size inventory totals approximately257.38MiB before SquashFS compression. Its largest libraries are WebKitGTK85.84MiB, JavaScriptCore32.20MiB and ICU data28.12MiB; the native executable is14.17MiB. The static Next export is0.98MiB. Library packaging, compression/deduplication and compatibility strategy need independent work; shrinking a package by omitting a required runtime does not establish a working portable artifact.

## Baseline core measurements

Three alternating fresh-process rounds ran on the same Linuxx64 host, using localhost HTTP,1MiB warmup and128MiB upload/download. The FileInfra input was its released `libdufs.so` hosted by a minimal C FFI driver; inout-next used its released CLI and static assets. Both used authenticated access. Actual filesystem/download SHA256 and64byte suffix Range matched in all six samples. Server PID RSS was sampled every10ms; client/observer memory was excluded.

| Median of3samples | inout-next Rust CLI | FileInfra bundled dufs core |
| --- | ---: | ---: |
|128MiB upload |0.135s |0.492s |
|128MiB download |0.538s |0.114s |
| Warmed baseline RSS |7.11MiB |7.17MiB |
| Sampled peak RSS |9.59MiB |9.21MiB |

These baseline samples show comparable core memory, faster uploads and slower downloads for the earlier inout-next engine. They motivated the bounded read-buffer comparison and same-handle Range positioning. Different authentication, filesystem/publication semantics and cache conditions remain part of the implementations; three localhost samples do not establish universal throughput or durability equivalence.

No Flutter GUI or Tauri WebView ran in this core benchmark. GUI startup, total application/child-webview memory, CPU, physical-LAN, slow-client, concurrency and mobile performance still need matched measurements. FileInfra's server already uses Rust; switching its Flutter shell to Tauri does not itself make the file-transfer engine faster. Feature and platform parity also remain incomplete in the preview.

## Python and repository language percentages

Python supports product/configuration synchronization, key/icon generation, packaging, identity/runtime checks and tool tests. These files are development tools, not HTTP request handlers. Node builds the static Next assets. Tauri uses the resulting local assets with its platform webview.

GitHub's language percentages describe detected source-code volume. They do not measure executable composition, memory, CPU time or request latency. Moving a build helper from Python to Rust would not accelerate a download path that never executes that helper. Keep runtime profiling and archive contents as the evidence for performance/size decisions.
