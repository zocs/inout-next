import { expect, type Page } from "@playwright/test";
import { utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { crc32, deflateSync } from "node:zlib";
import { test } from "./service";

// A real, decodable truecolor PNG; no binary fixture is kept in the repository.
function png(width: number, height: number): Buffer {
  const chunk = (type: string, data: Buffer) => {
    const body = Buffer.concat([Buffer.from(type, "latin1"), data]);
    const length = Buffer.alloc(4); length.writeUInt32BE(data.length);
    const checksum = Buffer.alloc(4); checksum.writeUInt32BE(crc32(body));
    return Buffer.concat([length, body, checksum]);
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0); header.writeUInt32BE(height, 4); header[8] = 8; header[9] = 2;
  const rows = Buffer.alloc((width * 3 + 1) * height);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const offset = y * (width * 3 + 1) + 1 + x * 3;
    rows[offset] = 34; rows[offset + 1] = 98 + (x * 60 / width | 0); rows[offset + 2] = 78 + (y * 60 / height | 0);
  }
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", header), chunk("IDAT", deflateSync(rows)), chunk("IEND", Buffer.alloc(0))]);
}

async function unlock(page: Page, url: string, key: string) {
  await page.goto(url);
  await page.getByLabel("Access key", { exact: true }).fill(key);
  await page.getByRole("button", { name: "Unlock files", exact: true }).click();
  await expect(page.getByRole("button", { name: "Select welcome.txt", exact: true })).toBeVisible();
}

const inlineRequests = (page: Page) => {
  const urls: string[] = [];
  page.on("request", (request) => { if (request.url().includes("inline=1")) urls.push(request.url()); });
  return urls;
};

test("refresh reloads a selected text file even when its size and modification time are unchanged", async ({ page, service, request }) => {
  const filename = join(service.root, "same-metadata.txt");
  const timestamp = new Date("2026-01-01T00:00:00Z");
  writeFileSync(filename, "old version\n");
  utimesSync(filename, timestamp, timestamp);
  const requested = inlineRequests(page);
  await unlock(page, service.url, service.key);
  await page.getByRole("button", { name: "Select same-metadata.txt", exact: true }).click();
  const preview = page.getByLabel("Preview of same-metadata.txt", { exact: true });
  await expect(preview).toHaveText("old version\n");
  const headers = { authorization: `Bearer ${service.key}` };
  const initialListing = await request.get(`${service.url}/api/v1/entries?path=`, { headers });
  expect(initialListing.ok()).toBe(true);
  const before = await initialListing.json();

  writeFileSync(filename, "new version\n");
  utimesSync(filename, timestamp, timestamp);
  const updatedListing = await request.get(`${service.url}/api/v1/entries?path=`, { headers });
  expect(updatedListing.ok()).toBe(true);
  const after = await updatedListing.json();
  expect(after.entries.find((entry: { name: string }) => entry.name === "same-metadata.txt"))
    .toEqual(before.entries.find((entry: { name: string }) => entry.name === "same-metadata.txt"));
  await page.getByRole("button", { name: "Refresh folder", exact: true }).click();
  await expect(preview).toHaveText("new version\n");
  expect(requested.filter((url) => url.includes("same-metadata.txt"))).toHaveLength(2);
});

test("refresh reloads decoded image content even when replacement metadata is unchanged", async ({ page, service }) => {
  const filename = join(service.root, "same-image.png");
  const timestamp = new Date("2026-01-01T00:00:00Z");
  const first = png(240, 160), next = png(160, 240);
  const size = Math.max(first.length, next.length);
  writeFileSync(filename, Buffer.concat([first, Buffer.alloc(size - first.length)]));
  utimesSync(filename, timestamp, timestamp);
  await unlock(page, service.url, service.key);
  await page.getByRole("button", { name: "Select same-image.png", exact: true }).click();
  const image = page.getByRole("img", { name: "Preview of same-image.png", exact: true });
  await expect.poll(() => image.evaluate((element: HTMLImageElement) => [element.naturalWidth, element.naturalHeight])).toEqual([240, 160]);
  writeFileSync(filename, Buffer.concat([next, Buffer.alloc(size - next.length)]));
  utimesSync(filename, timestamp, timestamp);
  await page.getByRole("button", { name: "Refresh folder", exact: true }).click();
  await expect.poll(() => image.evaluate((element: HTMLImageElement) => [element.naturalWidth, element.naturalHeight])).toEqual([160, 240]);
});

test("an image previews inside the inspector, fits a 360px viewport and enlarges with focus return", async ({ page, service }) => {
  writeFileSync(join(service.root, "wide-photo.png"), png(1600, 900));
  await page.setViewportSize({ width: 360, height: 780 });
  await unlock(page, service.url, service.key);
  const row = page.getByRole("button", { name: "Select wide-photo.png", exact: true });
  const response = page.waitForResponse((candidate) => candidate.url().includes("wide-photo.png") && candidate.url().includes("inline=1"));
  await row.click();
  const headers = (await response).headers();
  expect(headers["content-type"]).toBe("image/png");
  expect(headers["content-disposition"]).toMatch(/^inline; filename="download"; filename\*=UTF-8''/);
  expect(headers["content-security-policy"]).toBe("sandbox; default-src 'none'");
  expect(headers["x-content-type-options"]).toBe("nosniff");

  const inspector = page.getByRole("region", { name: "Selected entry", exact: true });
  const preview = inspector.getByRole("group", { name: "Preview", exact: true });
  const image = preview.getByRole("img", { name: "Preview of wide-photo.png", exact: true });
  await expect(image).toBeVisible();
  await expect(preview).toHaveAttribute("data-phase", "ready");
  expect(await image.evaluate((element: HTMLImageElement) => ({ width: element.naturalWidth, height: element.naturalHeight }))).toEqual({ width: 1600, height: 900 });
  // Pointer selection reveals the inspector below the list; focus stays on the selecting row.
  await expect(row).toBeFocused();
  await expect(row).toHaveAttribute("aria-pressed", "true");
  const layout = await page.evaluate(() => {
    const image = document.querySelector(".preview img")!.getBoundingClientRect();
    const panel = document.querySelector(".selection-panel")!.getBoundingClientRect();
    return { overflow: document.documentElement.scrollWidth > window.innerWidth, viewport: window.innerWidth, height: window.innerHeight,
      image: image.toJSON(), panelTop: panel.top, panelLeft: panel.left, panelRight: panel.right };
  });
  expect(layout.overflow).toBe(false);
  expect(layout.image.left).toBeGreaterThanOrEqual(layout.panelLeft);
  expect(layout.image.right).toBeLessThanOrEqual(layout.panelRight + 0.5);
  expect(layout.image.width).toBeGreaterThan(200);
  expect(layout.image.height).toBeLessThanOrEqual(320);
  expect(Math.abs(layout.image.width / layout.image.height - 16 / 9)).toBeLessThan(0.02);
  expect(layout.panelTop).toBeGreaterThanOrEqual(0);
  expect(layout.panelTop).toBeLessThan(layout.height);
  await expect(inspector.getByRole("link", { name: "Download selected wide-photo.png", exact: true })).toBeVisible();

  const enlarge = preview.getByRole("button", { name: "View larger", exact: true });
  await enlarge.focus();
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name: "wide-photo.png", exact: true });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Close preview", exact: true })).toBeFocused();
  const large = await dialog.getByRole("img", { name: "Preview of wide-photo.png", exact: true }).evaluate((element) => {
    const rect = element.getBoundingClientRect(); const box = element.closest("dialog")!.getBoundingClientRect();
    return { left: rect.left, right: rect.right, bottom: rect.bottom, dialogLeft: box.left, dialogRight: box.right, dialogBottom: box.bottom, viewport: window.innerWidth, height: window.innerHeight };
  });
  expect(large.left).toBeGreaterThanOrEqual(large.dialogLeft);
  expect(large.right).toBeLessThanOrEqual(large.dialogRight);
  expect(large.dialogRight).toBeLessThanOrEqual(large.viewport);
  expect(large.dialogBottom).toBeLessThanOrEqual(large.height);
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(enlarge).toBeFocused();
  await enlarge.click();
  await page.getByRole("button", { name: "Close preview", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(enlarge).toBeFocused();
});

test("an image over the automatic limit downloads only after an explicit request", async ({ page, service }) => {
  const name = "large-photo.png";
  // A valid small image with trailing padding exercises download size without
  // requiring a huge decoded image or a checked-in binary fixture.
  writeFileSync(join(service.root, name), Buffer.concat([png(2, 2), Buffer.alloc(33 * 1024 * 1024)]));
  const requested = inlineRequests(page);
  await unlock(page, service.url, service.key);
  await page.getByRole("button", { name: `Select ${name}`, exact: true }).click();
  const preview = page.getByRole("group", { name: "Preview", exact: true });
  await expect(preview).toHaveAttribute("data-phase", "waiting");
  await expect(preview.getByRole("button", { name: /^Load preview/ })).toBeVisible();
  await expect(preview.locator("img")).toHaveCount(0);
  expect(requested.filter((url) => url.includes(name))).toHaveLength(0);
  await preview.getByRole("button", { name: /^Load preview/ }).click();
  await expect(preview).toHaveAttribute("data-phase", "ready");
  expect(await preview.getByRole("img").evaluate((image: HTMLImageElement) => image.naturalWidth)).toBe(2);
  expect(requested.filter((url) => url.includes(name))).toHaveLength(1);
  await page.getByRole("button", { name: "Refresh folder", exact: true }).click();
  await expect(preview).toHaveAttribute("data-phase", "waiting");
  expect(requested.filter((url) => url.includes(name))).toHaveLength(1);
});

test("audio and video previews load metadata and play with the real content route", async ({ page, service }) => {
  const samples = 8_000;
  const audio = Buffer.alloc(44 + samples * 2);
  audio.write("RIFF", 0); audio.writeUInt32LE(audio.length - 8, 4); audio.write("WAVEfmt ", 8);
  audio.writeUInt32LE(16, 16); audio.writeUInt16LE(1, 20); audio.writeUInt16LE(1, 22);
  audio.writeUInt32LE(8_000, 24); audio.writeUInt32LE(16_000, 28);
  audio.writeUInt16LE(2, 32); audio.writeUInt16LE(16, 34);
  audio.write("data", 36); audio.writeUInt32LE(samples * 2, 40);
  writeFileSync(join(service.root, "sample.wav"), audio);
  // Chromium makes the short video in memory; no external encoder or video
  // fixture is required, and only metadata/playback state is inspected.
  const video = await page.evaluate(async () => {
    const canvas = document.createElement("canvas"); canvas.width = 320; canvas.height = 180;
    const context = canvas.getContext("2d")!;
    const stream = canvas.captureStream(10);
    const recorder = new MediaRecorder(stream, { mimeType: "video/webm;codecs=vp8" });
    const chunks: Blob[] = [];
    recorder.ondataavailable = (event) => chunks.push(event.data);
    const stopped = new Promise<void>((resolve) => { recorder.onstop = () => resolve(); });
    recorder.start();
    const timer = setInterval(() => { context.fillStyle = "#22624e"; context.fillRect(0, 0, 320, 180); }, 50);
    await new Promise<void>((resolve) => setTimeout(resolve, 400));
    clearInterval(timer); recorder.stop(); await stopped;
    stream.getTracks().forEach((track) => track.stop());
    return Array.from(new Uint8Array(await new Blob(chunks).arrayBuffer()));
  });
  writeFileSync(join(service.root, "sample.webm"), Buffer.from(video));
  await unlock(page, service.url, service.key);
  const preview = page.getByRole("group", { name: "Preview", exact: true });
  for (const [name, tag] of [["sample.wav", "audio"], ["sample.webm", "video"]] as const) {
    const response = page.waitForResponse((candidate) => candidate.url().includes(name) && candidate.url().includes("inline=1"));
    await page.getByRole("button", { name: `Select ${name}`, exact: true }).click();
    expect((await response).headers()["content-type"]).toBe(tag === "audio" ? "audio/wav" : "video/webm");
    await expect(preview).toHaveAttribute("data-phase", "ready");
    const media = preview.locator(tag);
    await expect(media).toHaveAttribute("controls", "");
    await media.evaluate(async (element: HTMLMediaElement) => { element.muted = true; await element.play(); });
    await expect.poll(() => media.evaluate((element: HTMLMediaElement) => element.currentTime)).toBeGreaterThan(0);
    await media.evaluate((element: HTMLMediaElement) => element.pause());
    if (tag === "video") expect(await media.evaluate((element: HTMLVideoElement) => [element.videoWidth, element.videoHeight])).toEqual([320, 180]);
  }
});

test("text previews read one bounded range and state when the content is cut", async ({ page, service }) => {
  const line = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde\n";
  writeFileSync(join(service.root, "long-notes.log"), line.repeat(300 * 1024 / line.length));
  writeFileSync(join(service.root, "资料.md"), "# 标题\n共享设备上的说明。\n");
  writeFileSync(join(service.root, "empty.txt"), "");
  await unlock(page, service.url, service.key);
  const request = page.waitForRequest((candidate) => candidate.url().includes("long-notes.log") && candidate.url().includes("inline=1"));
  const response = page.waitForResponse((candidate) => candidate.url().includes("long-notes.log") && candidate.url().includes("inline=1"));
  const row = page.getByRole("button", { name: "Select long-notes.log", exact: true });
  await row.focus();
  await page.keyboard.press("Enter");
  expect((await request).headers().range).toBe("bytes=0-262143");
  const partial = await response;
  expect(partial.status()).toBe(206);
  expect(partial.headers()["content-type"]).toBe("text/plain; charset=utf-8");
  expect(partial.headers()["content-range"]).toBe("bytes 0-262143/307200");
  expect(partial.headers()["content-length"]).toBe("262144");
  await expect(row).toBeFocused();

  const preview = page.getByRole("group", { name: "Preview", exact: true });
  const text = preview.getByLabel("Preview of long-notes.log", { exact: true });
  await expect(text).toBeVisible();
  expect(await text.evaluate((element) => element.textContent!.length)).toBe(262144);
  await expect(text).toContainText("0123456789abcdef");
  await expect(preview.getByText("Showing the first 256 KiB. Download the file to read all of it.", { exact: true })).toBeVisible();
  const box = await text.evaluate((element) => ({ scrolls: element.scrollHeight > element.clientHeight, height: element.getBoundingClientRect().height,
    overflow: document.documentElement.scrollWidth > window.innerWidth, tag: element.tagName }));
  expect(box).toMatchObject({ scrolls: true, overflow: false, tag: "PRE" });
  expect(box.height).toBeLessThanOrEqual(320);

  await page.getByRole("button", { name: "Select 资料.md", exact: true }).click();
  const short = preview.getByLabel("Preview of 资料.md", { exact: true });
  await expect(short).toHaveText("# 标题\n共享设备上的说明。\n");
  await expect(preview.getByText(/Showing the first/)).toHaveCount(0);
  // Markup is shown as characters, never interpreted.
  expect(await short.evaluate((element) => element.children.length)).toBe(0);

  await page.getByRole("button", { name: "Select empty.txt", exact: true }).click();
  await expect(preview.getByText("This file is empty.", { exact: true })).toBeVisible();
  await expect(preview.locator("pre")).toHaveCount(0);
});

test("unsupported and active document types offer download instead of rendering", async ({ page, service, request }) => {
  writeFileSync(join(service.root, "archive.bin"), Buffer.from([0, 1, 2, 3, 4, 5, 6, 7]));
  writeFileSync(join(service.root, "drawing.svg"), '<svg xmlns="http://www.w3.org/2000/svg" onload="window.__svgRan = true"><script>window.__svgRan = true</script><rect width="10" height="10"/></svg>');
  writeFileSync(join(service.root, "page.html"), "<!doctype html><script>window.__htmlRan = true</script><h1>Active</h1>");
  const requested = inlineRequests(page);
  await unlock(page, service.url, service.key);
  const preview = page.getByRole("group", { name: "Preview", exact: true });
  for (const name of ["archive.bin", "drawing.svg", "page.html"]) {
    await page.getByRole("button", { name: `Select ${name}`, exact: true }).click();
    await expect(preview).toHaveAttribute("data-phase", "unsupported");
    await expect(preview.getByText("No preview for this file type. Download it instead.", { exact: true })).toBeVisible();
    await expect(preview.locator("img, video, audio, pre, iframe, object, embed")).toHaveCount(0);
    await expect(page.getByRole("link", { name: `Download selected ${name}`, exact: true })).toBeVisible();
  }
  expect(requested).toEqual([]);
  expect(await page.evaluate(() => ({ svg: "__svgRan" in window, html: "__htmlRan" in window }))).toEqual({ svg: false, html: false });

  // A crafted inline request for an active type still receives the attachment response.
  for (const name of ["drawing.svg", "page.html"]) {
    const response = await request.get(`${service.url}/api/v1/content?path=${name}&inline=1`, { headers: { authorization: `Bearer ${service.key}` } });
    expect(response.status()).toBe(200);
    expect(response.headers()["content-type"]).toBe("application/octet-stream");
    expect(response.headers()["content-disposition"]).toMatch(/^attachment; /);
    expect(response.headers()["content-security-policy"]).toBeUndefined();
  }
  // The ordinary download link is unchanged by the preview feature.
  const plain = await request.get(`${service.url}/api/v1/content?path=welcome.txt`, { headers: { authorization: `Bearer ${service.key}` } });
  expect(plain.headers()["content-type"]).toBe("application/octet-stream");
  expect(plain.headers()["content-disposition"]).toMatch(/^attachment; /);
});

test("a preview that cannot be decoded or fetched reports a recoverable state", async ({ page, service }) => {
  writeFileSync(join(service.root, "broken.png"), "not an image");
  await unlock(page, service.url, service.key);
  const preview = page.getByRole("group", { name: "Preview", exact: true });
  await page.getByRole("button", { name: "Select broken.png", exact: true }).click();
  await expect(preview.getByText("Could not load the preview. Download the file instead.", { exact: true })).toBeVisible();
  await expect(preview.locator("img")).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Download selected broken.png", exact: true })).toBeVisible();

  let release: () => void = () => undefined;
  const held = new Promise<void>((resolve) => { release = resolve; });
  await page.route("**/api/v1/content?*inline=1", async (route) => { await held; await route.abort("connectionfailed"); });
  await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
  await expect(preview).toHaveAttribute("data-phase", "loading");
  await expect(preview.getByText("Loading preview…", { exact: true })).toBeVisible();
  await expect(preview.locator(".preview-frame")).toHaveAttribute("aria-busy", "true");
  release();
  await expect(preview.getByText("Could not load the preview. Download the file instead.", { exact: true })).toBeVisible();
  await page.unroute("**/api/v1/content?*inline=1");
  // Selecting the entry again retries from a clean state.
  await page.getByRole("button", { name: "Clear selection", exact: true }).click();
  await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
  await expect(preview.getByLabel("Preview of welcome.txt", { exact: true })).toHaveText("Files stay on the sharing device.\n");
});

test("preview strings follow the locale without refetching and folders have no preview", async ({ page, service }) => {
  await unlock(page, service.url, service.key);
  await page.getByRole("button", { name: "New folder", exact: true }).click();
  await page.getByLabel("Folder name", { exact: true }).fill("Projects");
  await page.getByRole("button", { name: "Create folder", exact: true }).click();
  await page.getByRole("button", { name: "Select Projects", exact: true }).click();
  await expect(page.getByRole("region", { name: "Selected entry", exact: true })).toBeVisible();
  await expect(page.getByRole("group", { name: "Preview", exact: true })).toHaveCount(0);

  const requested = inlineRequests(page);
  await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
  await expect(page.getByLabel("Preview of welcome.txt", { exact: true })).toHaveText("Files stay on the sharing device.\n");
  for (const [locale, language, group, label, larger] of [
    ["zh-Hans", "Language", "预览", "welcome.txt 的预览", "放大查看"],
    ["zh-Hant", "语言", "預覽", "welcome.txt 的預覽", "放大檢視"],
    ["en", "語言", "Preview", "Preview of welcome.txt", "View larger"],
  ]) {
    await page.getByLabel(language, { exact: true }).selectOption(locale);
    const preview = page.getByRole("group", { name: group, exact: true });
    await expect(preview.getByLabel(label, { exact: true })).toHaveText("Files stay on the sharing device.\n");
    await expect(preview.getByRole("button", { name: larger, exact: true })).toBeVisible();
  }
  expect(requested).toHaveLength(1);
});

test("servers without the inline capability keep the inspector free of preview requests", async ({ page, service }) => {
  await page.route("**/api/v1/context", async (route) => {
    const response = await route.fetch();
    if (!response.ok()) { await route.fulfill({ response }); return; }
    const context = await response.json();
    const { inline_preview: _removed, ...capabilities } = context.capabilities;
    await route.fulfill({ response, json: { ...context, capabilities } });
  });
  const requested = inlineRequests(page);
  await unlock(page, service.url, service.key);
  await page.getByRole("button", { name: "Select welcome.txt", exact: true }).click();
  await expect(page.getByRole("region", { name: "Selected entry", exact: true })).toBeVisible();
  await expect(page.getByRole("group", { name: "Preview", exact: true })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Download selected welcome.txt", exact: true })).toBeVisible();
  expect(requested).toEqual([]);
});

test.describe("current permissions readout", () => {
  const locales = [
    ["en", "Language", "Current permissions", ["Download: allowed", "Upload: allowed", "Edit: allowed", "Delete: allowed"], "Can edit"],
    ["zh-Hans", "Language", "当前权限", ["下载：允许", "上传：允许", "编辑：允许", "删除：允许"], "可编辑"],
    ["zh-Hant", "语言", "目前權限", ["下載：允許", "上傳：允許", "編輯：允許", "刪除：允許"], "可編輯"],
  ] as const;

  test("is a labelled group stating every permission in words in all three locales", async ({ page, service }) => {
    await unlock(page, service.url, service.key);
    for (const [locale, language, label, entries, summary] of locales) {
      await page.getByLabel(language, { exact: true }).selectOption(locale);
      const group = page.getByRole("banner").getByRole("group", { name: label, exact: true });
      await expect(group).toBeVisible();
      await expect(group.getByText(label, { exact: true })).toBeVisible();
      await expect(group.getByText(summary, { exact: true })).toBeVisible();
      expect(await group.getByRole("listitem").evaluateAll((items) => items.map((item) =>
        item.querySelector(".visually-hidden")!.textContent))).toEqual([...entries]);
      expect(await group.getByRole("listitem").evaluateAll((items) => items.every((item) =>
        item.querySelector("svg")?.getAttribute("aria-hidden") === "true" && item.getAttribute("data-allowed") === "true"))).toBe(true);
    }
  });

  test("aligns with the header controls and wraps inside narrow viewports", async ({ page, service }) => {
    await unlock(page, service.url, service.key);
    for (const [locale, language, label] of locales) {
      await page.getByLabel(language, { exact: true }).selectOption(locale);
      await expect(page.getByRole("group", { name: label, exact: true })).toBeVisible();
      for (const width of [1440, 760, 390, 360, 320]) {
        await page.setViewportSize({ width, height: 860 });
        const layout = await page.evaluate(() => {
          const rect = (selector: string) => document.querySelector(selector)!.getBoundingClientRect();
          const readout = rect(".access-readout"), label = rect(".access-label"), select = rect(".preferences select"), lock = rect(".header-button"), header = rect(".app-header");
          const items = [...document.querySelectorAll(".access-readout li, .access-mode")].map((item) => item.getBoundingClientRect());
          return { overflow: document.documentElement.scrollWidth > window.innerWidth, viewport: window.innerWidth,
            readout: readout.toJSON(), label: label.toJSON(), selectBottom: select.bottom, selectHeight: select.height, lockBottom: lock.bottom, headerBottom: header.bottom,
            contained: items.every((item) => item.left >= readout.left && item.right <= readout.right + 0.5 && item.top >= readout.top && item.bottom <= readout.bottom + 0.5) };
        });
        const context = `${locale}/${width}`;
        expect(layout.overflow, context).toBe(false);
        expect(layout.readout.left, context).toBeGreaterThanOrEqual(0);
        expect(layout.readout.right, context).toBeLessThanOrEqual(layout.viewport);
        if (width > 760) {
          expect(layout.label.bottom, context).toBeLessThanOrEqual(layout.readout.top);
          expect(layout.readout.height, context).toBeGreaterThanOrEqual(40);
        } else {
          // Compact caption row: the label leads on the same line or the line above.
          expect(layout.label.top, context).toBeLessThanOrEqual(layout.readout.top + 4);
          expect(layout.label.left <= layout.readout.left || layout.label.bottom <= layout.readout.top, context).toBe(true);
          expect(layout.readout.top, context).toBeGreaterThanOrEqual(layout.selectBottom);
        }
        expect(layout.readout.bottom, context).toBeLessThanOrEqual(layout.headerBottom);
        expect(layout.contained, context).toBe(true);
        if (width === 1440) {
          // One row: the readout shares the controls' height and baseline edge.
          expect(layout.readout.height, context).toBe(layout.selectHeight);
          expect(Math.abs(layout.readout.bottom - layout.selectBottom), context).toBeLessThanOrEqual(0.5);
          expect(Math.abs(layout.readout.bottom - layout.lockBottom), context).toBeLessThanOrEqual(0.5);
        }
      }
      await page.setViewportSize({ width: 1280, height: 860 });
    }
  });
});

test.describe("current permissions on a read-only share", () => {
  test.use({ serviceOptions: { writable: false, modules: "files" } });
  test("names unavailable actions without relying on color", async ({ page, service }) => {
    await page.goto(service.url);
    await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
    const group = page.getByRole("banner").getByRole("group", { name: "Current permissions", exact: true });
    await expect(group.getByText("Read only", { exact: true })).toBeVisible();
    const items = await group.getByRole("listitem").evaluateAll((elements) => elements.map((item) => ({
      text: item.querySelector(".visually-hidden")!.textContent, allowed: item.getAttribute("data-allowed"),
      icon: item.querySelector("svg")!.getAttribute("class"), struck: getComputedStyle(item.querySelector("span[aria-hidden]")!).textDecorationLine,
    })));
    expect(items.map((item) => item.text)).toEqual(["Download: allowed", "Upload: not allowed", "Edit: not allowed", "Delete: not allowed"]);
    expect(items.map((item) => item.allowed)).toEqual(["true", "false", "false", "false"]);
    expect(new Set(items.map((item) => item.icon)).size).toBe(2);
    expect(items.map((item) => item.struck)).toEqual(["none", "line-through", "line-through", "line-through"]);
  });
});
