import { expect, type Page } from "@playwright/test";
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { test } from "./service";
import { installNativeBoundary } from "./native-boundary";
async function unlock(page: Page, service: { url: string; key: string }) {
  await page.goto(service.url); await page.getByLabel("Access key", { exact: true }).fill(service.key);
  await page.getByRole("button", { name: "Unlock files", exact: true }).click();
}
test("locale switches preserve folder, filter, selection, rename input and open dialog", async ({ page, service }) => {
  mkdirSync(join(service.root, "资料")); writeFileSync(join(service.root, "资料", "welcome.txt"), "Nested data stays intact.\n");
  await unlock(page, service); await page.getByRole("button", { name: "Open 资料", exact: true }).click();
  await page.getByLabel("Search this folder", { exact: true }).fill("welcome");
  await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
  await page.getByRole("button", { name: "Rename", exact: true }).click(); await page.getByLabel("New name", { exact: true }).fill("draft-kept.txt");
  let contexts = 0; page.on("request", request => { if (request.url().includes("/api/v1/context")) contexts++; });
  for (const [locale, language, input, selection, search] of [
    ["zh-Hans", "Language", "新名称", "所选项目", "搜索此文件夹"], ["zh-Hant", "语言", "新名稱", "所選項目", "搜尋此資料夾"], ["en", "語言", "New name", "Selected entry", "Search this folder"],
  ]) {
    await page.getByLabel(language, { exact: true }).selectOption(locale); await expect(page.locator("html")).toHaveAttribute("lang", locale);
    await expect(page.getByLabel(input, { exact: true })).toHaveValue("draft-kept.txt"); await expect(page.getByRole("region", { name: selection, exact: true })).toContainText("welcome.txt");
    await expect(page.getByLabel(search, { exact: true })).toHaveValue("welcome"); expect(new URL(page.url()).searchParams.get("path")).toBe("资料");
  }
  expect(contexts).toBe(0); await page.getByRole("button", { name: "Delete", exact: true }).click();
  await page.getByLabel("Language", { exact: true }).selectOption("zh-Hans"); await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.getByRole("dialog")).toContainText("永久删除"); await page.getByRole("button", { name: "取消", exact: true }).click();
});
test("a throttled upload survives locale and theme switches with exact published bytes", async ({ page, service, context }) => {
  test.setTimeout(90_000); await unlock(page, service); const session = await context.newCDPSession(page); await session.send("Network.enable");
  await session.send("Network.emulateNetworkConditions", { offline: false, latency: 50, downloadThroughput: -1, uploadThroughput: 96 * 1024 });
  const bytes = Buffer.alloc(2 * 1024 * 1024); for (let i = 0; i < bytes.length; i++) bytes[i] = i % 251;
  await page.getByLabel("Upload files", { exact: true }).setInputFiles({ name: "transfer-资料.bin", mimeType: "application/octet-stream", buffer: bytes });
  await expect(page.getByRole("progressbar")).toBeVisible(); await expect(page.getByRole("progressbar")).not.toHaveAttribute("value", "100");
  await page.getByLabel("Language", { exact: true }).selectOption("zh-Hans"); await page.getByLabel("主题", { exact: true }).selectOption("tangerine");
  await expect(page.getByRole("region", { name: "传输", exact: true })).toContainText("上传中");
  await page.getByLabel("语言", { exact: true }).selectOption("zh-Hant"); await page.getByLabel("主題", { exact: true }).selectOption("ocean");
  await expect(page.getByRole("region", { name: "傳輸", exact: true })).toContainText("已儲存", { timeout: 60_000 }); const published = readFileSync(join(service.root, "transfer-资料.bin")); expect(published).toEqual(bytes);
  await test.info().attach("published-transfer.json", { body: JSON.stringify({ bytes: published.length, expectedSha256: createHash("sha256").update(bytes).digest("hex"), publishedSha256: createHash("sha256").update(published).digest("hex"), uploadBytesPerSecond: 96 * 1024 }), contentType: "application/json" });
});
for (const [languages, expected, language] of [ [["zh-TW", "en"], "zh-Hant", "語言"], [["zh-CN", "en"], "zh-Hans", "语言"], [["ja-JP", "en-GB"], "en", "Language"], [["ja-JP", "th-TH"], "en", "Language"], [["zh-Hant-HK", "zh-CN"], "zh-Hant", "語言"] ] as const) {
  test(`system languages ${languages.join(",")} resolve to ${expected}`, async ({ page, service }) => {
    await page.addInitScript(values => Object.defineProperty(navigator, "languages", { value: values }), [...languages]); await page.goto(service.url);
    await expect(page.locator("html")).toHaveAttribute("lang", expected); await expect(page.getByLabel(language, { exact: true })).toHaveValue("system");
  });
}
test("preferences persist and corrupt or denied storage recovers", async ({ page, service, browser }) => {
  await page.goto(service.url); await page.getByLabel("Language", { exact: true }).selectOption("zh-Hant"); await page.getByLabel("主題", { exact: true }).selectOption("berry"); await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "zh-Hant"); await expect(page.locator("html")).toHaveAttribute("data-theme", "berry");
  await page.evaluate(() => { localStorage.setItem("share-ui.locale", "bad"); localStorage.setItem("share-ui.theme", "bad"); }); await page.reload();
  await expect(page.locator("html")).toHaveAttribute("lang", "en"); await expect(page.locator("html")).toHaveAttribute("data-theme", "evergreen");
  const denied = await browser.newContext(); await denied.addInitScript(() => { Object.defineProperty(window, "localStorage", { get() { throw new DOMException("Denied", "SecurityError"); } }); });
  const deniedPage = await denied.newPage(); const errors: string[] = []; deniedPage.on("pageerror", error => errors.push(error.message)); await deniedPage.goto(service.url);
  await deniedPage.getByLabel("Language", { exact: true }).selectOption("zh-Hans"); await deniedPage.getByLabel("主题", { exact: true }).selectOption("ocean");
  await expect(deniedPage.locator("html")).toHaveAttribute("lang", "zh-Hans"); await expect(deniedPage.locator("html")).toHaveAttribute("data-theme", "ocean"); expect(errors).toEqual([]); await denied.close();
});
test("known errors rerender in the chosen locale and unknown details stay usable", async ({ page, service }) => {
  await page.goto(service.url); await page.getByLabel("Access key", { exact: true }).fill("incorrect-key"); await page.getByRole("button", { name: "Unlock files", exact: true }).click();
  await expect(page.locator(".unlock-panel").getByRole("alert")).toBeVisible(); await page.getByLabel("Language", { exact: true }).selectOption("zh-Hans"); await expect(page.locator(".unlock-panel").getByRole("alert")).toContainText("访问密钥");
  await page.route("**/api/v1/session", route => route.fulfill({ status: 400, contentType: "application/json", body: JSON.stringify({ error: { code: "provider_future", message: "Provider detail: 资料" } }) }));
  await page.getByRole("button", { name: "解锁文件", exact: true }).click(); await expect(page.locator(".unlock-panel").getByRole("alert")).toHaveText("Provider detail: 资料");
});
test("native settings and lazy diagnostics retain inputs and service across preferences", async ({ page, service }) => {
  const boundary = await installNativeBoundary(page, service.url); await page.goto("http://tauri.localhost"); await page.getByRole("button", { name: "Choose folder", exact: true }).click();
  await page.getByLabel("Port", { exact: true }).fill("0"); await page.getByLabel("Allow uploads", { exact: true }).check(); await page.getByRole("textbox", { name: "Access key", exact: true }).fill("retained-key");
  await page.getByLabel("Language", { exact: true }).selectOption("zh-Hans"); await expect(page.getByLabel("端口", { exact: true })).toHaveValue("0"); await expect(page.getByRole("textbox", { name: "访问密钥", exact: true })).toHaveValue("retained-key");
  await page.getByRole("button", { name: "开始共享", exact: true }).click(); await page.getByLabel("语言", { exact: true }).selectOption("zh-Hant"); await page.getByLabel("主題", { exact: true }).selectOption("berry");
  await expect(page.getByRole("button", { name: "停止共享", exact: true })).toBeVisible(); expect(boundary.calls.filter(call => call.command === "start_service")).toHaveLength(1); expect(boundary.calls.filter(call => call.command === "stop_service")).toHaveLength(0);
  await page.getByRole("heading", { level: 1 }).getByRole("button").click({ clickCount: 5 }); await page.getByLabel("共享檔案相對路徑", { exact: true }).fill("资料/test.bin");
  await page.getByLabel("語言", { exact: true }).selectOption("en"); await expect(page.getByLabel("Relative shared-file path", { exact: true })).toHaveValue("资料/test.bin");
  expect(boundary.calls.filter(call => call.command === "set_diagnostic_mode" && (call.args as { enabled: boolean }).enabled)).toHaveLength(1);
});

test("unknown API codes matching object property names retain provider details", async ({ page, service }) => {
  await page.goto(service.url);
  await page.getByLabel("Language", { exact: true }).selectOption("zh-Hans");
  await page.getByLabel("访问密钥", { exact: true }).fill("incorrect-key");
  for (const code of ["toString", "__proto__"]) {
    await page.route("**/api/v1/session", route => route.fulfill({ status: 400, contentType: "application/json", body: JSON.stringify({ error: { code, message: `Provider ${code}: 资料` } }) }));
    await page.getByRole("button", { name: "解锁文件", exact: true }).click();
    await expect(page.locator(".unlock-panel").getByRole("alert")).toHaveText(`Provider ${code}: 资料`);
    await page.unroute("**/api/v1/session");
  }
});
