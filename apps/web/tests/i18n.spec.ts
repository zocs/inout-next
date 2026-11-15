import { expect, type Page } from "@playwright/test";
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { test } from "./service";
import { installNativeBoundary } from "./native-boundary";
import type { EntryList } from "../src/lib/contracts";
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
test("a lost upload confirmation retains the publication-check instruction across locales", async ({ page, service }) => {
  await unlock(page, service);
  const bytes = Buffer.alloc(4096);
  for (let i = 0; i < bytes.length; i++) bytes[i] = i % 251;
  let publicationStatus: number | undefined;
  await page.route("**/api/v1/content?*", async route => {
    if (route.request().method() !== "PUT") { await route.continue(); return; }
    const response = await route.fetch();
    publicationStatus = response.status();
    await route.abort("connectionfailed");
  });
  await page.getByLabel("Upload files", { exact: true }).setInputFiles({ name: "lost-confirmation.bin", mimeType: "application/octet-stream", buffer: bytes });
  const transfer = page.locator(".transfer-panel");
  await expect(transfer.getByRole("status")).toHaveText("Failed");
  expect(publicationStatus).toBe(201);
  const published = readFileSync(join(service.root, "lost-confirmation.bin"));
  expect(published).toEqual(bytes);
  await expect(transfer.getByRole("alert")).toContainText("Refresh the folder to check whether it was saved");
  await expect(transfer.locator(".saved-mark")).toHaveCount(0);
  const alerts = [{ locale: "en", alert: await transfer.getByRole("alert").innerText() }];
  for (const [locale, language, instruction, status] of [
    ["zh-Hans", "Language", "刷新文件夹以检查文件是否已保存", "失败"],
    ["zh-Hant", "语言", "重新整理資料夾以確認檔案是否已儲存", "失敗"],
    ["en", "語言", "Refresh the folder to check whether it was saved", "Failed"],
  ]) {
    await page.getByLabel(language, { exact: true }).selectOption(locale);
    await expect(transfer.getByRole("alert")).toContainText(instruction);
    await expect(transfer.getByRole("status")).toHaveText(status);
    alerts.push({ locale, alert: await transfer.getByRole("alert").innerText() });
  }
  await page.getByRole("button", { name: "Refresh folder", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select lost-confirmation.bin", exact: true })).toBeVisible();
  await expect(transfer.getByRole("status")).toHaveText("Failed");
  await test.info().attach("lost-confirmation-publication.json", { body: JSON.stringify({ publicationStatus, bytes: published.length, expectedSha256: createHash("sha256").update(bytes).digest("hex"), publishedSha256: createHash("sha256").update(published).digest("hex"), alerts }), contentType: "application/json" });
});
test("ordinary connection failures keep their separate localized recovery instruction", async ({ page, service }) => {
  await page.route("**/api/v1/context", route => route.abort("connectionfailed"));
  await page.goto(service.url);
  const alert = page.locator(".connection-state").getByRole("alert");
  await expect(alert).toHaveText("Cannot reach the sharing device. Check the connection and try again.");
  await page.getByLabel("Language", { exact: true }).selectOption("zh-Hans");
  await expect(alert).toHaveText("无法连接共享设备。请检查连接后重试。");
  await page.getByLabel("语言", { exact: true }).selectOption("zh-Hant");
  await expect(alert).toHaveText("無法連線至共享裝置。請檢查連線後重試。");
});
test("unrepresentable modified dates keep browsing usable with a localized fallback", async ({ page, service }) => {
  writeFileSync(join(service.root, "epoch.txt"), "A valid zero timestamp.\n");
  const pageErrors: string[] = [];
  page.on("pageerror", error => pageErrors.push(`${error.name}: ${error.message}`));
  await page.route("**/api/v1/entries?*", async route => {
    const response = await route.fetch();
    const listing = await response.json() as EntryList;
    await route.fulfill({ response, json: { ...listing, entries: listing.entries.map(entry => ({ ...entry,
      modified: entry.name === "welcome.txt" ? 9007199254740991 : entry.name === "epoch.txt" ? 0 : entry.modified,
    })) } });
  });
  const observations: { locale: string; fallback: string; validDate: string }[] = [];
  try {
    await unlock(page, service);
    await expect(page.getByRole("button", { name: "Select welcome.txt", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
    for (const [locale, language, selection, epochSelection, fallback] of [
      ["en", "Language", "Select welcome.txt", "Select epoch.txt", "Unavailable"],
      ["zh-Hans", "Language", "选择 welcome.txt", "选择 epoch.txt", "不可用"],
      ["zh-Hant", "语言", "選取 welcome.txt", "選取 epoch.txt", "無法使用"],
      ["en", "語言", "Select welcome.txt", "Select epoch.txt", "Unavailable"],
    ]) {
      await page.getByLabel(language, { exact: true }).selectOption(locale);
      const row = page.getByRole("row").filter({ has: page.getByRole("button", { name: selection, exact: true }) });
      await expect(row.locator(".modified-column")).toHaveText(fallback);
      await expect(page.locator(".selection-panel")).toContainText("welcome.txt");
      await expect(page.locator(".selection-panel dd").filter({ hasText: fallback })).toHaveCount(1);
      const epochDate = page.getByRole("row").filter({ has: page.getByRole("button", { name: epochSelection, exact: true }) }).locator(".modified-column");
      await expect(epochDate).toHaveText(locale === "en" ? "Jan 1, 1970" : "1970年1月1日");
      observations.push({ locale, fallback: await row.locator(".modified-column").innerText(), validDate: await epochDate.innerText() });
    }
    await page.getByRole("button", { name: "Rename", exact: true }).click();
    await expect(page.getByLabel("New name", { exact: true })).toHaveValue("welcome.txt");
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("link", { name: "Download welcome.txt", exact: true }).click();
    const download = await downloadPromise;
    expect(readFileSync((await download.path())!)).toEqual(readFileSync(join(service.root, "welcome.txt")));
    expect(pageErrors).toEqual([]);
  } finally {
    await test.info().attach("modified-date-recovery.json", { body: JSON.stringify({ injectedModified: 9007199254740991, validModified: 0, pageErrors, fileRows: await page.locator("tbody tr").count(), observations }), contentType: "application/json" });
  }
});
for (const [languages, expected, language] of [
  [["zh-TW", "en"], "zh-Hant", "語言"], [["zh-CN", "en"], "zh-Hans", "语言"],
  [["ja-JP", "en-GB"], "en", "Language"], [["ja-JP", "th-TH"], "en", "Language"], [["zh-Hant-HK", "zh-CN"], "zh-Hant", "語言"],
  [["zh-Hans-HK", "en"], "zh-Hans", "语言"], [["zh-Hans-TW", "en"], "zh-Hans", "语言"], [["zh-Hans-MO", "en"], "zh-Hans", "语言"],
  [["zh-Hant-CN", "en"], "zh-Hant", "語言"], [["zh-HK", "en"], "zh-Hant", "語言"], [["zh-MO", "en"], "zh-Hant", "語言"],
] as const) {
  test(`system languages ${languages.join(",")} resolve to ${expected}`, async ({ page, service }) => {
    await page.addInitScript(values => Object.defineProperty(navigator, "languages", { value: values }), [...languages]); await page.goto(service.url);
    await expect(page.locator("html")).toHaveAttribute("lang", expected); await expect(page.getByLabel(language, { exact: true })).toHaveValue("system");
  });
}
for (const [systemLanguage, preference, language] of [
  ["zh-Hans-HK", "zh-Hant", "語言"], ["zh-Hant-CN", "zh-Hans", "语言"],
] as const) {
  test(`stored ${preference} preference wins over system ${systemLanguage}`, async ({ page, service }) => {
    await page.addInitScript(({ systemLanguage, preference }) => {
      Object.defineProperty(navigator, "languages", { value: [systemLanguage, "en"] });
      localStorage.setItem("share-ui.locale", preference);
    }, { systemLanguage, preference });
    await page.goto(service.url);
    await expect(page.locator("html")).toHaveAttribute("lang", preference);
    await expect(page.getByLabel(language, { exact: true })).toHaveValue(preference);
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
