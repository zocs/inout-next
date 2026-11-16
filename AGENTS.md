# Working on inout-next

This is an independent Rust file service, Next.js static HTTP manager and opt-in Slint native desktop trial. Existing Tauri desktop/Android packages are transitional previews. Slint is evaluated first, with egui as the fallback. Use FileInfra and the archived inout-tauri as experience and protocol references. The new implementation has no dufs dependency or copied dufs server.

## Boundaries

- Keep product branding in `product.json` and stable native installation values in `config/native-identity.json`. Internal package/module identifiers stay independent of the displayed name. Display-name changes must preserve native package/resource/install/upgrade identifiers and must not silently change application IDs or signing identity.
- Keep filesystem operations behind the storage interface; HTTP modules and the UI must not open host paths directly.
- Register feature modules through their manifests/interfaces. A disabled module must have no active API routes and must not appear as an available UI action.
- Server-side authorization, permissions, root containment, safe publication and transfer limits are required for every applicable API. UI visibility is not authorization.
- Prefer small modules with explicit contracts and real integration tests. Record platform limits; do not label a cross-compilation or source review as target-device validation.
- Keep credentials, binaries, caches and screenshots out of git. Do not inspect images/screenshots; verify the web UI through actual interaction, accessibility and DOM/layout inspection.

## Documentation after every major update

Update affected public product documents before completion: `README.md`, `README_zh.md`, `docs/requirements.md`, `ROADMAP.md`, `docs/architecture.md`, `docs/branding.md`, `docs/modules.md`, `docs/platforms.md`, `CHANGELOG.md` and distribution/design guidance. Record what actually builds/runs and distinguish target compilation from installation/device/signing clearance.

Keep handoff, execution plans, full review reports, detailed validation/debug evidence and design-tool sidecars local under their ignored paths. Preserve original records locally; never force-add them or embed their machine paths, process/session details, raw logs or internal chronology into public documents. Public documentation contains usage, contracts, architecture, current limitations, roadmap and concise measured facts.

Update `docs/handoff.md` through targeted patches only. Never replace it with shell redirection, sed, awk or Python whole-file writes. Include current commits, remaining tasks, exact verification results and known limitations so another session can continue without repeating completed work.
