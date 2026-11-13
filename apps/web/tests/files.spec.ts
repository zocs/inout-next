import { expect } from "@playwright/test";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { test } from "./service";

test("login and real folder/upload/rename/move/copy/download/delete flow", async ({ page, service }) => {
  await page.goto(service.url);
  await expect(page.getByRole("heading", { name: "Unlock shared files" })).toBeVisible();
  await page.getByLabel("Access key", { exact: true }).fill("wrong-key");
  await page.getByRole("button", { name: "Unlock files" }).click();
  await expect(page.getByRole("main").getByRole("alert")).toBeVisible();
  await page.getByLabel("Access key", { exact: true }).fill(service.key);
  await page.getByRole("button", { name: "Unlock files" }).click();
  await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();

  await page.getByRole("button", { name: "New folder", exact: true }).click();
  await page.getByLabel("Folder name", { exact: true }).fill("Projects");
  await page.getByRole("button", { name: "Create folder", exact: true }).click();
  await expect(page.getByRole("button", { name: "Open Projects", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Open Projects", exact: true }).click();

  const bytes = Buffer.alloc(1024 * 1024);
  for (let i = 0; i < bytes.length; i++) bytes[i] = (i * 31 + 7) % 256;
  await page.getByLabel("Upload files", { exact: true }).setInputFiles({ name: "report.bin", mimeType: "application/octet-stream", buffer: bytes });
  await expect(page.getByText("Saved", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Select report.bin", exact: true })).toBeVisible();
  expect(readFileSync(join(service.root, "Projects/report.bin"))).toEqual(bytes);

  await page.getByRole("button", { name: "Select report.bin", exact: true }).click();
  await page.getByRole("button", { name: "Rename", exact: true }).click();
  await page.getByLabel("New name", { exact: true }).fill("renamed.bin");
  await page.getByRole("button", { name: "Save name", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select renamed.bin", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Select renamed.bin", exact: true }).click();
  await page.getByRole("button", { name: "Move", exact: true }).click();
  await page.getByLabel("Destination path", { exact: true }).fill("moved.bin");
  await page.getByRole("button", { name: "Move file", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select renamed.bin", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Shared folder", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select moved.bin", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Select moved.bin", exact: true }).click();
  await page.getByRole("button", { name: "Copy", exact: true }).click();
  await page.getByLabel("Destination path", { exact: true }).fill("copy.bin");
  await page.getByRole("button", { name: "Copy file", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select copy.bin", exact: true })).toBeVisible();
  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("link", { name: "Download copy.bin", exact: true }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe("copy.bin");
  const downloaded = readFileSync((await download.path())!);
  expect(createHash("sha256").update(downloaded).digest("hex")).toBe(createHash("sha256").update(bytes).digest("hex"));

  await page.getByRole("button", { name: "Select copy.bin", exact: true }).click();
  await page.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Delete copy.bin?", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Delete permanently", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select copy.bin", exact: true })).toHaveCount(0);
  expect(existsSync(join(service.root, "copy.bin"))).toBe(false);
});

test("a duplicate upload shows a recoverable error and preserves the original", async ({ page, service }) => {
  await page.goto(service.url);
  await page.getByLabel("Access key", { exact: true }).fill(service.key);
  await page.getByRole("button", { name: "Unlock files" }).click();
  const original = readFileSync(join(service.root, "welcome.txt"));
  await page.getByLabel("Upload files", { exact: true }).setInputFiles({ name: "welcome.txt", mimeType: "text/plain", buffer: Buffer.from("do not replace") });
  await expect(page.getByText(/already exists|already present/i)).toBeVisible();
  expect(readFileSync(join(service.root, "welcome.txt"))).toEqual(original);
});

test("a provider without exclusive file moves hides rename and move", async ({ page, service }) => {
  await page.route("**/api/v1/context", async (route) => {
    const response = await route.fetch();
    const context = await response.json();
    await route.fulfill({ response, json: { ...context, capabilities: { ...context.capabilities, file_move: false } } });
  });
  await page.goto(service.url);
  await page.getByLabel("Access key", { exact: true }).fill(service.key);
  await page.getByRole("button", { name: "Unlock files" }).click();
  await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
  await expect(page.getByRole("button", { name: "Rename", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Move", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Copy", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Delete", exact: true })).toBeVisible();
});

test("canceling a throttled upload publishes no partial file", async ({ page, service }) => {
  await page.goto(service.url);
  await page.getByLabel("Access key", { exact: true }).fill(service.key);
  await page.getByRole("button", { name: "Unlock files" }).click();
  await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
  const session = await page.context().newCDPSession(page);
  await session.send("Network.enable");
  await session.send("Network.emulateNetworkConditions", { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: 64 * 1024 });
  const initialEntries = readdirSync(service.root).sort();
  await page.getByLabel("Upload files", { exact: true }).setInputFiles({ name: "cancel.bin", mimeType: "application/octet-stream", buffer: Buffer.alloc(2 * 1024 * 1024, 9) });
  await page.getByRole("button", { name: "Cancel cancel.bin", exact: true }).click();
  await expect(page.getByText("Canceled", { exact: true })).toBeVisible();
  await expect.poll(() => ({
    publicEntries: readdirSync(service.root).filter((name) => name !== "INOUTTMP").sort(),
    stagingFiles: existsSync(join(service.root, "INOUTTMP")) ? readdirSync(join(service.root, "INOUTTMP")) : [],
  })).toEqual({ publicEntries: initialEntries, stagingFiles: [] });
  const listedNames = await page.evaluate(async () => {
    const response = await fetch("/api/v1/entries?path=");
    if (!response.ok) throw new Error("Could not inspect the public listing after cancellation");
    const listing = await response.json();
    return listing.entries.map((entry: { name: string }) => entry.name).sort();
  });
  expect(listedNames).toEqual(initialEntries);
});

test.describe("read-only files module", () => {
  test.use({ serviceOptions: { writable: false, modules: "files" } });
  test("only available actions appear and keyboard/mobile layouts remain usable", async ({ page, service }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(service.url);
    await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
    await expect(page.getByRole("button", { name: "New folder", exact: true })).toHaveCount(0);
    await expect(page.getByLabel("Upload files", { exact: true })).toHaveCount(0);
    await expect(page.getByText("Read only", { exact: true })).toBeVisible();
    const row = page.getByRole("button", { name: "Select welcome.txt", exact: true });
    await row.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("region", { name: "Selected entry" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Delete", exact: true })).toHaveCount(0);
    const layout = await page.evaluate(() => ({
      overflow: document.documentElement.scrollWidth > window.innerWidth,
      main: document.querySelector("main")?.getBoundingClientRect().width,
      download: document.querySelector<HTMLAnchorElement>('a[download]')?.getBoundingClientRect().height,
    }));
    expect(layout.overflow).toBe(false);
    expect(layout.main).toBeLessThanOrEqual(390);
    expect(layout.download).toBeGreaterThanOrEqual(40);
    await page.setViewportSize({ width: 1440, height: 900 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});

test.describe("alternate brand with disabled writable modules", () => {
  test.use({ serviceOptions: { modules: "files", alternateProduct: true } });
  test("runtime product replaces static fallback and disabled APIs stay unavailable", async ({ page, service, request }) => {
    await page.goto(service.url);
    await page.getByLabel("Access key", { exact: true }).fill(service.key);
    await page.getByRole("button", { name: "Unlock files" }).click();
    await expect(page.getByRole("heading", { name: "Harbor", exact: true })).toBeVisible();
    await expect(page).toHaveTitle(/Harbor/);
    await expect(page.getByLabel("Upload files", { exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "New folder", exact: true })).toHaveCount(0);
    const response = await request.put(service.url + "/api/v1/content?path=disabled.bin", {
      headers: { authorization: "Bearer " + service.key }, data: "disabled",
    });
    expect([404, 405]).toContain(response.status());
    expect(existsSync(join(service.root, "disabled.bin"))).toBe(false);
  });
});
