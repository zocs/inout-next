# Native trial dependencies

Application source is licensed under the repository's MIT licence. This does not relicense dependencies.

The Slint 1.18.1 desktop trial uses the **Slint Royalty-free Desktop, Mobile, and Web Applications License, version 2.0** for Slint as part of the application. The unmodified licence is included in [LICENSES/Slint-Royalty-free-2.0.md](LICENSES/Slint-Royalty-free-2.0.md). The header About button displays the standard `AboutSlint` widget as attribution. This choice applies to the desktop trial; it is not a F-Droid free-software or Apple store clearance claim.

Slint offers [GPL, royalty-free and commercial alternatives](https://github.com/slint-ui/slint/blob/v1.18.1/LICENSE.md). The included royalty-free2.0 terms also permit general-purpose mobile applications with the required attribution; mobile use alone does not require changing the app's MIT source licence. A future F-Droid/store composition must still choose compatible distribution terms and satisfy actual dependency/source-build and channel requirements. The project's MIT source and stable app/signing identities are preserved.

Native composition pins Slint/slint-build 1.18.1, winit/FemtoVG/accessibility and the Windows-only Slint software fallback, rfd 0.17.2 (MIT), arboard 3.7.0 (MIT OR Apache-2.0), sys-locale 0.3.2 (MIT OR Apache-2.0) and Windows-only windows-sys 0.61.2 (MIT OR Apache-2.0). Slint default features are disabled; Qt, WebView, Node, Python and the mobile Skia composition are not application runtimes in this desktop build. Rust/Tokio and the existing HTTP/storage modules retain their own dependency licences. System fonts are used rather than redistributed font files.

The dot-matrix glyphs reuse the project's local branding geometry. Slint's retained toolkit components and About widget keep their own notices/attribution. This document describes this composition, not a complete store submission or dependency-source-build certification.

## qrcodegen 1.8.0

The native QR generator uses [Project Nayuki's QR Code generator](https://www.nayuki.io/page/qr-code-generator-library), distributed under MIT. Its notice is reproduced from the locked crate's source:

```text
QR Code generator library (Rust)

Copyright (c) Project Nayuki. (MIT License)
https://www.nayuki.io/page/qr-code-generator-library

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:
- The above copyright notice and this permission notice shall be included in
  all copies or substantial portions of the Software.
- The Software is provided "as is", without warranty of any kind, express or
  implied, including but not limited to the warranties of merchantability,
  fitness for a particular purpose and noninfringement. In no event shall the
  authors or copyright holders be liable for any claim, damages or other
  liability, whether in an action of contract, tort or otherwise, arising from,
  out of or in connection with the Software or the use or other dealings in the
  Software.
```

## tempfile 3.27.0

Atomic preferences use tempfile, distributed under MIT OR Apache-2.0. This distribution includes its MIT notice:

```text
Copyright (c) 2015 Steven Allen

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

## windows-sys 0.61.2

Windows preference publication uses windows-sys for directory identity and kernel mutex APIs. The crate is distributed under MIT OR Apache-2.0. This distribution includes its MIT notice:

```text
    MIT License

    Copyright (c) Microsoft Corporation.

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE
```
