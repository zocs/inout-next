import { expect } from "@playwright/test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "./service";
import { installNativeBoundary } from "./native-boundary";

test.describe("explicit public writes", () => {
  test.use({ serviceOptions: { keyed: false } });

  test("unkeyed sharing uploads and manages files while deletion stays unavailable", async ({ page, service, request }) => {
    await page.goto(service.url);
    await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
    await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
    await expect(page.getByRole("button", { name: "Rename", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Delete", exact: true })).toHaveCount(0);
    const original = readFileSync(join(service.root, "welcome.txt"));
    const denied = await request.delete(service.url + "/api/v1/entries?path=welcome.txt");
    expect(denied.status()).toBe(403);
    expect(readFileSync(join(service.root, "welcome.txt"))).toEqual(original);

    const bytes = Buffer.alloc(256 * 1024, 37);
    await page.getByLabel("Upload files", { exact: true }).setInputFiles({ name: "public.bin", mimeType: "application/octet-stream", buffer: bytes });
    await expect(page.getByText("Saved", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "New folder", exact: true }).click();
    await page.getByLabel("Folder name", { exact: true }).fill("Public");
    await page.getByRole("button", { name: "Create folder", exact: true }).click();
    await expect(page.getByRole("button", { name: "Open Public", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Select public.bin", exact: true }).click();
    await page.getByRole("button", { name: "Rename", exact: true }).click();
    await page.getByLabel("New name", { exact: true }).fill("renamed-public.bin");
    await page.getByRole("button", { name: "Save name", exact: true }).click();
    await expect(page.getByRole("button", { name: "Select renamed-public.bin", exact: true })).toBeVisible();
    expect(createHash("sha256").update(readFileSync(join(service.root, "renamed-public.bin"))).digest("hex"))
      .toBe(createHash("sha256").update(bytes).digest("hex"));
  });
});

for (const action of ["rename", "delete"] as const) {
  test(`the ${action} permission remains independent of general management`, async ({ page, service }) => {
    // Only the advertised context is replaced to test contribution selection.
    // The Rust policy integration suite separately checks action authorization.
    await page.route("**/api/v1/context", async (route) => {
      const response = await route.fetch();
      if (!response.ok()) { await route.fulfill({ response }); return; }
      const context = await response.json();
      await route.fulfill({ response, json: { ...context, permissions: {
        ...context.permissions, upload: false, manage: false, rename: action === "rename", delete: action === "delete",
      } } });
    });
    await page.goto(service.url);
    await page.getByLabel("Access key", { exact: true }).fill(service.key);
    await page.getByRole("button", { name: "Unlock files" }).click();
    await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
    await expect(page.getByRole("button", { name: action === "rename" ? "Rename" : "Delete", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: action === "rename" ? "Delete" : "Rename", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Move", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Copy", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "New folder", exact: true })).toHaveCount(0);
    await expect(page.getByText("Can edit", { exact: true })).toBeVisible();
  });
}

test("older context responses retain their management actions", async ({ page, service }) => {
  await page.route("**/api/v1/context", async (route) => {
    const response = await route.fetch();
    if (!response.ok()) { await route.fulfill({ response }); return; }
    const context = await response.json();
    const { rename: _rename, delete: _delete, ...permissions } = context.permissions;
    await route.fulfill({ response, json: { ...context, permissions } });
  });
  await page.goto(service.url);
  await page.getByLabel("Access key", { exact: true }).fill(service.key);
  await page.getByRole("button", { name: "Unlock files" }).click();
  await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
  await expect(page.getByRole("button", { name: "Rename", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Delete", exact: true })).toBeVisible();
});

test.describe("optional native access key", () => {
  test.use({ serviceOptions: { writable: false, modules: "files" } });

  test("native setup can submit explicitly enabled writes without a key", async ({ page, service }) => {
    const boundary = await installNativeBoundary(page, service.url);
    await page.goto("http://tauri.localhost");
    await page.getByRole("button", { name: "Choose folder", exact: true }).click();
    await page.getByLabel("Allow uploads", { exact: true }).check();
    await page.getByLabel("Allow file management", { exact: true }).check();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeEnabled();
    await expect(page.getByText(/Deleting files requires an access key/)).toBeVisible();
    await page.getByRole("button", { name: "Start sharing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Stop sharing", exact: true })).toBeVisible();
    expect(boundary.calls.find((call) => call.command === "start_service")?.args).toEqual({ options: {
      network: false, port: 5000, uploads: true, manage: true, key: null,
    } });
  });
});
