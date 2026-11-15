# Platform readiness

The project targets Windows, Linux, macOS and Android. Status distinguishes target compilation/tests, actual execution, installation, offline runtime, signing and store acceptance. Current 0.1.0 is experimental: [desktop preview.1](distribution/preview.md) and [Android foreground preview.2](distribution/android-preview.md) have separate packages and validation scopes. iOS remains scheduled for later native adaptation and device validation.

The application displays **inout** while repository/distribution names retain `inout-next`. Stable desktop installer/package labels remain `Inout Next`; Android's label uses the display name. See [branding and identity](branding.md). [Desktop CI at c004d68](https://github.com/zocs/inout-next/actions/runs/43649949553) completes all five targets serially; all five CLI ZIPs and six native packages match its 35 tested web resources and expected architectures. The Android extension has 32 native and 91 Chromium cases, bounded index/Kotlin asset checks and actual release-APK HTTP/browser/lifecycle checks on an API 36 x86_64 emulator with 4 KiB pages. Earlier Windows VM checks cover the prior picker/manager/shutdown behavior and do not establish current diagnostic IPC or startup frame times on target devices.

| Target | Verified scope | Remaining work |
| --- | --- | --- |
| Linux x64 | Ubuntu22.04 core/native/browser checks; CLI ZIP, AppImage/deb, emitted identity and exported-resource/ELF inspection | Native GUI/device/installation and signing |
| Windows x64 | Core/native tests, strict Clippy, CLI ZIP/NSIS, emitted identity, native PE/GUI subsystem and resource checks; bounded Windows10x64 native picker/manager/writable file/shutdown operation | Installation/uninstall/upgrade, offline WebView2, signing; move/rename unsupported |
| Windows x86 | Actual `i686-pc-windows-msvc` core/native tests/builds, CLI ZIP/NSIS and PE/resource checks; x86 native picker/readonly manager/download/Range/stop/close on Windowsx64 | Actual32bit Windows OS, installation/offline/signing; move/rename unsupported |
| macOS ARM64/Intel | Native macOS15 host tests/strict Clippy, CLI ZIP/app/DMG builds and extracted resources/Mach-O CPU checks | Native GUI/device/installation, older-macOS, signing/notarization and store profile |
| Linux ARM64 | Planned target | Core/desktop artifacts and target runtime |
| Ubuntu18.04/glibc2.27 | Independent compatibility goal | Actual compatible headless build/runtime and a separate WebKitGTK GUI strategy |
| Android arm64/API24+ | Signed CI foreground APK in preview.2, 15.39MiB, min24/target36, exact raw resources/single ABI, 16 KiB ELF and native ZIP alignment, retained preview certificate | Physical arm64, minimum-OS and 16 KiB runtime; two-device Wi-Fi; SAF/import/local manager, native foreground Service, AAB/store acceptance |
| Android x86_64 | Local release APK on API36/4 KiB emulator: authenticated transfers/Range, file management, Stop/restart/persistence, Home/Back/destruction/recreation and system/IME clearance | Additional OS/WebView/device coverage and separate public ABI delivery |
| Android armv7 | Later ABI coverage | Build/package and target runtime following the initial arm64 preview |
| iOS/iPadOS | Later adapter milestone | Security-scoped storage, permitted lifecycle, signing and device validation |

## Runtime and packaging

The final application targets a Rust-rendered GUI; Next.js remains its HTTP/browser manager. Existing Tauri artifacts and target evidence are transitional. [Native GUI evaluation](native-gui.md) records egui/Glow and Slint/FemtoVG execution on Linux x86_64/X11/Intel only. No Windows/mobile/Apple replacement GUI is built or released. API24, selected-renderer i686, CJK/IME/accessibility, mobile lifecycle, offline/installed packages and store results require independent verification; Slint's API26 minimum is an unresolved product decision.

CLI ZIPs contain the compiled server, static `web/`, product metadata, licence and usage instructions. They do not require Node or Python. Native packages additionally require a platform webview: WebKitGTK4.1/GTK3 on Linux and WebView2 on Windows. Current Windows NSIS uses a downloadable WebView2 bootstrapper; it is not verified offline GUI distribution. See [package size and runtime](distribution/size-and-runtime.md).

Current Ubuntu22.04 native executables import GLIBC through2.34. This rules out a glibc2.27 claim for those executables and does not audit the minimum versions of every bundled/system dependency. AppImage size and a successful extraction do not establish old-distribution runtime compatibility.

Tauri's [Windows installer guide](https://v2.tauri.app/distribute/windows-installer/#building-for-32-bit-or-arm) supports32bit `i686-pc-windows-msvc`. Executing that binary on Windowsx64 is distinct from testing a32bit Windows OS. Linux32bit is not a declared target. Modern browser/System WebView requirements remain relevant to the static Next export.

Windows upload/copy publication requires a filesystem supporting hard links; file/directory move and rename are disabled. Linux/Android/Apple publication and moves require native exclusive rename support. Directory copying is unsupported. The Android preview uses the existing filesystem provider inside app-private storage. Future SAF URIs require a separate adapter with truthful provider capabilities.

Android shares `files/shared`, separate from packaged runtime resources. Keep the app on screen: Home/root Back stops the listener, cancels unfinished transfers and leaves sharing stopped on return. Configuration recreation and true Activity destruction also leave sharing stopped; saved files survive, while clearing application data or uninstalling removes them. Another device's browser manages the share. This preview has no foreground Service or arbitrary-folder/local-manager capability.

## CI

Unreleased source adds localized themes, optional-key writes, separate rename/delete flags and application-side atomic policy/transfer controls. Native live-setting UI and durable recovery are pending. The earlier release CI counts below describe those exact packaged checkpoints, rather than this changed source or a new native renderer.

The [workflow](../.github/workflows/ci.yml) orders Ubuntu22.04 verification, Android arm64 packaging, Windows2022x86/x64 and native macOS15ARM64/Intel through one noncancelling queue and serial matrices. Android explicitly provisions command-line tools build13114758 through a fixed-commit setup action in an isolated SDK directory, then installs the pinned API/build-tools/NDK. It reuses that run's Linux-tested web export, verifies locked Cargo children and packages one optimized unsigned APK with manifest/resource/ABI/alignment checks. Direct preview signing follows artifact acceptance. macOS compiles once; only disk-image creation's resource-busy error permits up to three bundle-only attempts with retained logs. Other errors remain fatal. Resource/architecture/identity checks are separate from installed upgrades and device acceptance. The workflow does not declare LinuxARM64, Ubuntu18.04, iOS or offline Windows GUI artifacts.

The published desktop preview.1 checkpoint executes 45 core tests on Linux, 37 on each Windows target and 43 on each macOS target, reflecting platform-specific cases; every desktop target passes 23 native tests and strict Clippy. Linux also passes 19 Python tool and 89 Chromium tests plus streamed 128 MiB and relocated 8 MiB integrity/clean-stop checks. Both macOS bundles succeed on their first attempt; the bounded retry policy has real-child-process tests but no resource-busy event occurs in that run.

[CI 43704226914 at 4dd4b51](https://github.com/zocs/inout-next/actions/runs/43704226914) completes all six targets serially for the Android extension. Core counts remain Linux45 / Windows37each / macOS43each; every desktop target passes 32 native tests and strict Clippy, including protected-upload-alias and Android/desktop exit regressions. Linux passes 53 tool and 91 browser tests, streamed 128 MiB and relocated 8 MiB checks. Android passes seven Kotlin tests, two actual locked Cargo compilation children and inspection of the optimized arm64 APK against its exact 35-file / 1,052,299-byte export. The accepted APK is then signed and reinspected for [preview.2](distribution/android-preview.md). Current desktop CI builds do not replace the separately published preview.1 packages or their earlier device/extraction evidence.

## Store channels

The [official-source requirements checked2026-11-14](distribution/store-readiness.md) and [privacy inventory](distribution/privacy-data-inventory.md) define early preparation. All channels still need actual candidate validation.

| Channel | Required platform evidence | Current boundary |
| --- | --- | --- |
| Google Play | Current targetAPI36 separately from minimumAPI24 goal; intended ABIs/16KB/AAB/device checks, SAF, correct FGS/local-network adaptation, signing/forms/testing | Direct foreground APK only; no AAB, Play signing/submission or store result |
| F-Droid | Free-software dependency/assets/tool inventory; actual clean-source Next/Rust/Gradle recipe, static version codes/tags and reproducible-signing attempt | Durable Android text scaffold and direct APK exist; no fdroiddata build/MR or reproducibility result |
| iOS/iPadOS App Store | Current Apple SDK/deployment floor, scoped documents, permitted foreground/finite sharing, IPv6/privacy/network grants, signed archive/TestFlight/device checks | Later native adaptation; no indefinite-background-server claim |
| Mac App Store | Independent sandbox/signing/provision profile, directory/bookmark scope, network grants, signed app/pkg and disclosures | Separate from direct-distribution DMGs; no store candidate |

SDK/policy requirements are rechecked before submission. Build/runtime readiness, signing continuity and store acceptance remain distinct.

Different Android package IDs can share FileInfra's signing certificate and install independently. The direct Android preview instead uses an independent retained preview certificate and `.preview` ID; no FileInfra key import or production/channel continuity is claimed. [Signing guidance](distribution/signing.md) separates certificates, aliases, Play/F-Droid channels and Apple/Windows/Linux identities.
