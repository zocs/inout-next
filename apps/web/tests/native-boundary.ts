import type { Page } from "@playwright/test";

interface StartOptions {
  network: boolean;
  port: number;
  uploads: boolean;
  manage: boolean;
  key: string | null;
}

interface Status {
  root: string | null;
  running: boolean;
  managerUrl: string | null;
  addresses: string[];
  activeSettings: {
    network: boolean;
    port: number;
    uploads: boolean;
    manage: boolean;
    authRequired: boolean;
  } | null;
}

export const generatedKey = "fixture-native-key-64-characters-for-browser-interface-verification";

// Chromium cannot invoke a native dialog or Tauri IPC. Only this boundary is
// replaced; the exported app and its Rust static server remain real. Keeping
// state outside the document models a service that survives a page reload.
export async function installNativeBoundary(page: Page, managerUrl: string) {
  const state: Status = { root: null, running: false, managerUrl: null, addresses: [], activeSettings: null };
  const calls: { command: string; args: unknown }[] = [];
  await page.exposeBinding("fixtureNativeInvoke", async (_, command: string, args?: { options?: StartOptions }) => {
    calls.push({ command, args });
    switch (command) {
      case "host_status": break;
      case "choose_folder":
        if (args && Object.keys(args).length) throw new Error("Folder selection accepts no path from JavaScript");
        state.root = "/folder-from-native-dialog";
        break;
      case "create_access_key": return generatedKey;
      case "start_service": {
        const options = args?.options;
        if (!state.root || !options || state.running) throw new Error("Invalid native start transition");
        if ((options.uploads || options.manage) && !options.key?.trim()) throw new Error("An access key is required");
        const port = options.port === 0 ? 49321 : options.port;
        state.running = true;
        state.managerUrl = managerUrl;
        state.addresses = options.network ? [`http://192.168.1.20:${port}`] : [];
        state.activeSettings = { network: options.network, port, uploads: options.uploads,
          manage: options.manage, authRequired: options.key !== null };
        break;
      }
      case "stop_service":
        state.running = false;
        state.managerUrl = null;
        state.addresses = [];
        state.activeSettings = null;
        break;
      case "open_manager": return null;
      default: throw new Error("Unexpected native command: " + command);
    }
    return structuredClone(state);
  });
  await page.addInitScript(() => {
    const fixtureWindow = window as unknown as { fixtureNativeInvoke: (command: string, args?: unknown) => Promise<unknown> };
    Object.assign(window, { __TAURI_INTERNALS__: {
      invoke: (command: string, args?: unknown) => fixtureWindow.fixtureNativeInvoke(command, args),
    } });
  });
  await page.route("http://tauri.localhost/**", async (route) => {
    const original = new URL(route.request().url());
    const response = await route.fetch({ url: managerUrl + original.pathname + original.search });
    await route.fulfill({ response });
  });
  return { state, calls };
}
