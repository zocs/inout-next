# Package size and runtime

Measurements record experimental checkpoints from 2026-11-15–27, with desktop-package comparisons against [FileInfra 0.5.13](https://github.com/zocs/FileInfra/releases/tag/v0.5.13). Sizes use MiB (1,048,576 bytes). Archive size, uncompressed executable size, system dependencies and process memory are different quantities. Local builds, historical [Windows fallback/input CI packages](#windows-fallback-and-input-ci-packages) and the latest [partial embedded-source CI](#embedded-source-ci-packages) retain separate provenance. [Desktop preview.1](preview.md) and [Android preview.2](android-preview.md) distribute their earlier verified CI packages; no stable release is published yet.

## Local desktop checkpoint

The Linux x64 desktop checkpoint uses Rust 1.99.1 and Node 26.13.1. Its export and executables were verified together, including a relocated CLI ZIP and a streamed 128 MiB runtime check. These measurements precede the Android adaptation; the Android APK and its own export are measured separately below.

| Item | Bytes | MiB |
| --- | ---: | ---: |
| Static Next export, 35 files | 1,050,698 | 1.00 |
| Uncompressed Rust CLI | 3,977,248 | 3.79 |
| Uncompressed Tauri native executable | 15,213,808 | 14.51 |
| CLI + static web ZIP | 1,721,866 | 1.64 |

The export grew by 22,310 bytes compared with the earlier 32-file export. The CLI and pages deliver no Python or Node runtime. The native executable excludes external webview libraries and is not an AppImage-size measurement. An older local native executable has insufficient build provenance for attributing growth specifically to diagnostics or branding; total native diagnostics memory and startup frame times remain unmeasured.

## Local CLI symbol stripping

The 2026-11-27 Linux x64 comparison uses Rust 1.100.0 and identical CLI/HTTP/storage source. The root workspace adds only `strip = "symbols"` to its release profile and removes the unused HTTP middleware dependency. Existing Cargo optimization and unwind defaults apply; native workspaces have their own profiles.

| Release executable | Bytes | MiB |
| --- | ---: | ---: |
| Before symbol stripping | 4,081,128 | 3.89 |
| Stripped current CLI | 2,900,520 | 2.77 |

The reduction is 1,180,608 bytes, or **28.9%**. ELF inspection confirms removal of the static symbol table. Each exact executable passes help, public-page/status access, unauthorized rejection, authenticated listing, full/HEAD/Range downloads, upload and download/disk hashes, and Ctrl-C normal exit with listener release. This is a local executable-size and functional comparison; existing CI archives and older-OS/other-target readiness keep their recorded scope. It does not measure transfer speed, process memory or package savings. Stripped builds provide less symbolic backtrace detail; use an unstripped or debug build for diagnosis.

All three Rust lockfiles remove only `tower-http 0.7.1` and its now-unreachable `http-range-header 0.4.2`, preserving every surviving package version/checksum. The Tauri composition retains its separately required `tower-http 0.6.11`.

## Published CLI delivery

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

## Published transitional native packages

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

The latest [CI at da7a770](https://github.com/zocs/inout-next/actions/runs/45864654594) exports **36 files / 1,098,091 bytes**, with **1,020,555 raw JavaScript bytes** and **315,543 gzip bytes**, measured separately per file. Its Linux browser suite passes146 cases with no skipped, unexpected or flaky results. The accepted Linux, Windows x86/x64, macOS x64 and Android artifacts below match that exact export; this is a separate build from the local checkpoint.

The locally verified preview/management integration at `77abf6f` exports **36 files totalling 1,098,089 bytes**. JavaScript totals **1,020,553 raw bytes** and **315,532 gzip bytes**, measured separately per file. The same export passes **146 Chromium cases**, including obsolete management replies, independent directory permissions and refresh of same-metadata content. Every exported resource is also served byte-for-byte by that checkpoint's native executable. The local `nsfs` storage repair preserves this export; its later CI uses the separate build measured above, with no additional native GUI runtime acceptance.

The earlier local localization/permission checkpoint exported 36 files / 1,084,605 bytes, with 1,011,194 raw JavaScript bytes and 312,634 per-file gzip bytes, and passed 131 Chromium cases. Its figures and the earlier release-package measurements retain their own exports and provenance.

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

The current opt-in Slint desktop client controls the real service without a WebView. It is separate from the published Tauri previews and the throwaway toolkit fixtures below. The redesigned local Linux x64 package at `77abf6f` uses Rust 1.100.0 and Node 26.13.1, embeds its own exact browser export and includes source/framework/qrcodegen/tempfile notices. Its executable runs outside the checkout from an unrelated working directory and shares a folder containing itself. The following package/runtime figures retain that checkpoint; the `nsfs` repair's rebuilt executable has separate bytes and no additional GUI runtime acceptance.

| Redesigned local trial item | Bytes | MiB |
| --- | ---: | ---: |
| Slint executable, including browser assets | 20,185,896 | 19.25 |
| Embedded browser export, 36 files | 1,098,089 | 1.05 |
| Native executable + notices/usage ZIP, without duplicate `web/` | 9,339,760 | 8.91 |

The ZIP's executable is byte-identical to the validated native run; its final usage and notices match the packaging source. Actual X11 checks cover language/accent/appearance choices, keyboard setup traversal, a 780×600 viewport, Chinese IME composition, invalid-port rejection, ordinary-close draft restoration and manual address copying. All 36 embedded HTTP resources match the export. The same original 2 MiB upload survives presentation changes and canceled Stop/normal-close before publication and exact disk/download/suffix verification; confirmed Stop cleans another unfinished stage and the listener. At that checkpoint, masked-key keyboard/PRIMARY copying is a confirmed limitation, and local system-picker selection is unverified. The subsequent clipboard correction is measured separately below. This is a local functional checkpoint, with no new presented-frame, memory or throughput comparison.

The earlier local completion-correction package used separate browser files:

| Earlier local trial item | Bytes | MiB |
| --- | ---: | ---: |
| Slint native executable | 18,752,320 | 17.88 |
| Exact static HTTP export, 36 files | 1,084,605 | 1.03 |
| Native executable + web + notices ZIP | 9,216,822 | 8.79 |

The normal Linux dependency graph selects winit/FemtoVG and contains no Tauri/Wry/WebKit, Skia or software-renderer runtime. Loaded mappings of the tested process contain no embedded browser, Node or Python runtime. The executable directly needs fontconfig, libc, libm and the GCC/runtime loader; OpenGL/GLX, X11, xkbcommon and FreeType are also loaded through OS services. These libraries, graphics drivers, fonts, desktop dialog/clipboard services and the external browser are not bundled by the ZIP. The local executable imports GLIBC symbols through **2.43**, so this local archive is not an Ubuntu22.04/18.04 compatibility artifact. A target build and its complete dependency/runtime floor must be inspected separately.

One traced launch reaches host-ready at 118.270 ms and first/ready render callbacks at 276.133/276.155 ms. These are individual callback markers before confirmed display, not cold-start, median, FPS or input-to-display results. A preceding functional Slint checkpoint was sampled for 65.34 seconds with 66 observations, a running loopback service, completed receipts, warm font/renderer caches and no GUI/transfer/accessibility observation during sampling. Median RSS/PSS/private memory was 99.45/88.24/83.32 MiB with 13 threads. No CPU-tick increase was detected at 10 ms accounting resolution. That earlier checkpoint remains separate from the current completion-correction package; neither interval establishes zero actual CPU, complete GPU memory, energy cost or an intrinsic toolkit ranking.

The earlier local functional checkpoint covered byte-exact relocated transfers, optional-key/public-write policy, actual folder selection/cancellation, multiple address selection/copy, external browser navigation and confirmed Stop/normal-close cleanup. The redesigned Linux X11 checkpoint above verifies picker cancellation; new picker selection remains unverified. Current-run receipts and canceled interruption preserve the listener/admitted body; durable history/resume and individual cancellation remain pending. Three languages/four themes and explicit startup reduced motion are implemented. Native OS motion policy, full assistive/CJK input coverage, Windows32/offline/installed operation, mobile/Apple/F-Droid distribution and matched sustained presentation measurements remain independent adoption gates. See [native evaluation](../native-gui.md) and [trial usage/licensing](../../apps/native-slint/README.md).

## Local native clipboard correction

The subsequent local Linux x64 release with the masked/shown key clipboard correction, built with Rust1.100.0, is **20,194,472 bytes (19.26 MiB)**, including its browser assets. Its SHA-256 is `fe9dcbf3d1ea9ddb2c03f8e24617793e4717c23460db76004ca12a8e95e61b95`. Direct ELF imports reach GLIBC2.43 and require fontconfig/libgcc/libm/libc/the system loader; this does not establish the complete OS dependency floor. This executable is distinct from the earlier local ZIPs and all CI packages below.

The exact executable passes a new owned X11 run with independent Default/PRIMARY reads, authenticated content checks after protected Cut and replacement paste, ordinary Unicode folder/address operations, fresh-profile Rime composition and normal-close cleanup. Local native tests pass24 with GUI/22 without it, plus the separate8-process×32 preference fixture in both configurations; fmt, strict all-target Clippy and locked optimized build pass. The guard preserves the selected backend and renderer features without a toolkit/package upgrade. These checks add no new frame, memory, throughput, Windows/macOS/Wayland or installed/device result. GTK portal selection remains unverified, and historical callback/package/compatibility figures above retain their own scope.

The new local Linux x64 ZIP of that exact tested executable is **9,345,519 bytes (8.91 MiB)**, SHA-256 `5e24778dd0593995156e20a6b8bd01b866ab866984137bf680e4f601d38c6594`. It has seven regular-file members: executable, product metadata, source/framework notices and usage, with no duplicate `web/`. Current usage/notices match the packaging inputs; all36 exported resource byte spans, totaling1,098,089bytes, are present in its executable. Archive content inspection adds no new archive execution, signing, installed/offline or replacement-release acceptance.

The same corrected source also passes local Rust **1.99.1** all-target checks on the Linux host for Linux with/without GUI, Windows MSVC x86/x64 and macOS Intel/ARM64. These checks compile regression targets without executing them and do not replace target tests or the cancelled ARM64 CI job. A separate locked Linux optimized build produces **20,185,560 bytes (19.25 MiB)**, SHA-256 `697a8fcaba5de11f8ec46df65bc0803e6efd53a1b245c6d64c00ecdfacb2ac36`; its36 resource byte spans also match the export and direct GLIBC imports reach2.43. That compiler-only executable has no new GUI/HTTP/installed/device acceptance and is distinct from the Rust1.100.0 executable in the new local ZIP.

## Slint trial serial CI packages

[Historical CI 43844556784 at 3d0e656](https://github.com/zocs/inout-next/actions/runs/43844556784) completes all six jobs serially on 2026-11-16. All five desktop targets pass locked core/host, Tauri composition and Slint tests, strict Clippy and optimized builds. Linux also passes 56 tool and 131 Chromium cases, with no skipped, unexpected or flaky browser results, plus streamed 128 MiB and relocated 8 MiB integrity/cleanup checks. These artifacts precede the Windows fallback/input fixes; their measurements remain separate from current CI, the local functional package above and published Tauri previews.

This CI's own export contains **36 files / 1,084,607 bytes**, including **1,011,196 raw JavaScript bytes** and **312,635 gzip bytes** measured separately per file. Five CLI ZIPs, five Slint ZIPs, six transitional desktop packages and the unsigned Android APK match these exact web resources. Native ZIP inspection also checks product metadata, source/framework notices, archive checksums and actual ELF/PE/Mach-O architecture. Build tools are absent from the delivered runtime.

| Target | Slint executable | Slint + HTTP export + notices ZIP | CLI + HTTP export ZIP |
| --- | ---: | ---: | ---: |
| Linux x64 | 18,714,064 bytes / 17.85MiB | 9,008,964 bytes / 8.59MiB | 1.65MiB |
| Windows x86 | 11,938,304 bytes / 11.39MiB | 6,830,899 bytes / 6.51MiB | 1.39MiB |
| Windows x64 | 12,506,624 bytes / 11.93MiB | 6,903,747 bytes / 6.58MiB | 1.49MiB |
| macOS ARM64 | 11,625,904 bytes / 11.09MiB | 6,491,529 bytes / 6.19MiB | 1.53MiB |
| macOS Intel | 12,594,120 bytes / 12.01MiB | 6,798,155 bytes / 6.48MiB | 1.56MiB |

The CI Linux Slint executable directly imports GLIBC through **2.35**, compared with **2.43** in the separate local build above. Its direct library requirements include fontconfig, libc, libm and the GCC/runtime loader. Graphics drivers, fonts, display/dialog/clipboard services, the external browser and their transitive library requirements are not bundled or fully audited by this ZIP inspection. No Ubuntu18.04, complete minimum-runtime or target-device clearance follows from the direct import ceiling.

Windows Slint executables have the expected x86/x64 PE machine and GUI subsystem2; CLI executables use subsystem3. The historical 6.58MiB Windows x64 Slint ZIP and FileInfra's 15.42MiB GUI ZIP describe different runtimes and feature sets, so the delivery-byte comparison does not establish relative memory, throughput or offline operation. Slint does not embed WebView; that CI provides no Windows/macOS GUI, relocated/offline or installed acceptance. Source remains MIT with separately included Slint royalty-free desktop terms; mobile/store licensing and final renderer adoption are independent gates.

The same run retains these transitional Tauri packages:

| Target / package | Bytes | MiB |
| --- | ---: | ---: |
| Linux x64 AppImage | 82,946,552 | 79.10 |
| Linux x64 deb | 4,692,272 | 4.47 |
| Windows x86 NSIS | 2,868,486 | 2.74 |
| Windows x64 NSIS | 3,085,443 | 2.94 |
| macOS ARM64 DMG | 4,412,551 | 4.21 |
| macOS Intel DMG | 4,507,479 | 4.30 |

Both macOS bundles succeed on their first attempt; no resource-busy recovery is exercised in this run. AppImage/deb/NSIS/DMG outputs remain Tauri packages with the platform webview and compatibility requirements documented above. They are not Slint installers or replacements for the published preview assets.

The transitional unsigned arm64 APK is **16,149,684 bytes (15.40 MiB)**, with one **13,304,624-byte Rust library** and this CI's same 36 indexed web resources. Seven Kotlin cases and both actual locked release-library compilation children pass; both source lockfiles remain unchanged. Manifest checks retain `inout`, the `.preview` ID, code100001, minAPI24 and targetAPI36. Exact raw resources, single ABI and ELF/native ZIP16KiB alignment pass inspection, while signature verification confirms it is unsigned. There is no Slint APK, new signing/install/emulator/device/16KiB runtime or store acceptance; the signed 15.39MiB preview.2 retains its separate provenance.

## Windows source build with software fallback

The Windows-only software-renderer fallback and explicit native language/theme value actions have separate optimized source-build measurements:

| Target | Native executable bytes | MiB |
| --- | ---: | ---: |
| Windows x86 | 13,051,904 | 12.45 |
| Windows x64 | 13,569,536 | 12.94 |

Both pinned Rust/MSVC builds pass14 native tests and strict Clippy. Actual Windows10x64 operation passes default-renderer startup, localized value/label and theme retention, CJK-root files, public-write policy and each same2MiB upload through presentation changes and canceled Stop/close. Final disk/download hashes, confirmed Stop, normal exit and zero listener/stage checks pass. [Native evaluation](../native-gui.md#windows-default-renderer-and-input) retains the remaining selection/IME/assistive, real32bit OS, installed/offline and presentation gates.

These are source-built executables tested with the earlier CI's exact 36-file / 1,084,607-byte browser resources. They are separate from the complete historical ZIP sizes above and current CI packages below; no newly measured local ZIP size is inferred. The added renderer is Windows-only. Linux/macOS feature graphs and all historical FileInfra, Tauri, core-transfer and exploratory measurements remain unchanged. There is no Node/Python/WebView app runtime; OS fonts/desktop services and an external browser still have their own costs. Callback readiness does not measure displayed frames, throughput, memory or energy.

## Windows fallback and input CI packages

[CI 44239092778 at d42048d](https://github.com/zocs/inout-next/actions/runs/44239092778) passes all six jobs on 2026-11-18 with nonoverlapping actual intervals. All five desktop targets pass locked core/host, 15 Tauri composition and 14 Slint tests, strict Clippy and optimized builds. Linux also passes 56 tool/131 Chromium cases and streamed 128 MiB/relocated 8 MiB integrity/cleanup checks; Android passes seven Kotlin cases and two actual locked arm64 release-library children. [Platform status](../platforms.md#ci) records the per-target counts and remaining gates.

All nine downloaded artifact digests match GitHub's declarations. Five CLI ZIPs, five Slint ZIPs, six transitional desktop packages and the unsigned Tauri APK match this run's own **36 files / 1,084,607 bytes** of web resources. JavaScript totals **1,011,196 raw bytes** and **312,635 gzip bytes**, measured separately per file. Native ZIPs additionally pass actual ELF/PE/Mach-O, product, source/framework-notice and archive-checksum inspection. The Windows executables include the software fallback and validated native language/theme value actions.

| Target | Slint executable | Slint + HTTP export + notices ZIP | CLI + HTTP export ZIP |
| --- | ---: | ---: | ---: |
| Linux x64 | 18,719,312 bytes / 17.85MiB | 9,012,382 bytes / 8.59MiB | 1,727,770 bytes / 1.65MiB |
| Windows x86 | 12,997,120 bytes / 12.40MiB | 7,286,920 bytes / 6.95MiB | 1,459,789 bytes / 1.39MiB |
| Windows x64 | 13,514,240 bytes / 12.89MiB | 7,334,321 bytes / 6.99MiB | 1,565,316 bytes / 1.49MiB |
| macOS ARM64 | 11,642,416 bytes / 11.10MiB | 6,494,568 bytes / 6.19MiB | 1,607,703 bytes / 1.53MiB |
| macOS Intel | 12,598,216 bytes / 12.01MiB | 6,801,162 bytes / 6.49MiB | 1,634,810 bytes / 1.56MiB |

Both exact Windows ZIP executables pass default-renderer startup, Simplified/Traditional/English value and label readback, retained Ocean theme and normal close. The public CI probe starts no sharing. The separate source-built Windows10x64 checks above cover CJK-root operations and original-upload continuity; neither scope establishes a real32bit OS, standard option/IME/screen-reader, installed/offline or presented-frame result. The 6.99MiB current x64 Slint ZIP and FileInfra's 15.42MiB GUI ZIP have different features and runtime packaging; delivery size does not rank throughput, memory or energy. No new performance comparison was performed.

The Linux Slint executable directly imports GLIBC through **2.35**, while this run's CLI and Tauri executables import through **2.34**. The Slint ZIP retains its OS graphics/fonts/display/dialog/clipboard and external-browser requirements. Inspection does not establish a complete transitive runtime floor, Ubuntu18.04 compatibility or macOS GUI operation. No Node/Python/WebView runtime is embedded in the Slint app.

This run's transitional Tauri packages remain distinct from the Slint ZIPs:

| Target / package | Bytes | MiB |
| --- | ---: | ---: |
| Linux x64 AppImage | 82,946,552 | 79.10 |
| Linux x64 deb | 4,692,174 | 4.47 |
| Windows x86 NSIS | 2,868,757 | 2.74 |
| Windows x64 NSIS | 3,085,024 | 2.94 |
| macOS ARM64 DMG | 4,412,380 | 4.21 |
| macOS Intel DMG | 4,507,161 | 4.30 |

Both macOS bundles succeed on their first attempt, exercising no resource-busy recovery. Debian/NSIS emitted-identity fixtures are separate from installed upgrade/rebranding acceptance; the NSIS fixtures use their own copied inputs, distinct from the distribution installer payloads. Tauri retains its system-webview and compatibility requirements. These artifacts have not replaced the released previews.

The unsigned Tauri arm64 APK is **16,149,852 bytes (15.40MiB)**, with one **13,304,800-byte Rust library** and the same 36 raw web resources. Manifest checks retain `inout`, the `.preview` ID, code100001, minAPI24 and targetAPI36. Both locked compilation children exit successfully with source lockfiles unchanged; exact resources, single ABI and ELF/native ZIP16KiB alignment pass inspection. It has no signature-based installation clearance, Slint renderer, new installed-device/emulator/16KiB-runtime or store result. The signed 15.39MiB preview.2 remains a separate delivery. Source stays MIT with the included Slint royalty-free2.0 attribution and separate mobile/channel gates.

## Embedded source CI packages

### Current embedded source checkpoint

[CI45864654594 at da7a770](https://github.com/zocs/inout-next/actions/runs/45864654594) succeeds on Linux, Android, Windows x86/x64 and macOS x64. macOS arm64 is cancelled before acquiring a hosted runner because of capacity constraints, so the run ends in failure and complete six-target acceptance remains pending. The five executed jobs do not overlap. All eight produced artifact envelopes have verified GitHub API digests; no macOS arm64 package exists for this run. These packages do not replace the published previews.

| Target | CLI ZIP bytes | Server bytes | Slint ZIP bytes | Slint executable bytes | Transitional Tauri package bytes |
| --- | ---: | ---: | ---: | ---: | --- |
| Linux x64 | 1,752,662 | 4,089,424 | 9,122,352 | 20,178,128 | AppImage82,987,512; deb4,737,374 |
| Windows x86 | 1,469,046 | 2,391,552 | 7,361,903 | 14,397,952 | NSIS2,880,076 |
| Windows x64 | 1,574,725 | 2,988,544 | 7,437,184 | 14,994,944 | NSIS3,093,686 |
| macOS x64 | 1,647,494 | 3,475,544 | 6,895,482 | 14,001,168 | DMG4,519,558 |

All four Slint archives have seven members, omit duplicate `web/` and contain every exact path and original payload from the36file/1,098,091byte export in their executable bytes. CLI/Tauri resources and ELF/PE/Mach-O architectures pass inspection. Both exact Windows Slint executables pass default-renderer startup, Simplified/Traditional/English readback, retained theme and normal close; NSIS identity uses a separate rendered/copy-input fixture. macOS x64 creates its DMG on the first attempt. No macOS GUI, full Windows input/IME/clipboard, real32bit OS, installed/offline or signing clearance follows.

Linux passes146 Chromium cases and streamed128MiB/relocated8MiB CLI integrity, public listing, empty staging and Ctrl-C cleanup, with RSS growth3,088,384bytes (2.95MiB) and1,998,848bytes (1.91MiB). These CLI probes do not measure native GUI presentation or matched performance.

The unsigned Tauri arm64 APK is **16,213,704 bytes (15.46MiB)**, with one **13,365,056-byte Rust library** and the same36web resources. Its SHA-256 is `7e8f38dbd6b152e897e4b1dad2deb7c7ab3ab6cae8aced7df6d6b754ebeb75b1`. Seven Kotlin cases, two actual locked release-library builds, unchanged source locks and exact manifest/resources/single-ABI/ELF/native ZIP16KiB alignment pass. It has no v1 signature entries or APK signing block, and establishes no new device,16KiB runtime, signing, Slint mobile or store acceptance.

### Prior embedded source checkpoint

[CI45837200172 at386005d](https://github.com/zocs/inout-next/actions/runs/45837200172) succeeds on Linux and Android, then still fails Windows x86 concurrent preference saving after closing staged handles; Windows x64 is cancelled and macOS skipped. Its three executed jobs do not overlap. The following measurements describe that run's accepted Linux and unsigned Android artifacts, before the Windows process-coordination repair. All five produced artifact envelopes have verified GitHub API digests.

| Linux x64 item | Bytes | MiB |
| --- | ---: | ---: |
| CLI + static web ZIP | 1,752,649 | 1.67 |
| Server executable | 4,089,424 | 3.90 |
| Native Slint ZIP, embedded resources and notices | 9,121,870 | 8.70 |
| Slint executable | 20,178,000 | 19.24 |
| Transitional Tauri AppImage | 82,987,512 | 79.14 |
| Transitional Tauri deb | 4,737,280 | 4.52 |
| Web export,36files | 1,098,091 | 1.05 |

The native ZIP has seven members and omits duplicate `web/`. Every exact export path and original payload is present in the executable's bytes; this inspection is separate from HTTP serving or a GUI launch. CLI and Tauri resource layouts, executable ELF architecture, notices and emitted identity pass inspection. Slint directly imports GLIBC through2.35 and Tauri through2.34; full transitive dependency and old-distribution runtime floors remain open. Linux verifies streamed128MiB and relocated8MiB CLI integrity, suffix/public listing, empty staging and Ctrl-C cleanup; RSS grows2,064,384bytes (1.97MiB) and1,110,016bytes (1.06MiB) respectively. These CLI probes add no native GUI, presented-frame or matched-performance result.

The unsigned Tauri arm64 APK is **16,213,728 bytes (15.46MiB)**, with one **13,365,056-byte Rust library** and the same36resources/1,098,091bytes. Its SHA-256 is `d30ec9bd39e4e4bf18ffe2e0080e8a0973ea6273247b4365305d34015e6729d9`. Seven Kotlin cases, two actual locked release-library builds, unchanged source locks and exact manifest/resources/single-ABI/ELF/native ZIP16KiB alignment pass. The package has no v1 signature entries or APK signing block. It establishes no new device,16KiB runtime, signing, installed/offline, Slint mobile or store acceptance. Windows/macOS produce no newly accepted packages; their historical measurements above retain their named source scope.

## Exploratory Rust GUI comparison

The [native GUI evaluation](../native-gui.md) records a separate exploratory Linux x86_64/X11/Intel comparison of egui 0.37/Glow and Slint 1.18.1/FemtoVG. Both throwaway fixtures use the real Rust service, the same optimized profile and 10,000 filename rows with system-font fallback. Thirty warm launches and six checksum-verified, deliberately throttled 64 MiB workloads passed.

Populated render-callback medians are 125.96/181.08 ms, before OS presentation. Single-executable ZIPs are 5.01/7.85 MiB and idle process PSS is 46.32/48.09 MiB. These are concrete probe measurements, excluding static Next assets, external libraries/fonts, installed runtime cost and complete GPU allocations. Selected font faces/layouts and active render cadence differ; there is no normalized FPS/CPU ranking, cold-start result or mobile/Windows/Apple package acceptance. The reported animation sample slice is post-transfer. No Rust-rendered application preview is released; existing Tauri package sizes above remain separate.

## Python and repository language percentages

Python supports product/configuration synchronization, key/icon generation, packaging, identity/runtime checks and tool tests. These files are development tools, not HTTP request handlers. Node builds the static Next assets. Transitional Tauri uses local assets with its platform webview; Slint renders its own native controls and serves the packaged export to external browsers.

GitHub's language percentages describe detected source-code volume. They do not measure executable composition, memory, CPU time or request latency. Moving a build helper from Python to Rust would not accelerate a download path that never executes that helper. Keep runtime profiling and archive contents as the evidence for performance/size decisions.
