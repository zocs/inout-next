# inout product context

inout lets one device share a chosen desktop directory or its Android app storage and lets other devices manage files through a browser. Transfers stay on the selected host/network. The project serves offline LANs, personal devices and industrial environments where installing a client on every receiving device is impractical. The repository and distribution slug remain `inout-next`; display branding is independently configurable.

The product is an independent HTTP file service with a modern web interface. The common runtime and renderer-independent host/session are Rust; Next.js builds the static HTTP manager. An opt-in Slint desktop trial renders the application without WebView, with egui retained as the fallback. Existing Tauri desktop/Android packages are transitional previews. The Android foreground preview uses app-private storage and stops sharing when backgrounded; native foreground Service ownership and SAF storage adapters remain milestones.

The native host has its own design direction. The browser manager's appearance is an aesthetic reference; native layout, controls and motion can evolve around the sharing workflow and platform conventions. Configurable pixel branding, readable state, local languages and transfer safety remain product requirements.

Primary tasks are browsing, uploading, downloading, creating folders, renaming, moving, copying files and deleting. Show actual permissions, retain useful error state, and distinguish sent upload bytes from confirmed publication. Names and module availability come from configuration/server manifests.

Windows, Linux, macOS and Android are product targets, with iOS adaptation scheduled later. Experimental desktop packages and an Android foreground preview have separate validation scopes; platform readiness is recorded per artifact and test rather than inferred from framework support. See the [platform matrix](docs/platforms.md).

The Slint trial builds and passes locked tests on Linux x64, Windows x86/x64 and macOS ARM64/x64 in [serial CI at 3d0e656](https://github.com/zocs/inout-next/actions/runs/43844556784). Its inspected native ZIPs are trial artifacts; actual local Linux interaction, target-device operation, mobile integration and published previews retain distinct scopes. Slint is not adopted solely on build results.
