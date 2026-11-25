# inout product context

inout lets one device share a chosen desktop directory or its Android app storage and lets other devices manage files through a browser. Transfers stay on the selected host/network. The project serves offline LANs, personal devices and industrial environments where installing a client on every receiving device is impractical. The repository and distribution slug remain `inout-next`; display branding is independently configurable.

The product is an independent HTTP file service with a modern web interface. The common runtime and renderer-independent host/session are Rust; Next.js builds the static HTTP manager. An opt-in Slint desktop trial renders the application without WebView, with egui retained as the fallback. Existing Tauri desktop/Android packages are transitional previews. The Android foreground preview uses app-private storage and stops sharing when backgrounded; native foreground Service ownership and SAF storage adapters remain milestones.

The native host has its own design direction. The browser manager's appearance is an aesthetic reference; native layout, controls and motion can evolve around the sharing workflow and platform conventions. Configurable pixel branding, readable state, local languages and transfer safety remain product requirements.

The native desktop source keeps GPU rendering preferred and adds a Windows software fallback for hosts without suitable OpenGL. Validated language/theme accessibility writes reuse presentation state and the same service session. Full assistive-device operation, displayed-frame performance and installed/offline behavior retain separate gates.

Primary tasks are browsing, uploading, downloading, creating folders, renaming, moving, copying files and deleting. Show actual permissions, retain useful error state, and distinguish sent upload bytes from confirmed publication. Names and module availability come from configuration/server manifests.

Current unreleased source adds a labelled current-permissions block and bounded file previews to the browser manager. The Slint host offers common-folder shortcuts, three access presets, labelled connections, URL-only QR codes, four accents with system/light/dark appearance and per-user setup preferences that exclude the key. Release GUI builds embed the static browser export, so sharing a home folder or a folder containing the application does not depend on a separate resource directory. Delayed browser operations retain only their own form/selection authority.

Windows, Linux, macOS and Android are product targets, with iOS adaptation scheduled later. Experimental desktop packages and an Android foreground preview have separate validation scopes; platform readiness is recorded per artifact and test rather than inferred from framework support. See the [platform matrix](docs/platforms.md).

The earlier Slint trial passed locked tests on Linux x64, Windows x86/x64 and macOS ARM64/x64 in [serial CI at d42048d](https://github.com/zocs/inout-next/actions/runs/44239092778). Its inspected ZIPs and packaged Windows startup checks precede the current control redesign and embedded export. Native target/device input, masked-key clipboard handling, installed/offline behavior and presented-frame performance remain separate from local Linux checks and published previews. Slint is not adopted solely on build results.

The [technology direction](ROADMAP.md#technology-direction) keeps the Rust service stack and plans a Vite + React browser build with the existing component/module contracts. Next.js remains the current export tool; performance and size claims follow actual target/package measurements.
