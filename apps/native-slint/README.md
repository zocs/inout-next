# Native desktop trial

This opt-in Slint client controls the real inout Rust service. It uses winit, prefers FemtoVG and enables a Windows software-renderer fallback when a suitable OpenGL driver is unavailable. It does not embed a WebView. Next.js remains the static HTTP manager opened by connecting browsers. The existing Tauri desktop/Android releases retain their separate preview status.

The last accepted cross-target trial artifacts come from [serial CI at d42048d](https://github.com/zocs/inout-next/actions/runs/44239092778), with locked tests, strict Clippy and optimized builds passing on Linux x64, Windows x86/x64 and macOS ARM64/x64. Those artifacts include the Windows software-fallback/value-write fixes and predate the authored controls, appearance, QR, preferences and embedded-resource changes. On that run page, download `inout-next-<target>` and select its `inout-next-0.1.0-<target>-native-slint-trial.zip`. Compare its digest with the accompanying `.zip.sha256`, extract the complete ZIP and run `share-native-slint` (`share-native-slint.exe` on Windows); those historical archives require their included `web/`. New source-built native ZIPs embed the browser export and need no separate resource directory. CLI ZIPs and Tauri installers retain their own compositions. [Target measurements](../../docs/distribution/size-and-runtime.md#windows-fallback-and-input-ci-packages) preserve each checkpoint separately. Installed/offline, signing and adoption gates remain open.

[CI at 78457fd](https://github.com/zocs/inout-next/actions/runs/45783944321) passes the redesigned Linux trial's22 native tests and produces an inspected seven-member ZIP with embedded resources. Android also passes its separate Tauri build/package checks; Windows x86 fails concurrent preference saving, Windows x64 is cancelled and macOS skipped. Current source closes the fully written preferences stage before atomic replacement. Windows verification of that repair is pending; [current package measurements](../../docs/distribution/size-and-runtime.md#embedded-source-ci-packages) retain the exact partial CI scope and do not add GUI-runtime acceptance.

Choose or enter a folder, including a leading `~`, or use an existing Home/Desktop/Documents/Downloads shortcut. Select one of three access presets: view/download, also upload, or also manage. The native manage preset includes uploads; the Rust host API keeps its independent flags. Network and port options are in a disclosure. The optional masked key has a Show/Hide control; published deletion requires configured-key authentication. Startup errors distinguish folder, application-resource, key and port problems.

The connection panel shows effective settings, a labelled current-permissions group, the selected connection and its LAN URL QR code. Linux/Android classification uses system facts; other platforms may show unknown network labels. Select another connection, copy the address with Copy or select text in the read-only address field, and open it in an external browser. The QR contains the URL only, never the key. Loopback explains local-only access. A failed browser handoff retains a recoverable message and the address.

Stop and closing a running host require confirmation. Keep sharing cancels the request without replacing the listener or an admitted transfer. Confirmed interruption shuts down owned work. Transfer receipts cover the current sharing run and reset at the next successful Start. Sent download bytes do not establish saving on another device. Durable resume/history, individual cancellation, native safe-open and live-setting editors remain later work.

English, Simplified Chinese and Traditional Chinese are local typed catalogs. The initial language follows supported system language preferences. Four accents have light/dark tokens and a system appearance option; authored fields, buttons and choices use the same semantic tokens and focus states. Scrollbars, context menus and AboutSlint retain toolkit behavior. Language/accent/appearance changes preserve drafts and sharing. Set `INOUT_NATIVE_REDUCE_MOTION=1` before launch to disable startup motion. Native OS motion-preference detection remains pending. The bounded startup exit follows host readiness and does not delay ready controls.

Validated language, accent, appearance, folder, port, network and access drafts are saved atomically to `<configuration-directory>/<application_id>/native-preferences.conf`. The configuration directory is `XDG_CONFIG_HOME` or `~/.config` on Linux, `%APPDATA%` on Windows and `~/Library/Application Support` on macOS. Ordinary window close saves drafts even before starting sharing. Concurrent app instances replace complete documents with last-writer-wins semantics. The access key, network addresses and transfer history are never saved. `INOUT_NATIVE_PREFERENCES=<path>` selects another file; an empty value disables saving. Delete that file to reset stored preferences.

Authored language, accent and appearance Select controls handle native accessibility value writes explicitly, accepting only their current model values while enabled. Language updates the existing presentation catalog; accent/appearance keep their index bindings. These actions do not recreate the service or window. A local Linux X11 run verifies all three languages, four accents, system/light/dark choices, keyboard traversal and access presets, About dismissal, a 780×600 viewport, invalid-port rejection and Rime composition of a Chinese folder name. Ordinary close/restart restores unstarted drafts without the key. A relocated executable shares its own enclosing directory and serves every embedded resource; the same 2 MiB upload survives presentation changes and canceled Stop/normal-close, then publishes and downloads with matching hashes. Confirmed Stop removes unfinished staging and releases the listener. The system picker cancels correctly; selection remains unverified in that local GTK fixture.

Linux keyboard and semantic interaction are tested separately from assistive acceptance. The locked AccessKit AT-SPI bridge lacks editable-text operations and reports disabled buttons as enabled. It also omits Enabled/Sensitive states for the read-only address, whose physical selection and copy are verified. Application button guards prevent disabled actions from changing sharing. Full screen-reader, font/IME and target-device behavior remain adoption gates.

Known masked-key limitation: in the verified Linux X11 build, Ctrl+C, Ctrl+X and selection into PRIMARY can place the plaintext key on the desktop clipboard while the field is masked. The custom context menu disables key copying, but Slint 1.18.1's input handling still permits these paths. Use this trial with that limitation in mind. A complete input-level correction must preserve selection and IME; final adoption remains gated on it. Windows/macOS clipboard behavior has not been verified by this run.

For source builds, install Rust 1.99.1, Node for the static export, and the native prerequisites for the target OS, then run from the repository:

```sh
npm ci --prefix apps/web
npm --prefix apps/web run build
cargo test --manifest-path apps/native-slint/Cargo.toml --locked
cargo build --manifest-path apps/native-slint/Cargo.toml --release --locked
python3 scripts/package_native_slint.py --binary apps/native-slint/target/release/share-native-slint --target linux-x64 --output .local/packages
```

When using an explicit `CARGO_TARGET_DIR` or `--target`, pass that actual output binary to the packaging script. A GUI release build requires a complete `apps/web/out` export with `index.html` and `_next`; an absent export fails the build. The bounded, symlink-free export is compiled into the executable. Extract the complete ZIP and run `share-native-slint` (`share-native-slint.exe` on Windows) from any working directory; a folder containing the package can be shared. Debug/headless builds may omit the export. Build tools are not runtime dependencies.

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

Open the header's About button for native trial information and the standard AboutSlint attribution. The application source remains MIT; Slint's distribution terms and qrcodegen/tempfile notices are in [THIRD_PARTY.md](THIRD_PARTY.md), with the included [framework licence](LICENSES/Slint-Royalty-free-2.0.md). F-Droid, Apple channels and other store acceptance are not established by this desktop trial.

Pinned Slint Android uses native-activity and Skia with an API26 minimum. Its platform helper forwards IME composition and Back, but application/device input and accessibility remain unvalidated. Arm64/16 KiB, persistent Service ownership, SAF and iOS integration have separate milestones. The existing Android preview's API24 minimum and signing identity are unchanged. Slint is tried first; egui remains the fallback if required performance/platform/distribution gates cannot be met.
