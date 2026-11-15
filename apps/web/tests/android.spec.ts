import { expect } from "@playwright/test";
import { generatedKey, installNativeBoundary } from "./native-boundary";
import { test } from "./service";

test.describe("Android private-storage host", () => {
  test.use({ serviceOptions: { writable: false, modules: "files" } });

  test("supported controls start LAN sharing without an unavailable picker or local manager", async ({ page, service }) => {
    const boundary = await installNativeBoundary(page, service.url, { privateStorage: true });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("http://tauri.localhost");
    await expect(page.getByRole("button", { name: "Choose folder", exact: true })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "App storage", exact: true })).toBeVisible();
    await expect(page.getByRole("note")).toContainText("background");
    await expect(page.getByLabel("Share on the local network", { exact: true })).toBeChecked();
    await page.getByLabel("Allow uploads", { exact: true }).check();
    await page.getByLabel("Allow file management", { exact: true }).check();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeDisabled();
    await page.getByRole("button", { name: "Generate access key", exact: true }).click();
    await page.getByRole("button", { name: "Start sharing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Stop sharing", exact: true })).toBeEnabled();
    await expect(page.getByRole("button", { name: "Open file manager", exact: true })).toHaveCount(0);
    await expect(page.getByText("http://192.168.1.20:5000", { exact: true })).toBeVisible();
    expect(boundary.calls.find((call) => call.command === "start_service")?.args).toEqual({ options: {
      network: true, port: 5000, uploads: true, manage: true, key: generatedKey,
    } });
    await page.getByRole("button", { name: "Stop sharing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeEnabled();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeFocused();
    for (const width of [320, 390, 760]) {
      await page.setViewportSize({ width, height: 844 });
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      for (const button of await page.getByRole("button").all()) {
        if (await button.isVisible()) expect((await button.boundingBox())!.height).toBeGreaterThanOrEqual(48);
      }
    }
    expect(boundary.calls.some((call) => call.command === "choose_folder" || call.command === "open_manager")).toBe(false);
  });

  test("keyboard retry focuses the available Start control for private storage", async ({ page, service }) => {
    await installNativeBoundary(page, service.url, { privateStorage: true, initialStatusError: "Native status temporarily unavailable" });
    await page.goto("http://tauri.localhost");
    const retry = page.getByRole("button", { name: "Try again", exact: true });
    await expect(retry).toBeVisible();
    await retry.focus();
    await retry.press("Enter");
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeEnabled();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeFocused();
  });
});
