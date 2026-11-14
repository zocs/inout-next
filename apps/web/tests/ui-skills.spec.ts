import { expect, type Locator, type Page } from "@playwright/test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "./service";

async function unlock(page: Page, url: string, key: string) {
  await page.goto(url);
  await page.getByLabel("Access key", { exact: true }).fill(key);
  await page.getByRole("button", { name: "Unlock files", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select welcome.txt", exact: true })).toBeVisible();
}

async function activate(page: Page, control: Locator) {
  await control.focus();
  await page.keyboard.press("Enter");
}

for (const action of ["New folder", "Rename", "Move", "Copy"]) {
  test(`inline ${action} cancellation returns to its opener after form removal`, async ({ page, service }) => {
    await unlock(page, service.url, service.key);
    if (action !== "New folder") await activate(page, page.getByRole("button", { name: "Select welcome.txt", exact: true }));
    const opener = page.getByRole("button", { name: action, exact: true });
    await activate(page, opener);
    const input = page.getByRole("textbox", { name: action === "New folder" ? "Folder name" : action === "Rename" ? "New name" : "Destination path", exact: true });
    await expect(input).toBeFocused();
    await activate(page, page.getByRole("button", { name: "Cancel", exact: true }));
    await expect(input).toHaveCount(0);
    await expect(opener).toBeFocused();
  });
}

for (const cancellation of ["Cancel", "Escape"]) {
  test(`delete ${cancellation} returns to Delete after dialog removal`, async ({ page, service }) => {
    await unlock(page, service.url, service.key);
    await activate(page, page.getByRole("button", { name: "Select welcome.txt", exact: true }));
    const opener = page.getByRole("button", { name: "Delete", exact: true });
    await activate(page, opener);
    const cancel = page.getByRole("button", { name: "Cancel", exact: true });
    await expect(cancel).toBeFocused();
    await page.keyboard.press(cancellation === "Escape" ? "Escape" : "Enter");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(opener).toBeFocused();
    expect(existsSync(join(service.root, "welcome.txt"))).toBe(true);
  });
}

test("real folder creation returns to the current nested folder after form removal", async ({ page, service }) => {
  mkdirSync(join(service.root, "Projects"));
  await unlock(page, service.url, service.key);
  await activate(page, page.getByRole("button", { name: "Open Projects", exact: true }));
  await activate(page, page.getByRole("button", { name: "New folder", exact: true }));
  const input = page.getByLabel("Folder name", { exact: true });
  await input.fill("Created");
  await activate(page, page.getByRole("button", { name: "Create folder", exact: true }));
  await expect(page.getByRole("button", { name: "Open Created", exact: true })).toBeVisible();
  expect(existsSync(join(service.root, "Projects", "Created"))).toBe(true);
  await expect(input).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Projects", exact: true })).toBeFocused();
});

for (const action of ["Rename", "Move", "Copy"]) {
  test(`real ${action} returns to the current folder after form removal`, async ({ page, service }) => {
    await unlock(page, service.url, service.key);
    await activate(page, page.getByRole("button", { name: "Select welcome.txt", exact: true }));
    await activate(page, page.getByRole("button", { name: action, exact: true }));
    const input = page.getByLabel(action === "Rename" ? "New name" : "Destination path", { exact: true });
    await input.fill("result.txt");
    await activate(page, page.getByRole("button", { name: action === "Rename" ? "Save name" : `${action} file`, exact: true }));
    await expect(page.getByRole("button", { name: "Select result.txt", exact: true })).toBeVisible();
    expect(readFileSync(join(service.root, "result.txt"), "utf8")).toBe("Files stay on the sharing device.\n");
    expect(existsSync(join(service.root, "welcome.txt"))).toBe(action === "Copy");
    await expect(input).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Shared folder", exact: true })).toBeFocused();
  });
}

test("real deletion returns to the current folder after dialog removal", async ({ page, service }) => {
  await unlock(page, service.url, service.key);
  await activate(page, page.getByRole("button", { name: "Select welcome.txt", exact: true }));
  await activate(page, page.getByRole("button", { name: "Delete", exact: true }));
  await activate(page, page.getByRole("button", { name: "Delete permanently", exact: true }));
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Select welcome.txt", exact: true })).toHaveCount(0);
  expect(existsSync(join(service.root, "welcome.txt"))).toBe(false);
  await expect(page.getByRole("button", { name: "Shared folder", exact: true })).toBeFocused();
});

for (const action of ["New folder", "Copy", "Delete"]) {
  test(`failed real ${action} retains its actionable task and recovers focus`, async ({ page, service }) => {
    mkdirSync(join(service.root, "Occupied"));
    writeFileSync(join(service.root, "Occupied", "keep.txt"), "Keep this entry.\n");
    await unlock(page, service.url, service.key);
    if (action !== "New folder") await activate(page, page.getByRole("button", { name: action === "Delete" ? "Select Occupied" : "Select welcome.txt", exact: true }));
    await activate(page, page.getByRole("button", { name: action, exact: true }));
    const input = page.getByLabel(action === "New folder" ? "Folder name" : "Destination path", { exact: true });
    if (action !== "Delete") await input.fill(action === "Copy" ? "welcome.txt" : "Occupied");
    const submit = page.getByRole("button", { name: action === "New folder" ? "Create folder" : action === "Copy" ? "Copy file" : "Delete permanently", exact: true });
    await activate(page, submit);
    await expect(page.getByRole("alert")).toBeVisible();
    await expect(submit).toBeEnabled();
    if (action === "Delete") {
      await expect(page.getByRole("dialog")).toBeVisible();
      await expect(page.getByRole("button", { name: "Cancel", exact: true })).toBeFocused();
    } else {
      await expect(input).toHaveValue(action === "Copy" ? "welcome.txt" : "Occupied");
      await expect(input).toBeFocused();
    }
    expect(readFileSync(join(service.root, "Occupied", "keep.txt"), "utf8")).toBe("Keep this entry.\n");
    expect(existsSync(join(service.root, "welcome.txt"))).toBe(true);
  });
}

test("Clear finished retains keyboard position after history removal", async ({ page, service }) => {
  await unlock(page, service.url, service.key);
  await page.getByLabel("Upload files", { exact: true }).setInputFiles({ name: "saved.txt", mimeType: "text/plain", buffer: Buffer.from("Confirmed history.\n") });
  await expect(page.getByText("Saved", { exact: true })).toBeVisible();
  expect(readFileSync(join(service.root, "saved.txt"), "utf8")).toBe("Confirmed history.\n");
  const clear = page.getByRole("button", { name: "Clear finished", exact: true });
  await activate(page, clear);
  await expect(clear).toHaveCount(0);
  await expect(page.getByText("Saved", { exact: true })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Transfers", exact: true })).toBeFocused();
});

test("Dismiss message retains keyboard position after notice removal", async ({ page, service }) => {
  await unlock(page, service.url, service.key);
  await activate(page, page.getByRole("button", { name: "New folder", exact: true }));
  await page.getByLabel("Folder name", { exact: true }).fill("Notice");
  await activate(page, page.getByRole("button", { name: "Create folder", exact: true }));
  const dismiss = page.getByRole("button", { name: "Dismiss message", exact: true });
  await activate(page, dismiss);
  await expect(dismiss).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Shared folder", exact: true })).toBeFocused();
});

for (const action of ["New folder", "Rename", "Copy", "failed New folder"]) {
  test(`pending real ${action} preserves an unrelated user-chosen focus destination`, async ({ page, service }) => {
    await unlock(page, service.url, service.key);
    if (action.includes("New folder")) {
      if (action.startsWith("failed")) mkdirSync(join(service.root, "Result"));
      await activate(page, page.getByRole("button", { name: "New folder", exact: true }));
      await page.getByLabel("Folder name", { exact: true }).fill("Result");
    } else {
      await activate(page, page.getByRole("button", { name: "Select welcome.txt", exact: true }));
      await activate(page, page.getByRole("button", { name: action, exact: true }));
      await page.getByLabel(action === "Rename" ? "New name" : "Destination path", { exact: true }).fill("result.txt");
    }
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    let reached!: () => void;
    const requested = new Promise<void>((resolve) => { reached = resolve; });
    await page.route(action.includes("New folder") ? "**/api/v1/directories" : "**/api/v1/operations", async (route) => {
      const response = await route.fetch(); // The actual service executes before its response is delayed.
      reached();
      await gate;
      await route.fulfill({ response });
    });
    try {
      await activate(page, page.getByRole("button", { name: action.includes("New folder") ? "Create folder" : action === "Rename" ? "Save name" : "Copy file", exact: true }));
      await requested;
      expect(existsSync(join(service.root, action.includes("New folder") ? "Result" : "result.txt"))).toBe(true);
      const search = page.getByRole("searchbox", { name: "Search this folder", exact: true });
      await search.focus();
      release();
      if (action.startsWith("failed")) {
        await expect(page.getByRole("alert")).toBeVisible();
        await expect(page.getByLabel("Folder name", { exact: true })).toHaveValue("Result");
      } else {
        await expect(page.getByRole("textbox", { name: action === "New folder" ? "Folder name" : action === "Rename" ? "New name" : "Destination path", exact: true })).toHaveCount(0);
      }
      await expect(search).toBeFocused();
    } finally { release(); }
  });
}

test("explicit removal activated outside its task preserves unrelated focus", async ({ page, service }) => {
  await unlock(page, service.url, service.key);
  const search = page.getByRole("searchbox", { name: "Search this folder", exact: true });
  await activate(page, page.getByRole("button", { name: "New folder", exact: true }));
  await search.focus();
  await page.getByRole("button", { name: "Cancel", exact: true }).evaluate((button: HTMLButtonElement) => button.click());
  await expect(page.getByLabel("Folder name", { exact: true })).toHaveCount(0);
  await expect(search).toBeFocused();
  await activate(page, page.getByRole("button", { name: "New folder", exact: true }));
  await page.getByLabel("Folder name", { exact: true }).fill("Notice");
  await activate(page, page.getByRole("button", { name: "Create folder", exact: true }));
  const dismiss = page.getByRole("button", { name: "Dismiss message", exact: true });
  await search.focus();
  await dismiss.evaluate((button: HTMLButtonElement) => button.click());
  await expect(dismiss).toHaveCount(0);
  await expect(search).toBeFocused();
  await page.getByLabel("Upload files", { exact: true }).setInputFiles({ name: "saved.txt", mimeType: "text/plain", buffer: Buffer.from("History.\n") });
  await expect(page.getByText("Saved", { exact: true })).toBeVisible();
  await search.focus();
  const clear = page.getByRole("button", { name: "Clear finished", exact: true });
  await clear.evaluate((button: HTMLButtonElement) => button.click());
  await expect(clear).toHaveCount(0);
  await expect(search).toBeFocused();
});
