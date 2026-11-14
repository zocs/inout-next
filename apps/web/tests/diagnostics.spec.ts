import { expect } from "@playwright/test";
import { test } from "./service";
import { installNativeBoundary } from "./native-boundary";
import product from "../src/generated/product.json";

async function openDiagnostics(page: import("@playwright/test").Page, keyboard = false) {
  const brand = page.getByRole("button", { name: product.name, exact: true });
  for (let index = 0; index < 5; index++) {
    if (keyboard) { await brand.focus(); await brand.press(index % 2 ? "Space" : "Enter"); }
    else await brand.click();
  }
  await expect(page.getByRole("region", { name: "Local diagnostics", exact: true })).toBeVisible();
}

test.describe("native local diagnostics", () => {
  test.use({ serviceOptions: { writable: false, modules: "files" } });

  test("hidden gesture works with keyboard and close restores the native opener", async ({ page, service }) => {
    await installNativeBoundary(page,service.url);
    await page.goto("http://tauri.localhost");
    await expect(page.getByRole("region", { name: "Local diagnostics", exact: true })).toHaveCount(0);
    const brand = page.getByRole("button", { name: product.name, exact: true });
    for (let index=0; index<4; index++) await brand.click();
    await expect(page.getByRole("region", { name: "Local diagnostics", exact: true })).toHaveCount(0);
    await brand.click();
    await expect(page.getByText("Start sharing to use local diagnostics.", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Close diagnostics", exact: true }).click();
    await expect(brand).toBeFocused();
    await openDiagnostics(page,true);
    await expect(page.getByRole("heading", { name: "Local diagnostics", exact: true })).toBeFocused();
    await page.getByRole("button", { name: "Close diagnostics", exact: true }).click();
    await expect(brand).toBeFocused();
  });

  test("buffer apply/reset and local sample states keep sharing controls usable", async ({ page, service }) => {
    const boundary = await installNativeBoundary(page,service.url);
    await page.goto("http://tauri.localhost");
    await page.getByRole("button", { name: "Choose folder", exact: true }).click();
    await page.getByRole("button", { name: "Start sharing", exact: true }).click();
    await openDiagnostics(page);
    await expect(page.getByRole("combobox", { name: "Download buffer", exact: true })).toHaveValue("128");
    await page.getByRole("combobox", { name: "Download buffer", exact: true }).selectOption("32");
    await page.getByRole("button", { name: "Apply buffer", exact: true }).click();
    await expect(page.getByText("Current buffer: 32 KiB", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Restore default buffer", exact: true }).click();
    await expect(page.getByRole("combobox", { name: "Download buffer", exact: true })).toHaveValue("128");
    await page.getByLabel("Relative shared-file path", { exact: true }).fill("folder/资料 & #+%.bin");
    await page.getByRole("combobox", { name: "Test connections", exact: true }).selectOption("4");
    boundary.probeOutcome = "pending";
    await page.getByRole("button", { name: "Run read-only test", exact: true }).click();
    await expect(page.getByRole("button", { name: "Cancel test", exact: true })).toBeEnabled();
    await expect(page.getByRole("combobox", { name: "Download buffer", exact: true })).toBeDisabled();
    await expect(page.getByRole("button", { name: "Stop sharing", exact: true })).toBeEnabled();
    await page.getByRole("button", { name: "Cancel test", exact: true }).click();
    await expect(page.getByText("Local test canceled", { exact: true })).toBeVisible();
    boundary.probeOutcome = "error";
    await page.getByRole("button", { name: "Run read-only test", exact: true }).click();
    await expect(page.getByRole("region", { name: "Local diagnostics", exact: true }).getByRole("alert")).toContainText("Shared file is unavailable");
    boundary.probeOutcome = "success";
    await page.getByRole("button", { name: "Run read-only test", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Local throughput sample", exact: true })).toBeVisible();
    await expect(page.getByText("10.00 MiB/s", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Stop sharing", exact: true }).click();
    await expect(page.getByText("Start sharing to use local diagnostics.", { exact: true })).toBeVisible();
  });

  test("activations expire after five seconds and pending close resets the session", async ({page,service}) => {
    const boundary = await installNativeBoundary(page,service.url);
    await page.clock.install();
    await page.goto("http://tauri.localhost");
    const brand = page.getByRole("button",{name:product.name,exact:true});
    for (let index=0; index<4; index++) await brand.click();
    await page.clock.fastForward(5_001);
    await brand.click();
    await expect(page.getByRole("region",{name:"Local diagnostics",exact:true})).toHaveCount(0);
    await page.getByRole("button",{name:"Choose folder",exact:true}).click();
    await page.getByRole("button",{name:"Start sharing",exact:true}).click();
    // Four more activations complete the new five-activation window.
    for (let index=0; index<4; index++) await brand.click();
    await expect(page.getByRole("region",{name:"Local diagnostics",exact:true})).toBeVisible();
    await page.getByRole("combobox",{name:"Download buffer",exact:true}).selectOption("16");
    await page.getByRole("button",{name:"Apply buffer",exact:true}).click();
    boundary.probeOutcome = "pending";
    await page.getByLabel("Relative shared-file path",{exact:true}).fill("sample.bin");
    await page.getByRole("button",{name:"Run read-only test",exact:true}).click();
    await expect(page.getByRole("button",{name:"Cancel test",exact:true})).toBeVisible();
    await page.getByRole("button",{name:"Close diagnostics",exact:true}).click();
    await expect(brand).toBeFocused();
    await expect(page.getByRole("region",{name:"Local diagnostics",exact:true})).toHaveCount(0);
    const pollsAfterClose = boundary.calls.filter((call) => call.command === "diagnostic_status").length;
    await page.clock.fastForward(5_000);
    expect(boundary.calls.filter((call) => call.command === "diagnostic_status")).toHaveLength(pollsAfterClose);
    await openDiagnostics(page);
    await expect(page.getByRole("combobox",{name:"Download buffer",exact:true})).toHaveValue("128");
    await page.getByLabel("Relative shared-file path",{exact:true}).fill("sample.bin");
    await page.getByRole("button",{name:"Run read-only test",exact:true}).click();
    await expect(page.getByRole("button",{name:"Cancel test",exact:true})).toBeVisible();
    await page.getByRole("button",{name:"Stop sharing",exact:true}).click();
    await expect(page.getByText("Start sharing to use local diagnostics.",{exact:true})).toBeVisible();
    await expect(page.getByRole("button",{name:"Run read-only test",exact:true})).toBeDisabled();
  });

  test("a status reply from a pending apply cannot overwrite the applied buffer", async ({page,service}) => {
    const boundary = await installNativeBoundary(page,service.url);
    await page.clock.install();
    await page.goto("http://tauri.localhost");
    await page.getByRole("button",{name:"Choose folder",exact:true}).click();
    await page.getByRole("button",{name:"Start sharing",exact:true}).click();
    await openDiagnostics(page);
    await page.getByRole("combobox",{name:"Download buffer",exact:true}).selectOption("32");
    boundary.holdBufferApply = true;
    boundary.holdDiagnosticStatus = true;
    await page.getByRole("button",{name:"Apply buffer",exact:true}).click();
    await expect(page.getByRole("button",{name:"Working…",exact:true})).toBeDisabled();
    await page.clock.fastForward(1_000);
    boundary.releaseApply();
    await expect(page.getByText("Current buffer: 32 KiB",{exact:true})).toBeVisible();
    boundary.releaseStatus();
    await page.clock.fastForward(100);
    await expect(page.getByText("Current buffer: 32 KiB",{exact:true})).toBeVisible();
  });

  for (const action of ["buffer", "probe"] as const) {
    for (const delivery of ["stopped", "restarted"] as const) {
      test(`completed ${action} reply delivered while ${delivery} cannot reclaim diagnostic activity`, async ({ page, service }) => {
        const boundary = await installNativeBoundary(page, service.url);
        await page.clock.install({ time: new Date("2030-01-01T00:00:00Z") });
        await page.goto("http://tauri.localhost");
        await page.getByRole("button", { name: "Choose folder", exact: true }).click();
        await page.getByRole("button", { name: "Start sharing", exact: true }).click();
        await openDiagnostics(page);
        await page.clock.pauseAt(new Date("2030-01-01T00:01:00Z"));
        const command = action === "buffer" ? "set_download_buffer" : "run_diagnostic_test";
        const panel = page.getByRole("region", { name: "Local diagnostics", exact: true });
        const buffer = panel.getByRole("combobox", { name: "Download buffer", exact: true });
        const path = panel.getByLabel("Relative shared-file path", { exact: true });
        await buffer.selectOption("32");
        await path.fill("sample.bin");
        boundary.holdCompletedReplies.add(command);
        await panel.getByRole("button", { name: action === "buffer" ? "Apply buffer" : "Run read-only test", exact: true }).click();
        await expect.poll(() => boundary.completedReplyCount(command)).toBe(1);
        await page.getByRole("button", { name: "Stop sharing", exact: true }).click();
        await expect(panel.getByText("Start sharing to use local diagnostics.", { exact: true })).toBeVisible();
        await expect(panel.getByText("Current buffer: 128 KiB", { exact: true })).toBeVisible();
        await expect(buffer).toHaveValue("128");
        await expect(panel.getByRole("heading", { name: "Local throughput sample", exact: true })).toHaveCount(0);
        if (delivery === "restarted") {
          await page.getByRole("button", { name: "Start sharing", exact: true }).click();
          await expect(buffer).toBeEnabled();
          await buffer.selectOption("64");
          await panel.getByRole("button", { name: action === "buffer" ? "Apply buffer" : "Run read-only test", exact: true }).click();
          await expect.poll(() => boundary.completedReplyCount(command)).toBe(2);
        }
        boundary.releaseCompletedReply(command);
        // Drain reply delivery and React work without letting a status poll
        // hide a stale result or a finalizer that unlocked the new action.
        await expect.poll(() => page.evaluate((command) => (window as unknown as { fixtureDeliveredReplies: string[] }).fixtureDeliveredReplies.filter((reply) => reply === command).length, command)).toBe(1);
        await expect(buffer).toHaveValue(delivery === "stopped" ? "128" : "64");
        await expect(buffer).toBeDisabled();
        await expect(panel.getByText("Buffer applied to subsequent requests.", { exact: true })).toHaveCount(0);
        await expect(panel.getByRole("heading", { name: "Local throughput sample", exact: true })).toHaveCount(0);
        await expect(panel.getByRole("alert")).toHaveCount(0);
        if (delivery === "stopped") {
          await expect(panel.getByText("Current buffer: 128 KiB", { exact: true })).toBeVisible();
          await expect(panel.getByRole("button", { name: "Cancel test", exact: true })).toHaveCount(0);
          await page.getByRole("button", { name: "Start sharing", exact: true }).click();
          boundary.holdCompletedReplies.delete(command);
          await expect(buffer).toBeEnabled();
          await buffer.selectOption("64");
          await panel.getByRole("button", { name: "Apply buffer", exact: true }).click();
          await expect(panel.getByText("Current buffer: 64 KiB", { exact: true })).toBeVisible();
          await panel.getByRole("button", { name: "Run read-only test", exact: true }).click();
        } else {
          if (action === "buffer") await expect(panel.getByRole("button", { name: "Working…", exact: true })).toBeDisabled();
          else await expect(panel.getByRole("button", { name: "Cancel test", exact: true })).toBeEnabled();
          await page.clock.fastForward(1_000);
          await expect(buffer).toBeDisabled();
          boundary.releaseCompletedReply(command);
          await expect(buffer).toBeEnabled();
          if (action === "buffer") {
            await expect(panel.getByText("Current buffer: 64 KiB", { exact: true })).toBeVisible();
            await panel.getByRole("button", { name: "Run read-only test", exact: true }).click();
          }
        }
        await expect(panel.getByRole("heading", { name: "Local throughput sample", exact: true })).toBeVisible();
        await expect(panel.getByText("10.00 MiB/s", { exact: true })).toBeVisible();
      });
    }
  }

  for (const outcome of ["error", "pending"] as const) {
    test(`obsolete probe ${outcome === "error" ? "error" : "cancellation notice"} cannot change a restarted action`, async ({ page, service }) => {
      const boundary = await installNativeBoundary(page, service.url);
      await page.clock.install({ time: new Date("2030-01-01T00:00:00Z") });
      await page.goto("http://tauri.localhost");
      await page.getByRole("button", { name: "Choose folder", exact: true }).click();
      await page.getByRole("button", { name: "Start sharing", exact: true }).click();
      await openDiagnostics(page);
      await page.clock.pauseAt(new Date("2030-01-01T00:01:00Z"));
      const panel = page.getByRole("region", { name: "Local diagnostics", exact: true });
      await panel.getByLabel("Relative shared-file path", { exact: true }).fill("sample.bin");
      boundary.probeOutcome = outcome;
      boundary.holdCompletedReplies.add("run_diagnostic_test");
      await panel.getByRole("button", { name: "Run read-only test", exact: true }).click();
      await expect(panel.getByRole("button", { name: "Cancel test", exact: true })).toBeVisible();
      await page.getByRole("button", { name: "Stop sharing", exact: true }).click();
      await expect.poll(() => boundary.completedReplyCount("run_diagnostic_test")).toBe(1);
      await page.getByRole("button", { name: "Start sharing", exact: true }).click();
      await expect(panel.getByRole("combobox", { name: "Download buffer", exact: true })).toBeEnabled();
      boundary.holdCompletedReplies.add("set_download_buffer");
      await panel.getByRole("combobox", { name: "Download buffer", exact: true }).selectOption("64");
      await panel.getByRole("button", { name: "Apply buffer", exact: true }).click();
      await expect.poll(() => boundary.completedReplyCount("set_download_buffer")).toBe(1);
      boundary.releaseCompletedReply("run_diagnostic_test");
      await expect.poll(() => page.evaluate(() => (window as unknown as { fixtureDeliveredReplies: string[] }).fixtureDeliveredReplies.filter((reply) => reply === "run_diagnostic_test").length)).toBe(1);
      await expect(panel.getByRole("alert")).toHaveCount(0);
      await expect(panel.getByText("Local test canceled", { exact: true })).toHaveCount(0);
      await expect(panel.getByRole("button", { name: "Working…", exact: true })).toBeDisabled();
      boundary.releaseCompletedReply("set_download_buffer");
      await expect(panel.getByText("Current buffer: 64 KiB", { exact: true })).toBeVisible();
      await expect(panel.getByRole("combobox", { name: "Download buffer", exact: true })).toBeEnabled();
    });
  }

  for (const width of [320,390,1440]) {
    test(`long path and results fit ${width}px without page overflow`, async ({ page,service }) => {
      await page.setViewportSize({width,height:900});
      await installNativeBoundary(page,service.url);
      await page.goto("http://tauri.localhost");
      await page.getByRole("button", {name:"Choose folder",exact:true}).click();
      await page.getByRole("button", {name:"Start sharing",exact:true}).click();
      await openDiagnostics(page);
      await page.getByLabel("Relative shared-file path",{exact:true}).fill("long-folder/".repeat(30)+"资料 & #+%.bin");
      await page.getByRole("button",{name:"Run read-only test",exact:true}).click();
      await expect(page.getByRole("heading",{name:"Local throughput sample",exact:true})).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      for (const control of await page.getByRole("region",{name:"Local diagnostics",exact:true}).locator("button,input,select").all()) {
        const box = await control.boundingBox();
        expect(box?.width).toBeGreaterThan(0);
        expect(box?.x).toBeGreaterThanOrEqual(0);
        expect((box?.x ?? 0)+(box?.width ?? 0)).toBeLessThanOrEqual(width);
      }
    });
  }

  test("remote manager has no diagnostic activation even with injected IPC", async ({page,service}) => {
    await page.addInitScript(() => Object.assign(window,{__TAURI_INTERNALS__:{invoke:()=>{throw new Error("Remote manager cannot use native controls");}}}));
    await page.goto(service.url);
    await expect(page.getByRole("table",{name:"Shared files"})).toBeVisible();
    await expect(page.getByRole("button",{name:product.name,exact:true})).toHaveCount(0);
    await expect(page.getByRole("region",{name:"Local diagnostics",exact:true})).toHaveCount(0);
  });
});
