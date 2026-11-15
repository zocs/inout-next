import { expect, type Page } from "@playwright/test";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { generatedKey, installNativeBoundary } from "./native-boundary";
import { test } from "./service";

async function unlock(page: Page, url: string, key: string) {
  await page.goto(url);
  await page.getByLabel("Access key", { exact: true }).fill(key);
  await page.getByRole("button", { name: "Unlock files", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select welcome.txt", exact: true })).toBeVisible();
}

test("failed folder navigation hides stale actions and retry recovers from the real filesystem", async ({ page, service }) => {
  const folder = join(service.root, "Projects");
  mkdirSync(folder);
  await unlock(page, service.url, service.key);
  await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
  rmSync(folder, { recursive: true });
  const missing = page.waitForResponse((response) => new URL(response.url()).searchParams.get("path") === "Projects" && response.status() === 404);
  await page.getByRole("button", { name: "Open Projects", exact: true }).click();
  await missing;
  await expect(page.getByRole("heading", { name: "Projects", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Select welcome.txt", exact: true })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Download welcome.txt", exact: true })).toHaveCount(0);
  await expect(page.getByRole("region", { name: "Selected entry", exact: true })).toHaveCount(0);
  await expect(page.getByRole("region", { name: "Folder unavailable", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "New folder", exact: true })).toHaveCount(0);
  await expect(page.getByLabel("Upload files", { exact: true })).toHaveCount(0);

  mkdirSync(folder);
  writeFileSync(join(folder, "recovered.txt"), "The requested folder has returned.\n");
  await page.getByRole("button", { name: "Retry folder", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select recovered.txt", exact: true })).toBeVisible();
  await expect(page.getByRole("region", { name: "Folder unavailable", exact: true })).toHaveCount(0);
  expect(readFileSync(join(folder, "recovered.txt"), "utf8")).toBe("The requested folder has returned.\n");

  await page.getByRole("button", { name: "Select recovered.txt", exact: true }).click();
  rmSync(folder, { recursive: true });
  await page.getByRole("button", { name: "Refresh folder", exact: true }).click();
  await expect(page.getByRole("region", { name: "Folder unavailable", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Select recovered.txt", exact: true })).toHaveCount(0);
  await expect(page.getByRole("region", { name: "File actions", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Shared folder", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select welcome.txt", exact: true })).toBeVisible();
});

test("file and folder selection clearing restores keyboard focus and acknowledges state", async ({ page, service }) => {
  mkdirSync(join(service.root, "Projects"));
  await unlock(page, service.url, service.key);
  const file = page.getByRole("button", { name: "Select welcome.txt", exact: true });
  const folder = page.getByRole("button", { name: "Select Projects", exact: true });
  const openFolder = page.getByRole("button", { name: "Open Projects", exact: true });
  expect(await openFolder.getAttribute("aria-pressed")).toBeNull();
  await expect(file).toHaveAttribute("aria-pressed", "false");
  await expect(folder).toHaveAttribute("aria-pressed", "false");
  await file.focus();
  await page.keyboard.press("Enter");
  await expect(file).toBeFocused();
  await expect(file).toHaveAttribute("aria-pressed", "true");
  const selectedStatus = page.getByRole("status").filter({ hasText: /selected.*welcome\.txt/i });
  await expect(selectedStatus).toHaveAttribute("aria-live", "polite");
  const clear = page.getByRole("button", { name: "Clear selection", exact: true });
  await clear.focus();
  await page.keyboard.press("Enter");
  await expect(clear).toHaveCount(0);
  await expect(file).toBeFocused();
  await expect(file).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByRole("status").filter({ hasText: /selection cleared/i })).toHaveAttribute("aria-live", "polite");
  await folder.focus();
  await page.keyboard.press("Enter");
  await expect(folder).toHaveAttribute("aria-pressed", "true");
  await expect(file).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByRole("status").filter({ hasText: /selected.*Projects/i })).toHaveAttribute("aria-live", "polite");
  expect(await openFolder.getAttribute("aria-pressed")).toBeNull();
  await clear.focus();
  await page.keyboard.press("Enter");
  await expect(clear).toHaveCount(0);
  await expect(folder).toBeFocused();
  await expect(folder).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByRole("status").filter({ hasText: /selection cleared/i })).toHaveAttribute("aria-live", "polite");
});

test("selection clearing falls back to the current folder when its origin is filtered out", async ({ page, service }) => {
  mkdirSync(join(service.root, "Projects"));
  writeFileSync(join(service.root, "Projects", "note.txt"), "A filtered selection.\n");
  await unlock(page, service.url, service.key);
  await page.getByRole("button", { name: "Open Projects", exact: true }).click();
  const file = page.getByRole("button", { name: "Select note.txt", exact: true });
  await file.focus();
  await page.keyboard.press("Enter");
  await expect(file).toHaveAttribute("aria-pressed", "true");
  const search = page.getByRole("searchbox", { name: "Search this folder", exact: true });
  await search.fill("no matching entry");
  await expect(file).toHaveCount(0);
  const clear = page.getByRole("button", { name: "Clear selection", exact: true });
  await clear.focus();
  await page.keyboard.press("Enter");
  await expect(clear).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Projects", exact: true })).toBeFocused();
  await expect(page.getByRole("status").filter({ hasText: /selection cleared/i })).toHaveAttribute("aria-live", "polite");
  await search.fill("");
  await expect(file).toHaveAttribute("aria-pressed", "false");
});

test("passive selection reconciliation preserves focus during refresh and removal", async ({ page, service }) => {
  await unlock(page, service.url, service.key);
  const file = page.getByRole("button", { name: "Select welcome.txt", exact: true });
  await file.focus();
  await page.keyboard.press("Enter");
  const search = page.getByRole("searchbox", { name: "Search this folder", exact: true });
  const refresh = page.getByRole("button", { name: "Refresh folder", exact: true });
  await search.focus();
  const refreshed = page.waitForResponse((response) => new URL(response.url()).pathname === "/api/v1/entries" && response.status() === 200);
  await refresh.evaluate((button: HTMLButtonElement) => button.click());
  await refreshed;
  await expect(page.getByRole("table", { name: "Shared files", exact: true })).toHaveAttribute("aria-busy", "false");
  await expect(file).toHaveAttribute("aria-pressed", "true");
  await expect(search).toBeFocused();
  rmSync(join(service.root, "welcome.txt"));
  const reconciled = page.waitForResponse((response) => new URL(response.url()).pathname === "/api/v1/entries" && response.status() === 200);
  await refresh.evaluate((button: HTMLButtonElement) => button.click());
  await reconciled;
  await expect(file).toHaveCount(0);
  await expect(page.getByRole("region", { name: "Selected entry", exact: true })).toHaveCount(0);
  await expect(search).toBeFocused();
  await expect(page.getByRole("status").filter({ hasText: /selection cleared/i })).toHaveAttribute("aria-live", "polite");
});

test("management selection clearing preserves the new-folder input focus", async ({ page, service }) => {
  await unlock(page, service.url, service.key);
  const file = page.getByRole("button", { name: "Select welcome.txt", exact: true });
  await file.focus();
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: "New folder", exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("region", { name: "Selected entry", exact: true })).toHaveCount(0);
  await expect(file).toHaveAttribute("aria-pressed", "false");
  await expect(page.getByRole("textbox", { name: "Folder name", exact: true })).toBeFocused();
});

test("management follows the selected inspector before twelve confirmed mobile uploads", async ({ page, service }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await unlock(page, service.url, service.key);
  const uploads = Array.from({ length: 12 }, (_, index) => ({
    name: `confirmed-${index + 1}.txt`, mimeType: "text/plain", buffer: Buffer.from(`Confirmed upload ${index + 1}.\n`),
  }));
  await page.getByLabel("Upload files", { exact: true }).setInputFiles(uploads);
  await expect(page.getByText("Saved", { exact: true })).toHaveCount(12);
  for (const upload of uploads) expect(readFileSync(join(service.root, upload.name))).toEqual(upload.buffer);
  await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
  const layout = await page.evaluate(() => {
    const selected = document.querySelector(".selection-panel")!.getBoundingClientRect();
    const management = document.querySelector(".manage-panel")!.getBoundingClientRect();
    const transfers = document.querySelector(".transfer-panel")!.getBoundingClientRect();
    return { order: Array.from(document.querySelectorAll(".tools-column > section")).map((element) => element.getAttribute("aria-label")),
      actionGap: management.top - selected.bottom, transferGap: transfers.top - management.bottom,
      overflow: document.documentElement.scrollWidth > window.innerWidth };
  });
  expect(layout.order).toEqual(["Selected entry", "File actions", "Transfers"]);
  expect(layout.actionGap).toBeGreaterThanOrEqual(0);
  expect(layout.actionGap).toBeLessThanOrEqual(24);
  expect(layout.transferGap).toBeGreaterThanOrEqual(0);
  expect(layout.overflow).toBe(false);
  await test.info().attach("confirmed-upload-action-layout.json", {
    body: Buffer.from(JSON.stringify({ confirmedUploads: 12, ...layout }, null, 2)), contentType: "application/json",
  });
});

for (const productName of ["NeighborhoodFileStation", "NeighborhoodFileStationNeighborhoodFileStationNeighborhoodFileStationStationFile"]) {
  test.describe(`long runtime brand ${productName.length} characters`, () => {
    test.use({ serviceOptions: { modules: "files", productName } });
    test("mobile branding wraps without intersecting access controls or footer text", async ({ page, service }) => {
      await page.setViewportSize({ width: 390, height: 844 });
      await unlock(page, service.url, service.key);
      await expect(page.getByRole("heading", { name: productName, exact: true })).toBeVisible();
      await expect(page.getByText("Read only", { exact: true })).toBeVisible();
      await expect(page.getByRole("button", { name: "Lock files", exact: true })).toBeVisible();
      const layout = await page.evaluate(() => {
        const heading = document.querySelector(".brand h1")!.getBoundingClientRect();
        const brand = document.querySelector(".brand")!.getBoundingClientRect();
        const actions = document.querySelector(".header-actions")!.getBoundingClientRect();
        const footerBrand = document.querySelector(".app-footer > span:first-child")!.getBoundingClientRect();
        const footerNote = document.querySelector(".app-footer > span:last-child")!.getBoundingClientRect();
        return { intersects: heading.left < actions.right && heading.right > actions.left && heading.top < actions.bottom && heading.bottom > actions.top,
          footerIntersects: footerBrand.left < footerNote.right && footerBrand.right > footerNote.left && footerBrand.top < footerNote.bottom && footerBrand.bottom > footerNote.top,
          headingRight: heading.right, brandRight: brand.right,
          heading: heading.toJSON(), brand: brand.toJSON(), actions: actions.toJSON(),
          footerBrand: footerBrand.toJSON(), footerNote: footerNote.toJSON(),
          overflow: document.documentElement.scrollWidth > window.innerWidth };
      });
      expect(layout.intersects).toBe(false);
      expect(layout.headingRight).toBeLessThanOrEqual(layout.brandRight);
      expect(layout.footerIntersects).toBe(false);
      expect(layout.overflow).toBe(false);
      await test.info().attach(`mobile-brand-${productName.length}-characters.json`, {
        body: Buffer.from(JSON.stringify({ productName, viewport: { width: 390, height: 844 }, ...layout }, null, 2)), contentType: "application/json",
      });
    });
  });
}

test.describe("native finish behavior at the mocked command boundary", () => {
  test.use({ serviceOptions: { writable: false, modules: "files" } });

  test("running settings survive reload and an optional fresh key protects restart", async ({ page, service }) => {
    await installNativeBoundary(page, service.url);
    await page.goto("http://tauri.localhost");
    const accessKey = page.getByRole("region", { name: "Access key", exact: true }).locator("input");
    await page.getByRole("button", { name: "Choose folder", exact: true }).click();
    await page.getByLabel("Share on the local network", { exact: true }).check();
    await page.getByLabel("Allow uploads", { exact: true }).check();
    await page.getByLabel("Allow file management", { exact: true }).check();
    await page.getByLabel("Port", { exact: true }).fill("0");
    await page.getByRole("button", { name: "Generate access key", exact: true }).click();
    await page.getByRole("button", { name: "Start sharing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Stop sharing", exact: true })).toBeVisible();
    await expect(page.getByLabel("Port", { exact: true })).toHaveValue("49321");
    expect(await page.evaluate(() => JSON.stringify({ local: { ...localStorage }, session: { ...sessionStorage } }))).not.toContain(generatedKey);
    await page.reload();
    await expect(page.getByRole("button", { name: "Stop sharing", exact: true })).toBeVisible();
    await expect(page.getByLabel("Share on the local network", { exact: true })).toBeChecked();
    await expect(page.getByLabel("Allow uploads", { exact: true })).toBeChecked();
    await expect(page.getByLabel("Allow file management", { exact: true })).toBeChecked();
    await expect(page.getByLabel("Port", { exact: true })).toHaveValue("49321");
    await expect(accessKey).toHaveValue("");
    await expect(page.getByText(/access key is configured/i)).toBeVisible();
    await expect(page.getByRole("button", { name: "Show access key", exact: true })).toBeDisabled();
    await page.getByRole("button", { name: "Stop sharing", exact: true }).click();
    await expect(page.getByLabel("Port", { exact: true })).toHaveValue("5000");
    await expect(page.getByLabel("Share on the local network", { exact: true })).not.toBeChecked();
    await expect(page.getByLabel("Allow uploads", { exact: true })).not.toBeChecked();
    await expect(page.getByLabel("Allow file management", { exact: true })).not.toBeChecked();
    await page.getByLabel("Allow uploads", { exact: true }).check();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeEnabled();
    await accessKey.fill("new-local-restart-key");
    await page.getByRole("button", { name: "Start sharing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Stop sharing", exact: true })).toBeVisible();
    await expect(page.getByLabel("Allow uploads", { exact: true })).toBeChecked();
    expect(await page.evaluate(() => JSON.stringify({ local: { ...localStorage }, session: { ...sessionStorage } }))).not.toContain("new-local-restart-key");
  });

  test("initiated start and stop restore focus and announce status without passive focus theft", async ({ page, service }) => {
    const boundary = await installNativeBoundary(page, service.url);
    await page.goto("http://tauri.localhost");
    await page.getByRole("button", { name: "Choose folder", exact: true }).click();
    await page.getByLabel("Port", { exact: true }).fill("0");
    const start = page.getByRole("button", { name: "Start sharing", exact: true });
    await start.focus();
    await page.keyboard.press("Enter");
    const stop = page.getByRole("button", { name: "Stop sharing", exact: true });
    await expect(stop).toBeFocused();
    await expect(page.getByRole("status").filter({ hasText: /sharing is active/i })).toHaveAttribute("aria-live", "polite");
    const openManager = page.getByRole("button", { name: "Open file manager", exact: true });
    await openManager.click();
    await expect(openManager).toBeFocused();
    await stop.focus();
    await page.keyboard.press("Enter");
    await expect(start).toBeFocused();
    await expect(page.getByRole("status").filter({ hasText: /sharing is stopped/i })).toHaveAttribute("aria-live", "polite");
    await expect(page.getByLabel("Port", { exact: true })).toHaveValue("0");
    const chooseFolder = page.getByRole("button", { name: "Choose folder", exact: true });
    await chooseFolder.focus();
    const refreshes = boundary.calls.filter((call) => call.command === "host_status").length;
    await page.evaluate(() => window.dispatchEvent(new Event("focus")));
    await expect.poll(() => boundary.calls.filter((call) => call.command === "host_status").length).toBeGreaterThan(refreshes);
    await expect(chooseFolder).toBeFocused();
    expect(boundary.calls.filter((call) => call.command === "start_service")).toHaveLength(1);
  });

  test("enabled input boundaries meet contrast while text focus and reduced motion remain usable", async ({ page, service }) => {
    await installNativeBoundary(page, service.url);
    await page.goto("http://tauri.localhost");
    const port = page.getByLabel("Port", { exact: true });
    await expect(port).toBeEnabled();
    const colors = await port.evaluate((input) => {
      const style = getComputedStyle(input);
      return { border: style.borderTopColor, fill: style.backgroundColor, paper: getComputedStyle(document.body).backgroundColor,
        text: style.color };
    });
    const luminance = (color: string) => {
      const channels = color.match(/\d+(?:\.\d+)?/g)!.slice(0, 3).map(Number).map((channel) => {
        const value = channel / 255;
        return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
      });
      return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
    };
    const contrast = (first: string, second: string) => {
      const a = luminance(first), b = luminance(second);
      return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
    };
    expect(contrast(colors.border, colors.fill)).toBeGreaterThanOrEqual(3);
    expect(contrast(colors.border, colors.paper)).toBeGreaterThanOrEqual(3);
    expect(contrast(colors.text, colors.fill)).toBeGreaterThanOrEqual(4.5);
    await port.focus();
    const focus = await port.evaluate((input) => {
      const style = getComputedStyle(input);
      return { style: style.outlineStyle, width: style.outlineWidth, color: style.outlineColor };
    });
    expect(focus.style).toBe("solid");
    expect(focus.width).toBe("2px");
    expect(contrast(focus.color, colors.fill)).toBeGreaterThanOrEqual(3);
    await page.emulateMedia({ reducedMotion: "reduce" });
    expect(await page.getByRole("button", { name: "Generate access key", exact: true }).evaluate((button) => getComputedStyle(button).transitionDuration)).toBe("0s");
    await test.info().attach("input-contrast-focus-motion.json", {
      body: Buffer.from(JSON.stringify({ ...colors, borderFillContrast: contrast(colors.border, colors.fill),
        borderPaperContrast: contrast(colors.border, colors.paper), textFillContrast: contrast(colors.text, colors.fill),
        focus, focusFillContrast: contrast(focus.color, colors.fill), reducedMotionTransitionDuration: "0s" }, null, 2)),
      contentType: "application/json",
    });
  });
});
