# Third-party components

The application source uses the repository's [MIT licence](LICENSE). Dependencies retain their own licences; the application licence does not relicense them. Rust and JavaScript manifests and lockfiles identify the dependencies for each compiled composition.

The native Slint trial's distribution terms, attribution and the complete MIT notices for qrcodegen 1.8.0 and tempfile 3.27.0 are in [apps/native-slint/THIRD_PARTY.md](apps/native-slint/THIRD_PARTY.md). Native ZIPs include that document and the unmodified Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0. The About button exposes Slint attribution. System fonts are used without redistributing font assets.

The CLI, native Slint trial and transitional Tauri desktop/Android previews have separate build/runtime compositions. Node and Python are build/verification tools, rather than delivered Rust-service runtimes. A store or F-Droid candidate still needs the complete dependency, source-build and channel review described in [store readiness](docs/distribution/store-readiness.md); these notices do not establish store acceptance.
