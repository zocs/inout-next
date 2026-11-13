# Inout Next product context

Inout Next lets one device share a directory and lets other devices manage files through a browser. Transfers stay on the selected host/network. The project serves offline LANs, personal devices and industrial environments where installing a client on every receiving device is impractical.

The product is an independent HTTP file service with a modern web interface. The common runtime is Rust; Next.js builds static UI assets. Tauri adds a native application shell. Android needs native service and storage adapters.

Primary tasks are browsing, uploading, downloading, creating folders, renaming, moving, copying files and deleting. Show actual permissions, retain useful error state, and distinguish sent upload bytes from confirmed publication. Names and module availability come from configuration/server manifests.

Windows, Linux, macOS and Android are product targets. Initial server/browser operation and native desktop construction are the first milestones; platform readiness is recorded per artifact and test rather than inferred from framework support.
