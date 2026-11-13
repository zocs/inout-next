import { expect } from "@playwright/test";
import { test } from "./service";

test.describe("native host entry", () => {
  test.use({ serviceOptions: { writable: false, modules: "files" } });

  test("native controls use the folder dialog and explicit service options", async ({ page, service }) => {
    // The operating-system dialog and IPC are unavailable in Chromium. Keep the
    // exported app and asset server real, and replace only that native boundary.
    await page.addInitScript(({ managerUrl }) => {
      const state = { root: null as string | null, running: false, managerUrl: null as string | null, addresses: [] as string[] };
      const calls: { command: string; args: unknown }[] = [];
      Object.assign(window, {
        nativeCalls: calls,
        __TAURI_INTERNALS__: {
          invoke: async (command: string, args?: { options?: unknown }) => {
            calls.push({ command, args });
            switch (command) {
              case "host_status": return { ...state };
              case "choose_folder":
                if (args && Object.keys(args).length) throw new Error("Folder selection accepts no path from JavaScript");
                state.root = "/folder-from-native-dialog"; return { ...state };
              case "create_access_key": return "fixture-native-key-64-characters-for-browser-interface-verification";
              case "start_service":
                state.running = true; state.managerUrl = managerUrl; state.addresses = ["http://192.168.1.20:5000"]; return { ...state };
              case "stop_service": state.running = false; state.managerUrl = null; state.addresses = []; return { ...state };
              case "open_manager": return null;
              default: throw new Error("Unexpected native command: " + command);
            }
          },
        },
      });
    }, { managerUrl: service.url });
    await page.route("http://tauri.localhost/**", async (route) => {
      const original = new URL(route.request().url());
      const response = await route.fetch({ url: service.url + original.pathname + original.search });
      await route.fulfill({ response });
    });
    await page.goto("http://tauri.localhost");
    await expect(page.getByRole("heading", { name: "Share a folder", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeDisabled();
    await page.getByRole("button", { name: "Choose folder", exact: true }).click();
    await expect(page.getByText("/folder-from-native-dialog", { exact: true })).toBeVisible();
    await page.getByLabel("Share on the local network", { exact: true }).check();
    await page.getByLabel("Allow uploads", { exact: true }).check();
    await page.getByLabel("Allow file management", { exact: true }).check();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeDisabled();
    await page.getByRole("button", { name: "Generate access key", exact: true }).click();
    await page.getByRole("button", { name: "Start sharing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Stop sharing", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Choose folder", exact: true })).toBeDisabled();
    await expect(page.getByText("http://192.168.1.20:5000", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Open file manager", exact: true }).click();
    const calls = await page.evaluate(() => (window as unknown as { nativeCalls: { command: string; args: unknown }[] }).nativeCalls);
    expect(calls.find((call) => call.command === "start_service")?.args).toEqual({ options: {
      network: true, port: 5000, uploads: true, manage: true,
      key: "fixture-native-key-64-characters-for-browser-interface-verification",
    } });
    expect(calls.filter((call) => call.command === "open_manager")).toHaveLength(1);
    await page.getByRole("button", { name: "Stop sharing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeVisible();
    await expect(page.getByText("http://192.168.1.20:5000", { exact: true })).toHaveCount(0);
    await page.setViewportSize({ width: 390, height: 844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });

  test("loopback file manager stays a file manager when Tauri IPC is injected", async ({ page, service }) => {
    await page.addInitScript(() => {
      Object.assign(window, { __TAURI_INTERNALS__: { invoke: () => { throw new Error("The file webview must not use native controls"); } } });
    });
    await page.goto(service.url);
    await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Choose folder", exact: true })).toHaveCount(0);
  });
});
