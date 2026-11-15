import { expect } from "@playwright/test";
import { test } from "./service";
import { installNativeBoundary } from "./native-boundary";

const longBrand = "NeighborhoodFileStationNeighborhoodFileStationNeighborhoodFileStationStationFile";

test.describe("default brand consumer", () => {
  test.use({ serviceOptions: { writable: false, modules: "files" } });
  test("the built server and exported app agree on the current name and identity", async ({ page, request, service }) => {
    const response = await request.get(service.url + "/api/v1/status");
    expect(response.ok()).toBe(true);
    expect((await response.json()).product).toMatchObject({ name: "inout", slug: "inout-next", application_id: "io.github.zocs.inoutnext" });
    await page.goto(service.url);
    await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
    await expect(page).toHaveTitle("inout");
    await expect(page.getByRole("heading", { name: "inout", exact: true }).locator(".wordmark svg")).toBeVisible();
  });
});

for (const { name, matrix } of [
  { name: "inout", matrix: true },
  { name: "Harbor", matrix: true },
  { name: "atlas2", matrix: true },
  { name: longBrand, matrix: false },
  { name: "文件驿站", matrix: false },
  { name: "Harbor & homes", matrix: false },
]) {
  test.describe(`product wordmark ${name}`, () => {
    // A real product file exercises the runtime consumer without replacing
    // the API.
    test.use({ serviceOptions: { modules: "files", productName: name } });

    for (const width of [320, 390, 1440]) {
      test(`accessible name and ${matrix ? "matrix" : "text fallback"} fit ${width}px`, async ({ page, service }) => {
        await page.setViewportSize({ width, height: 900 });
        await page.goto(service.url);
        const heading = page.getByRole("heading", { name, exact: true });
        await expect(heading).toBeVisible();
        await expect(page).toHaveTitle(name);
        await page.getByLabel("Access key", { exact: true }).fill(service.key);
        await page.getByRole("button", { name: "Unlock files", exact: true }).click();
        await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
        await expect(page.getByRole("button", { name: "Lock files", exact: true })).toBeEnabled();
        await expect(page.getByRole("button", { name, exact: true })).toHaveCount(0);
        const mark = heading.locator(".wordmark");
        await expect(mark).toHaveText(name);
        await expect(mark.locator("svg")).toHaveCount(matrix ? 1 : 0);
        if (matrix) {
          await expect(mark.locator("svg")).toHaveAttribute("aria-hidden", "true");
          expect(await mark.locator("circle").count()).toBeGreaterThan(20);
        }
        await expect(page.locator(".app-footer")).toContainText(name);
        const layout = await page.evaluate(() => {
          const heading = document.querySelector(".brand h1")!.getBoundingClientRect();
          const brand = document.querySelector(".brand")!.getBoundingClientRect();
          const actions = document.querySelector(".header-actions")!.getBoundingClientRect();
          const footer = document.querySelector(".app-footer")!;
          const identity = footer.firstElementChild!.getBoundingClientRect();
          const description = footer.lastElementChild!.getBoundingClientRect();
          const matrix = document.querySelector(".brand .wordmark svg")?.getBoundingClientRect();
          return {
            viewport: window.innerWidth,
            pageWidth: document.documentElement.scrollWidth,
            heading: heading.toJSON(), brand: brand.toJSON(), actions: actions.toJSON(),
            identity: identity.toJSON(), description: description.toJSON(),
            matrix: matrix?.toJSON(),
          };
        });
        expect(layout.pageWidth).toBeLessThanOrEqual(width);
        expect(layout.heading.right).toBeLessThanOrEqual(layout.brand.right + 1);
        // Appearance controls can wrap beneath the brand on narrow screens.
        // Require real separation on the rendered axis in both compositions.
        const sameRow = layout.brand.top < layout.actions.bottom && layout.brand.bottom > layout.actions.top;
        if (sameRow) expect(layout.brand.right + 15).toBeLessThanOrEqual(layout.actions.left);
        else expect(layout.brand.bottom + 15).toBeLessThanOrEqual(layout.actions.top);
        expect(layout.actions.right).toBeLessThanOrEqual(width);
        expect(layout.identity.right).toBeLessThanOrEqual(layout.description.left);
        if (matrix) {
          expect(layout.matrix!.right).toBeLessThanOrEqual(layout.heading.right + 1);
          expect(layout.matrix!.height).toBeGreaterThanOrEqual(14);
        }
        await test.info().attach("wordmark-layout.json", {
          body: Buffer.from(JSON.stringify({ name, matrix, ...layout }, null, 2)),
          contentType: "application/json",
        });
      });
    }
  });
}

test.describe("generated startup brand", () => {
  test.use({ serviceOptions: { writable: false, modules: "files", productName: "inout" } });

  test("initial exported markup identifies the product without scripts or animation", async ({ browser, service }) => {
    const context = await browser.newContext({ javaScriptEnabled: false });
    const page = await context.newPage();
    try {
      await page.goto(service.url);
      await expect(page).toHaveTitle("inout");
      const startup = page.getByRole("status", { name: "Opening inout", exact: true });
      await expect(startup).toBeVisible();
      await expect(startup.locator(".wordmark")).toHaveText("inout");
      await expect(startup.locator("svg")).toHaveAttribute("aria-hidden", "true");
      expect(await startup.evaluate((element) => getComputedStyle(element).opacity)).toBe("1");
      expect(await startup.evaluate((element) => element.getAnimations({ subtree: true }).length)).toBe(0);
    } finally { await context.close(); }
  });

  test("the rendered native matrix retains keyboard diagnosis and returns focus", async ({ page, service }) => {
    await installNativeBoundary(page, service.url);
    await page.goto("http://tauri.localhost");
    const brand = page.getByRole("button", { name: "inout", exact: true });
    await expect(brand.locator(".wordmark svg")).toBeVisible();
    for (let index = 0; index < 4; index++) {
      await brand.focus();
      await brand.press(index % 2 ? "Space" : "Enter");
    }
    await expect(page.getByRole("region", { name: "Local diagnostics", exact: true })).toHaveCount(0);
    await brand.press("Enter");
    await expect(page.getByRole("heading", { name: "Local diagnostics", exact: true })).toBeFocused();
    const close = page.getByRole("button", { name: "Close diagnostics", exact: true });
    await close.focus();
    await close.press("Enter");
    await expect(brand).toBeFocused();
    expect(await brand.evaluate((element) => getComputedStyle(element).outlineStyle)).toBe("solid");
  });
});
