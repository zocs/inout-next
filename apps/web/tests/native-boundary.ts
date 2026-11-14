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
  const diagnostic = { enabled: false, running: false, bufferKib: 128, defaultBufferKib: 128, probeState: "idle", result: null as null | {bytes:number;durationMs:number;mibPerSecond:number;bufferKib:number;connections:number} };
  let applyReply: (() => void) | undefined;
  const statusReplies: (() => void)[] = [];
  const boundary = { state, calls, holdBufferApply: false, holdDiagnosticStatus: false, releaseApply: () => applyReply?.(), releaseStatus: () => { for (const reply of statusReplies.splice(0)) reply(); }, probeOutcome: "success" as "success" | "error" | "pending" };
  let rejectProbe: ((error: Error) => void) | undefined;
  await page.exposeBinding("fixtureNativeInvoke", async (_, command: string, args?: { options?: StartOptions; enabled?: boolean; kib?: number; path?: string; connections?: number }) => {
    calls.push({ command, args });
    switch (command) {
      case "diagnostic_status": {
        const snapshot = structuredClone(diagnostic);
        if (boundary.holdDiagnosticStatus) await new Promise<void>((resolve) => statusReplies.push(resolve));
        return snapshot;
      }
      case "set_diagnostic_mode":
        if (args?.enabled && !state.running) throw new Error("Start sharing first");
        diagnostic.enabled = !!args?.enabled;
        if (!diagnostic.enabled) { diagnostic.bufferKib = 128; diagnostic.result = null; rejectProbe?.(new Error("Local test canceled")); }
        return structuredClone(diagnostic);
      case "set_download_buffer":
        if (!diagnostic.enabled || diagnostic.running || !args?.kib || args.kib < 16 || args.kib > 128 || args.kib % 16) throw new Error("Invalid diagnostic buffer");
        if (boundary.holdBufferApply) await new Promise<void>((resolve) => { applyReply = resolve; });
        diagnostic.bufferKib = args.kib;
        return structuredClone(diagnostic);
      case "reset_diagnostic_settings":
        if (!diagnostic.enabled || diagnostic.running) throw new Error("Diagnostic settings are unavailable");
        diagnostic.bufferKib = 128;
        return structuredClone(diagnostic);
      case "cancel_diagnostic_test":
        rejectProbe?.(new Error("Local test canceled"));
        return structuredClone(diagnostic);
      case "run_diagnostic_test": {
        if (!diagnostic.enabled || diagnostic.running || !args?.path || ![1,2,4,8].includes(args.connections ?? 0)) throw new Error("Invalid diagnostic test");
        diagnostic.running = true; diagnostic.probeState = "running"; diagnostic.result = null;
        try {
          if (boundary.probeOutcome === "pending") await new Promise((_,reject) => { rejectProbe = reject; });
          if (boundary.probeOutcome === "error") throw new Error("Shared file is unavailable");
          diagnostic.result = { bytes: 10485760, durationMs: 1000, mibPerSecond: 10, bufferKib: diagnostic.bufferKib, connections: args.connections! };
          return structuredClone(diagnostic.result);
        } finally { diagnostic.running = false; diagnostic.probeState = "idle"; rejectProbe = undefined; }
      }
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
        diagnostic.enabled = false; diagnostic.bufferKib = 128; diagnostic.result = null; rejectProbe?.(new Error("Local test canceled"));
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
  return boundary;
}
