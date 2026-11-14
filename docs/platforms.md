# Platform readiness

The project targets Windows, Linux, macOS and Android. Status distinguishes target compilation/tests, actual execution, installation, offline runtime, signing and store acceptance. The current0.1.0 is an experimental desktop/CLI preview.

| Target | Verified scope | Remaining work |
| --- | --- | --- |
| Linux x64 | Ubuntu22.04 core/native/browser checks; CLI ZIP, AppImage/deb, emitted identity and exported-resource/ELF inspection | Native GUI/device/installation and signing |
| Windows x64 | Core/native tests, strict Clippy, CLI ZIP/NSIS, emitted identity, native PE/GUI subsystem and resource checks; bounded Windows10x64 native picker/manager/writable file/shutdown operation | Installation/uninstall/upgrade, offline WebView2, signing; move/rename unsupported |
| Windows x86 | Actual `i686-pc-windows-msvc` core/native tests/builds, CLI ZIP/NSIS and PE/resource checks; x86 native picker/readonly manager/download/Range/stop/close on Windowsx64 | Actual32bit Windows OS, installation/offline/signing; move/rename unsupported |
| macOS ARM64/Intel | Native macOS15 host tests/strict Clippy, CLI ZIP/app/DMG builds and extracted resources/Mach-O CPU checks | Native GUI/device/installation, older-macOS, signing/notarization and store profile |
| Linux ARM64 | Planned target | Core/desktop artifacts and target runtime |
| Ubuntu18.04/glibc2.27 | Independent compatibility goal | Actual compatible headless build/runtime and a separate WebKitGTK GUI strategy |
| Android arm64/API24+ | Native-adapter milestone | Rust/JNI, SAF storage, foreground Service, ABI/16KB/package/device/signing checks |
| Android armv7/x86_64 | Later ABI coverage | Following validated arm64 adaptation |
| iOS/iPadOS | Later adapter milestone | Security-scoped storage, permitted lifecycle, signing and device validation |

## Runtime and packaging

CLI ZIPs contain the compiled server, static `web/`, product metadata, licence and usage instructions. They do not require Node or Python. Native packages additionally require a platform webview: WebKitGTK4.1/GTK3 on Linux and WebView2 on Windows. Current Windows NSIS uses a downloadable WebView2 bootstrapper; it is not verified offline GUI distribution. See [package size and runtime](distribution/size-and-runtime.md).

Current Ubuntu22.04 native executables import GLIBC through2.34. This rules out a glibc2.27 claim for those executables and does not audit the minimum versions of every bundled/system dependency. AppImage size and a successful extraction do not establish old-distribution runtime compatibility.

Tauri's [Windows installer guide](https://v2.tauri.app/distribute/windows-installer/#building-for-32-bit-or-arm) supports32bit `i686-pc-windows-msvc`. Executing that binary on Windowsx64 is distinct from testing a32bit Windows OS. Linux32bit is not a declared target. Modern browser/System WebView requirements remain relevant to the static Next export.

Windows upload/copy publication requires a filesystem supporting hard links; file/directory move and rename are disabled. Linux/Apple publication and moves require native exclusive rename support. Directory copying is unsupported. Android SAF URIs require a storage adapter with truthful provider capabilities.

## CI

The [workflow](../.github/workflows/ci.yml) runs Ubuntu22.04, Windows2022x86/x64 and native macOS15ARM64/Intel targets through one noncancelling queue and serial matrices. Tests execute on the runner host and each target is separately packaged. Resource/architecture/identity checks are separate from installed upgrades and device acceptance. The workflow does not declare Android, LinuxARM64, Ubuntu18.04 or offline Windows GUI artifacts.

## Store channels

The [official-source requirements checked2026-11-14](distribution/store-readiness.md) and [privacy inventory](distribution/privacy-data-inventory.md) define early preparation. All channels still need actual candidate validation.

| Channel | Required platform evidence | Current boundary |
| --- | --- | --- |
| Google Play | Current targetAPI36 separately from minimumAPI24 goal; intended ABIs/16KB/AAB/device checks, SAF, correct FGS/local-network adaptation, signing/forms/testing | No Android candidate or store result |
| F-Droid | Free-software dependency/assets/tool inventory; actual clean-source Next/Rust/Gradle recipe, static version codes/tags and reproducible-signing attempt | No fdroiddata build/MR or APK verification |
| iOS/iPadOS App Store | Current Apple SDK/deployment floor, scoped documents, permitted foreground/finite sharing, IPv6/privacy/network grants, signed archive/TestFlight/device checks | Later native adaptation; no indefinite-background-server claim |
| Mac App Store | Independent sandbox/signing/provision profile, directory/bookmark scope, network grants, signed app/pkg and disclosures | Separate from direct-distribution DMGs; no store candidate |

SDK/policy requirements are rechecked before submission. Build/runtime readiness, signing continuity and store acceptance remain distinct.
