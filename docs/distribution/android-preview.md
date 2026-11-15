# Android foreground preview

This guide covers the experimental inout Android arm64 preview, with package version `0.1.0`. It shares files in the app's own storage with a browser on another device. The [desktop preview](preview.md) remains a separate download. Android's native foreground Service, SAF folder grants, local file import/manager and store distribution are later milestones.

## Install and connect

1. Use an arm64 Android device. The declared minimum is Android 7 / API24; actual runtime validation currently uses API36/x86_64, so minimum-OS and physical arm64 operation remain unverified. Keep Android System WebView current.
2. Download the Android arm64 APK and `SHA256SUMS.txt` from the preview release. Verify its checksum, then open the APK; if Android asks, allow installation from the browser or file manager you used. This is direct APK distribution, outside Play/F-Droid.
3. Put the sharing phone and the other device on the same local network. Open **inout**; **App storage** identifies the files being shared.
4. Enable **Allow uploads** and/or **Allow file management** as needed, then generate or enter an access key. Writes require a key. Start sharing and open a displayed local-network address in the other device's browser; enter the key there.
5. Keep inout on screen while transferring. **Stop sharing**, Home or root Back closes sharing and cancels incomplete transfers. Returning leaves the service stopped until you choose **Start sharing** again.

The native app controls sharing; the other device's browser lists files, creates folders, uploads, downloads, copies files, renames/moves supported entries and deletes them. The current app-private provider supports file/directory moves and file copy; directory copying is unavailable. There is no native picker or import action for existing files elsewhere on the phone. HTTP access keys control access but do not encrypt files or credentials in transit.

Saved files remain after Stop, process restart and Activity recreation. Delete files through the remote manager when sharing is active. Clearing Android application data or uninstalling removes them. Fresh host-page/process creation resets in-memory access-setting drafts; configure a new session explicitly. The preview's data is separate from FileInfra and future production inout.

## Identity and package checks

| Item | Value |
| --- | --- |
| Installed display label | `inout` |
| Repository/distribution slug | `inout-next` |
| Preview application ID | `io.github.zocs.inoutnext.preview` |
| Production identity retained separately | `io.github.zocs.inoutnext` |
| Android version name / code | `0.1.0` / `100001` |
| Compile / target / minimum API | `36` / `36` / `24` |
| Distributed ABI | `arm64-v8a` |
| Runtime | Rust library, static Next export and Android System WebView |

The retained preview certificate's SHA-256 fingerprint is:

```text
12125fbf1e85dff4cecaab432fc4cef02ba4eb37e5e36f26f6625df94842e5a2
```

This certificate and `.preview` ID are independent of FileInfra/production/store signing. An APK checksum identifies a download; a certificate fingerprint identifies its signer. See [signing and channels](signing.md) for coexistence and upgrade requirements.

Each accepted APK is checked for the expected preview manifest/label/version/APIs, exact raw web names/lengths/SHA-256 values, one ABI, every native ELF LOAD segment and uncompressed native ZIP 16 KiB alignment. The manifest has `INTERNET` plus AndroidX's app-specific signature-protected internal receiver permission. No storage/media/location/foreground-service permission is requested by this scope. Package alignment is separate from execution on a 16 KiB device.

## Verification scope

The optimized signed arm64 validation candidate is 16,151,507 bytes (15.40 MiB), with 35 raw web files / 1,052,337 bytes. It includes the final Android shutdown correction and passes package/signature inspection. Final release size and checksums come from the accepted CI APK after direct preview signing; [size/runtime guidance](size-and-runtime.md) distinguishes local and delivered measurements.

Actual release-APK execution uses an owned API36/x86_64 emulator with 4 KiB pages, accessed through ADB forwarding. It verifies 26 HTTP checks and seven browser operation groups: authentication and origin policy, upload/download/Range hashes, file copy/rename/move, directory move/delete and runtime-resource separation. The final signed x86_64 candidate passes eight lifecycle groups covering Stop/restart, Home/Back, incomplete-upload cancellation and persisted-file SHA-256, plus true Activity destruction/cold launch without the earlier native renderer exit failure. Five immediate Back/return cycles are also verified. Accessibility checks cover font scale1.3, increased display density, landscape, system bars and the keyboard. Host checks include 32 native and 91 Chromium cases; the latter include two separate Android native-command fixtures.

These results establish that emulator/release-APK combination. Physical arm64, actual 16 KiB operation, two-device Wi-Fi, API24/other WebView combinations, installed upgrades, AAB and store acceptance remain unverified. The [platform matrix](../platforms.md), [roadmap](../../ROADMAP.md) and [store gates](store-readiness.md) track those milestones. iOS has no released artifact.

## Build from source

Use pinned Rust1.99.1, Node26.13.1, JDK17, Android API36/build-tools36.0.0 and NDK28.2.13676358. The committed text scaffold uses AGP8.13.1, Kotlin2.2.20 and Gradle8.14. Install the `aarch64-linux-android` Rust target and the corresponding SDK/NDK tools first. CI provisions Android command-line tools build13114758 and platform tools in an isolated SDK directory through a fixed-commit setup action before the locked target build.

```sh
npm ci --prefix apps/web
npm ci --prefix apps/desktop
python3 scripts/build_android.py --abi aarch64 --export build \
  --sdk /path/to/android-sdk --java-home /path/to/jdk17 \
  --fetch-wrapper --report .local/verification/android-build.json
```

The helper fetches only the checksum-pinned official Gradle wrapper when requested, creates the bounded web index, invokes real single-target locked Cargo children, preserves both lockfiles and inspects the unsigned APK. The report identifies the exact output and checks. Signing is a separate private operation. Build-tool tests use real temporary files/recording children/ZIP/ELF inputs; Kotlin assets tests use the production bounded-stream installer. A GitHub source build is separate from an actual F-Droid recipe and reproducible APK acceptance.

When reporting a test problem, include the release asset, Android/API and System WebView version, network type, sharing permissions, approximate file size and repeatable steps. Keep access keys and private filenames out of public reports.
