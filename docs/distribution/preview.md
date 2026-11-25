# Desktop preview

[Download v0.1.0-preview.1](https://github.com/zocs/inout-next/releases/tag/v0.1.0-preview.1). This is a prerelease for testing, with package version 0.1.0. The five desktop targets were built and tested at [c004d68](https://github.com/zocs/inout-next/actions/runs/43649949553); the preview tag adds product documentation without changing the application. [Android v0.1.0-preview.2](https://github.com/zocs/inout-next/releases/tag/v0.1.0-preview.2) has a separate arm64 APK and [foreground-sharing scope](android-preview.md). iOS remains scheduled for later adaptation and device validation.

These released GUI packages use the transitional Tauri host. The current source also has a separate [Slint native desktop trial](../../apps/native-slint/README.md), with no embedded WebView. Its release executable embeds the validated static browser export; `native-slint-trial.zip` contains that executable, product metadata, usage documents and source/framework notices, without a duplicate `web/` directory. These packages are separate from the server ZIPs and published installers below. Slint is evaluated first, with egui as the fallback. This source trial does not replace preview.1/preview.2 or establish installed/offline, mobile, signing or store acceptance.

[Historical CI 44239092778 at d42048d](https://github.com/zocs/inout-next/actions/runs/44239092778) provides inspected Slint trial ZIPs for all five desktop targets after passing locked tests, strict Clippy and optimized builds. Those older ZIPs carry the exact `web/` export beside the executable and precede the current embedded-export composition. They include the Windows software-fallback and language/theme value-write fixes; both exact Windows packaged programs pass startup/choices/theme retention and normal close without starting sharing. [Local Windows source validation](../native-gui.md#windows-default-renderer-and-input) separately covers CJK-root files and original-upload continuity. Download that earlier trial from the run's platform artifact using the native guide; [its measurements](size-and-runtime.md#windows-fallback-and-input-ci-packages) retain the same package scope. The release links and installation instructions below still describe preview.1's Tauri packages. Trial artifacts have not replaced that release.

## Choose a GUI package

| System | Release asset | Runtime requirement |
| --- | --- | --- |
| Windows x64 | `inout-next-0.1.0-windows-x64-setup.exe` | WebView2; the installer can download its bootstrapper |
| Windows x86 | `inout-next-0.1.0-windows-x86-setup.exe` | x86 WebView2; 32-bit Windows OS operation remains unverified |
| Linux x64 | `inout-next-0.1.0-linux-x64.AppImage` | Modern Linux; built on Ubuntu 22.04, with a bundled webview library closure |
| Linux x64 | `inout-next-0.1.0-linux-x64.deb` | Declared system WebKitGTK 4.1/GTK3 dependencies |
| macOS Apple Silicon | `inout-next-0.1.0-macos-arm64.dmg` | Apple system webview; signing/notarization/device installation remain unverified |
| macOS Intel | `inout-next-0.1.0-macos-x64.dmg` | Apple system webview; signing/notarization/device installation remain unverified |

The application displays **inout**; stable OS installation and disk-image labels remain **Inout Next**. These packages contain a Tauri GUI. Files named `inout-next-0.1.0-<target>-server.zip` contain the separate CLI service and static web manager, rather than a portable desktop GUI.

Choose a folder in the desktop window. Enable local-network sharing to connect from another device, then use a displayed LAN address. Uploads and management require an access key; generate or enter one before enabling writes. Closing the manager stops sharing, and quitting the main window performs controlled shutdown.

## CLI and checksums

CLI ZIPs cover Linux x64, Windows x86/x64 and macOS ARM64/Intel. Extract the whole ZIP so `web/` remains beside the server, then follow its `QUICKSTART.txt`. Running the compiled service needs no Node or Python. The default listener is `127.0.0.1:5000`; select a LAN listener explicitly when sharing with another device.

`SHA256SUMS.txt` covers all released packages and `PREVIEW-README.txt`. Compute the downloaded file's digest with `sha256sum <filename>` on Linux, `shasum -a 256 <filename>` on macOS or `Get-FileHash <filename> -Algorithm SHA256` on Windows, then compare its corresponding manifest entry.

## Scope of this preview

All five target CI jobs, strict Clippy, browser file operations and the complete package resource/architecture inspection pass. Windows has earlier bounded native picker/manager/file/shutdown checks. Package extraction and runner execution remain separate from installation, offline operation, physical-device validation and signing/store acceptance.

This desktop preview does not include the new automatic port-fallback change, network-recommendation/QR flow or later Android composition. Android's app-private foreground APK is described in its separate guide; SAF, native foreground Service, local import/manager and iOS remain incomplete. Durable resume, directory copy, Windows move/rename, Ubuntu 18.04 and offline Windows GUI portability are separate milestones. See [platform readiness](../platforms.md) and [network planning](../network-selection.md).

For testing feedback, include the release asset, operating system, chosen sharing mode and the steps that caused the problem. Avoid publishing access keys or private filenames.
