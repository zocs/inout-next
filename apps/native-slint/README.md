# Native desktop trial

This opt-in Slint client controls the real inout Rust service. It renders through winit/FemtoVG and does not embed a WebView. Next.js remains the static HTTP manager opened by connecting browsers. The existing Tauri desktop/Android releases retain their separate preview status.

Choose or enter a folder, choose local/LAN access, port and upload/management permissions, and start sharing. The access key is optional. Public uploads and general management can be explicitly enabled; published deletion requires configured-key authentication. The connection panel displays the effective running settings and actual addresses. Choose an address to open it in an external browser or copy it. A failed browser handoff shows a recoverable message so the address can be copied and opened manually.

Stop and closing a running host require confirmation. Keep sharing cancels the request without replacing the listener or an admitted transfer. Confirmed interruption shuts down owned work. Transfer receipts cover the current sharing run and reset at the next successful Start. Sent download bytes do not establish saving on another device. Durable resume/history, individual cancellation, native safe-open and live-setting editors remain later work.

English, Simplified Chinese and Traditional Chinese are local typed catalogs. The initial language follows supported system language preferences. Four themes recolor the authored paper/ink/connection surfaces; standard Fluent controls retain Slint's platform accent and focus behavior. Language/theme changes preserve drafts and sharing. Preferences are temporary; set `INOUT_NATIVE_REDUCE_MOTION=1` before launch to disable startup motion. Native OS motion-preference detection and settings persistence are pending. The bounded startup exit follows host readiness and does not delay ready controls.

Linux keyboard and semantic interaction are tested separately from assistive acceptance. The locked AccessKit AT-SPI bridge lacks editable-text operations and reports disabled buttons as enabled. Application button guards prevent those disabled actions from changing sharing, but the bridge's state reporting and full screen-reader/CJK input behavior remain adoption gates. This trial does not claim complete accessibility support.

For source builds, install Rust 1.99.1, Node for the static export, and the native prerequisites for the target OS, then run from the repository:

```sh
npm ci --prefix apps/web
npm --prefix apps/web run build
cargo test --manifest-path apps/native-slint/Cargo.toml --no-default-features --locked
cargo build --manifest-path apps/native-slint/Cargo.toml --release --locked
python3 scripts/package_native_slint.py --binary apps/native-slint/target/release/share-native-slint --target linux-x64 --output .local/packages
```

When using an explicit `CARGO_TARGET_DIR` or `--target`, pass that actual output binary to the packaging script. Extract the complete ZIP and run `share-native-slint` (`share-native-slint.exe` on Windows). Keep `web/` beside it; release builds have no repository/current-directory fallback. Build tools are not runtime dependencies.

Linux requires a graphical X11 or Wayland session, compatible OpenGL graphics, fontconfig/FreeType, xkbcommon, the applicable display libraries and desktop file-dialog/clipboard services. A system browser handles Open in browser. An archive of the executable/resources does not bundle those OS libraries or establish Ubuntu 18.04/fully offline compatibility. macOS signing/sandboxing and Windows installed/offline behavior have separate gates.

The application name/description/version come from `product.json` at compilation; rebranding requires rebuilding. Native installation/signing identity remains in `config/native-identity.json`. The fixed internal executable/crate names are independent of display branding.

Open the top-level About menu or button for the native trial information and the standard AboutSlint attribution. The application source remains MIT; Slint's distribution terms are separate in [THIRD_PARTY.md](THIRD_PARTY.md) and the included [framework licence](LICENSES/Slint-Royalty-free-2.0.md). F-Droid, Apple channels and other store acceptance are not established by this desktop trial.

Android Slint API26/Skia, input/accessibility, 16 KiB/arm64, Service/SAF and iOS integration remain unvalidated. The existing Android preview's API24 minimum and signing identity are unchanged. Slint is tried first; egui remains the fallback if required performance/platform/distribution gates cannot be met.
