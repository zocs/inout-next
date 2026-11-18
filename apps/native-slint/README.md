# Native desktop trial

This opt-in Slint client controls the real inout Rust service. It uses winit, prefers FemtoVG and enables a Windows software-renderer fallback when a suitable OpenGL driver is unavailable. It does not embed a WebView. Next.js remains the static HTTP manager opened by connecting browsers. The existing Tauri desktop/Android releases retain their separate preview status.

Current trial artifacts come from [serial CI at d42048d](https://github.com/zocs/inout-next/actions/runs/44239092778), with locked tests, strict Clippy and optimized builds passing on Linux x64, Windows x86/x64 and macOS ARM64/x64. They include the Windows software-fallback and value-write fixes; both packaged Windows executables pass startup, three-language/theme retention and normal close without starting sharing. On the run page, download `inout-next-<target>` and select its `inout-next-0.1.0-<target>-native-slint-trial.zip`. Compare its digest with the accompanying `.zip.sha256`, extract the complete ZIP and run `share-native-slint` (`share-native-slint.exe` on Windows). Keep `web/` beside the executable. CLI ZIPs and Tauri installers in the same artifacts have different compositions. [Current target measurements](../../docs/distribution/size-and-runtime.md#windows-fallback-and-input-ci-packages) preserve the preceding3d0e656 checkpoint separately. Installed/offline, signing and adoption gates remain open; current local Windows file/transfer operation is recorded separately below.

Choose or enter a folder, choose local/LAN access, port and upload/management permissions, and start sharing. The access key is optional. Public uploads and general management can be explicitly enabled; published deletion requires configured-key authentication. The connection panel displays the effective running settings and actual addresses. Choose an address to open it in an external browser or copy it. A failed browser handoff shows a recoverable message so the address can be copied and opened manually.

Stop and closing a running host require confirmation. Keep sharing cancels the request without replacing the listener or an admitted transfer. Confirmed interruption shuts down owned work. Transfer receipts cover the current sharing run and reset at the next successful Start. Sent download bytes do not establish saving on another device. Durable resume/history, individual cancellation, native safe-open and live-setting editors remain later work.

English, Simplified Chinese and Traditional Chinese are local typed catalogs. The initial language follows supported system language preferences. Four themes recolor the authored paper/ink/connection surfaces; standard Fluent controls retain Slint's platform accent and focus behavior. Language/theme changes preserve drafts and sharing. Preferences are temporary; set `INOUT_NATIVE_REDUCE_MOTION=1` before launch to disable startup motion. Native OS motion-preference detection and settings persistence are pending. The bounded startup exit follows host readiness and does not delay ready controls.

Language and theme controls handle native accessibility value writes explicitly, accepting only their current model values while enabled. Language updates the existing presentation catalog; theme keeps its index binding. Pointer/keyboard selection retains the standard callback. These actions do not recreate the service or window. Standard option selection, scrolling, screen readers and CJK composing input have separate acceptance gates.

Linux keyboard and semantic interaction are tested separately from assistive acceptance. The locked AccessKit AT-SPI bridge lacks editable-text operations and reports disabled buttons as enabled. Application button guards prevent those disabled actions from changing sharing, but the bridge's state reporting and full screen-reader/CJK input behavior remain adoption gates. This trial does not claim complete accessibility support.

For source builds, install Rust 1.99.1, Node for the static export, and the native prerequisites for the target OS, then run from the repository:

```sh
npm ci --prefix apps/web
npm --prefix apps/web run build
cargo test --manifest-path apps/native-slint/Cargo.toml --locked
cargo build --manifest-path apps/native-slint/Cargo.toml --release --locked
python3 scripts/package_native_slint.py --binary apps/native-slint/target/release/share-native-slint --target linux-x64 --output .local/packages
```

When using an explicit `CARGO_TARGET_DIR` or `--target`, pass that actual output binary to the packaging script. Extract the complete ZIP and run `share-native-slint` (`share-native-slint.exe` on Windows). Keep `web/` beside it; release builds have no repository/current-directory fallback. Build tools are not runtime dependencies.

Linux requires a graphical X11 or Wayland session, compatible OpenGL graphics, fontconfig/FreeType, xkbcommon, the applicable display libraries and desktop file-dialog/clipboard services. A system browser handles Open in browser. An archive of the executable/resources does not bundle those OS libraries or establish Ubuntu 18.04/fully offline compatibility. macOS signing/sandboxing and Windows installed/offline behavior have separate gates.

Windows keeps the preferred GPU path and uses Slint's existing fallback factories when OpenGL initialization fails. This handles basic-display and virtual-machine hosts without changing graphics drivers. Linux/macOS renderer features are unchanged. Software-renderer startup and input checks do not establish presented-frame rate, animation smoothness or energy use.

Both source-built Windows x86/x64 programs pass14 native tests, strict Clippy and optimized builds. Actual Windows10x64 checks cover startup, three-language/theme readback, invalid/disabled writes, CJK-root file operations and the same admitted2MiB upload through presentation changes and canceled Stop/close, followed by exact disk/download hashes and normal cleanup. [Windows evaluation](../../docs/native-gui.md#windows-default-renderer-and-input) distinguishes this from real32bit OS, standard option/IME/screen-reader, final offline/installed and presentation acceptance.

From a Windows source checkout, the public regression checks a built or extracted executable without starting sharing:

```powershell
./scripts/verify_native_slint_windows.ps1 -BinaryPath path/to/share-native-slint.exe -ExpectedTitle inout -ReportPath .local/verification/native-startup.json -VerifyChoices
```

Use a new report path for each invocation so an original failure is preserved. The CI invokes the same default-renderer, language/theme and normal-close checks on its packaged executable and retains failure evidence separately.

The Ubuntu22.04 CI executable directly imports GLIBC through 2.35; the separate local functional build imports through 2.43. These direct-symbol ceilings do not establish the minimum versions of all transitive dependencies or successful operation on an older system.

The application name/description/version come from `product.json` at compilation; rebranding requires rebuilding. Native installation/signing identity remains in `config/native-identity.json`. The fixed internal executable/crate names are independent of display branding.

Open the top-level About menu or button for the native trial information and the standard AboutSlint attribution. The application source remains MIT; Slint's distribution terms are separate in [THIRD_PARTY.md](THIRD_PARTY.md) and the included [framework licence](LICENSES/Slint-Royalty-free-2.0.md). F-Droid, Apple channels and other store acceptance are not established by this desktop trial.

Pinned Slint Android uses native-activity and Skia with an API26 minimum. Its platform helper forwards IME composition and Back, but application/device input and accessibility remain unvalidated. Arm64/16 KiB, persistent Service ownership, SAF and iOS integration have separate milestones. The existing Android preview's API24 minimum and signing identity are unchanged. Slint is tried first; egui remains the fallback if required performance/platform/distribution gates cannot be met.
