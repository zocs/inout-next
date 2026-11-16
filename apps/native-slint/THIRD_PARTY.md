# Native trial dependencies

Application source is licensed under the repository's MIT licence. This does not relicense dependencies.

The Slint 1.18.1 desktop trial uses the **Slint Royalty-free Desktop, Mobile, and Web Applications License, version 2.0** for Slint as part of the application. The unmodified licence is included in [LICENSES/Slint-Royalty-free-2.0.md](LICENSES/Slint-Royalty-free-2.0.md). The top-level About entry displays the standard `AboutSlint` widget as attribution. This choice applies to the desktop trial; it is not a F-Droid free-software or Apple store clearance claim.

Slint offers [GPL, royalty-free and commercial alternatives](https://github.com/slint-ui/slint/blob/v1.18.1/LICENSE.md). A future F-Droid/mobile/store composition must choose a compatible licence and satisfy the actual dependency/source-build requirements. The project's MIT source and stable app/signing identities are preserved.

Native composition pins Slint/slint-build 1.18.1, winit/FemtoVG/accessibility, rfd 0.17.2 (MIT), arboard 3.7.0 (MIT OR Apache-2.0) and sys-locale 0.3.2 (MIT OR Apache-2.0). Slint default features are disabled; Qt, WebView, Node, Python and the mobile Skia composition are not application runtimes in this desktop build. Rust/Tokio and the existing HTTP/storage modules retain their own dependency licences. System fonts are used rather than redistributed font files.

The dot-matrix glyphs reuse the project's local branding geometry. Slint's standard controls and About widget retain their own notices/attribution. This document describes this composition, not a complete store submission or dependency-source-build certification.
