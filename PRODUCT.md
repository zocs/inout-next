# inout product context

inout lets one device share a chosen desktop directory or its Android app storage and lets other devices manage files through a browser. Transfers stay on the selected host/network. The project serves offline LANs, personal devices and industrial environments where installing a client on every receiving device is impractical. The repository and distribution slug remain `inout-next`; display branding is independently configurable.

The product is an independent HTTP file service with a modern web interface. The common runtime is Rust; Next.js builds static UI assets. Tauri adds desktop and Android native hosts. The Android foreground preview uses app-private storage and stops sharing when backgrounded; native foreground Service ownership and SAF storage adapters remain milestones.

Primary tasks are browsing, uploading, downloading, creating folders, renaming, moving, copying files and deleting. Show actual permissions, retain useful error state, and distinguish sent upload bytes from confirmed publication. Names and module availability come from configuration/server manifests.

Windows, Linux, macOS and Android are product targets, with iOS adaptation scheduled later. Experimental desktop packages and an Android foreground preview have separate validation scopes; platform readiness is recorded per artifact and test rather than inferred from framework support. See the [platform matrix](docs/platforms.md).
