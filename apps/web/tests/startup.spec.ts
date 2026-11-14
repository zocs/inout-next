import { expect, type Page } from "@playwright/test";
import { test } from "./service";
import { installNativeBoundary } from "./native-boundary";

async function freezeClock(page: Page) {
  // Freeze before navigation, while no app timers exist. Avoid comparing an
  // already ticking browser clock against a separate Node wall clock.
  await page.clock.install({ time: new Date("2030-01-01T00:00:00Z") });
  await page.clock.pauseAt(new Date("2030-01-01T00:01:00Z"));
}

async function holdContext(page: Page) {
  let release!: () => void;
  const pending = new Promise<void>((resolve) => { release = resolve; });
  let arrived!: () => void;
  const requested = new Promise<void>((resolve) => { arrived = resolve; });
  await page.route("**/api/v1/context", async (route) => {
    arrived(); await pending; await route.continue();
  });
  return { release, requested };
}

async function observeStartupResources(page: Page) {
  await page.addInitScript(() => {
    const keys = new Set<EventListenerOrEventListenerObject>();
    const mediaListeners = new Set<EventListenerOrEventListenerObject>();
    const timeouts = new Set<number>();
    const add = window.addEventListener.bind(window);
    const remove = window.removeEventListener.bind(window);
    window.addEventListener = ((type: string, listener: EventListenerOrEventListenerObject, options?: boolean | AddEventListenerOptions) => {
      if (type === "keydown") keys.add(listener);
      add(type, listener, options);
    }) as typeof window.addEventListener;
    window.removeEventListener = ((type: string, listener: EventListenerOrEventListenerObject, options?: boolean | EventListenerOptions) => {
      if (type === "keydown") keys.delete(listener);
      remove(type, listener, options);
    }) as typeof window.removeEventListener;
    const match = window.matchMedia.bind(window);
    window.matchMedia = (query: string) => {
      const media = match(query);
      if (query === "(prefers-reduced-motion: reduce)") {
        const add = media.addEventListener.bind(media);
        const remove = media.removeEventListener.bind(media);
        media.addEventListener = ((type: string, listener: EventListenerOrEventListenerObject, options?: boolean | AddEventListenerOptions) => {
          if (type === "change") mediaListeners.add(listener);
          add(type, listener, options);
        }) as typeof media.addEventListener;
        media.removeEventListener = ((type: string, listener: EventListenerOrEventListenerObject, options?: boolean | EventListenerOptions) => {
          if (type === "change") mediaListeners.delete(listener);
          remove(type, listener, options);
        }) as typeof media.removeEventListener;
      }
      return media;
    };
    const schedule = window.setTimeout.bind(window);
    const cancel = window.clearTimeout.bind(window);
    window.setTimeout = ((handler: TimerHandler, delay?: number, ...args: unknown[]) => {
      let id: number;
      const callback = typeof handler === "function" ? () => {
        timeouts.delete(id);
        handler.apply(window, args);
      } : handler;
      id = schedule(callback, delay, ...args);
      // Track finite presentation work; Next's immediate scheduling is not an
      // owned startup delay, and native polling uses setInterval separately.
      if (typeof handler === "function" && delay && delay <= 500) timeouts.add(id);
      return id;
    }) as typeof window.setTimeout;
    window.clearTimeout = (id?: number) => { if (id !== undefined) timeouts.delete(id); cancel(id); };
    Object.assign(window, { startupResources: () => ({ keys: keys.size, media: mediaListeners.size, timeouts: timeouts.size }) });
  });
}

async function resources(page: Page) {
  return page.evaluate(() => (window as unknown as { startupResources: () => { keys: number; media: number; timeouts: number } }).startupResources());
}

async function observeExit(page: Page) {
  await page.evaluate(() => {
    const screen = document.querySelector(".startup-screen")!;
    const capture = new Promise((resolve) => {
      const observer = new MutationObserver(() => {
        if (screen.getAttribute("data-state") !== "leaving") return;
        observer.disconnect();
        const rect = screen.querySelector(".wordmark")!.getBoundingClientRect();
        // Flush once at the state change to inspect the actual browser effects.
        const opacity = getComputedStyle(screen).opacity;
        resolve({
          name: screen.querySelector(".wordmark")!.textContent,
          matrix: screen.querySelectorAll(".wordmark svg").length,
          mark: rect.toJSON(), viewport: window.innerWidth, opacity,
          effects: screen.getAnimations({ subtree: true }).map((animation) => {
            const effect = animation.effect as KeyframeEffect;
            return { duration: effect.getTiming().duration, iterations: effect.getTiming().iterations,
              properties: [...new Set(effect.getKeyframes().flatMap((frame) => Object.keys(frame).filter((key) => !["offset", "computedOffset", "easing", "composite"].includes(key))))] };
          }),
        });
      });
      observer.observe(screen, { attributes: true, attributeFilter: ["data-state"] });
    });
    Object.assign(window, { startupExit: capture });
  });
}

async function exitEvidence(page: Page) {
  return page.evaluate(() => (window as unknown as { startupExit: Promise<{
    name: string; matrix: number; mark: { left: number; right: number; width: number; height: number };
    viewport: number; opacity: string;
    effects: { duration: number; iterations: number; properties: string[] }[];
  }> }).startupExit);
}

function expectCompositedExit(evidence: Awaited<ReturnType<typeof exitEvidence>>) {
  expect(evidence.mark.left).toBeGreaterThanOrEqual(0);
  expect(evidence.mark.right).toBeLessThanOrEqual(evidence.viewport);
  expect(evidence.effects.length).toBeGreaterThan(0);
  expect(evidence.effects.length).toBeLessThanOrEqual(2);
  for (const effect of evidence.effects) {
    expect(effect.duration).toBeLessThanOrEqual(300);
    expect(effect.iterations).toBe(1);
    for (const property of effect.properties) expect(["opacity", "transform"]).toContain(property);
  }
}

test.describe("readiness-driven web startup", () => {
  test.use({ reducedMotion: "no-preference", serviceOptions: { writable: false, modules: "files", productName: "inout" } });

  test("real web context releases controls before the bounded exit finishes", async ({ page, service }) => {
    const gate = await holdContext(page);
    await freezeClock(page);
    await observeStartupResources(page);
    await page.goto(service.url);
    await gate.requested;
    const startup = page.getByRole("status", { name: "Opening inout", exact: true });
    await expect(startup).toBeVisible();
    expect(await startup.evaluate((element) => element.getAnimations({ subtree: true }).length)).toBe(0);
    gate.release();
    await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
    await expect(page.locator(".startup-screen")).toHaveAttribute("aria-hidden", "true");
    expect(await page.locator(".startup-screen").evaluate((element) => getComputedStyle(element).pointerEvents)).toBe("none");
    expect((await resources(page)).keys).toBe(0);
    const refresh = page.getByRole("button", { name: "Refresh folder", exact: true });
    await expect(refresh).toBeEnabled();
    await refresh.click();
    await expect(page.locator(".startup-screen")).toHaveCount(1);
    await page.clock.fastForward(500);
    await expect(page.locator(".startup-screen")).toHaveCount(0);
    expect(await resources(page)).toEqual({ keys: 0, media: 0, timeouts: 0 });
  });

  for (const { token, duration } of [{ token: "360ms", duration: 360 }, { token: "0.04s", duration: 40 }]) {
    test(`CSS motion override ${token} owns actual startup effects and event-independent removal`, async ({ page, service }) => {
      const gate = await holdContext(page);
      await freezeClock(page);
      await observeStartupResources(page);
      await page.goto(service.url);
      await gate.requested;
      await expect(page.getByRole("status", { name: "Opening inout", exact: true })).toBeVisible();
      await page.evaluate((value) => {
        document.documentElement.style.setProperty("--duration-state", value);
        // Cleanup must survive lost transition events, rather than depending
        // on an event handler firing after the compositor finishes.
        document.addEventListener("transitionend", (event) => event.stopImmediatePropagation(), true);
        document.addEventListener("transitioncancel", (event) => event.stopImmediatePropagation(), true);
      }, token);
      await observeExit(page);
      gate.release();
      await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
      const startup = page.locator(".startup-screen");
      await expect(startup).toHaveAttribute("aria-hidden", "true");
      expect(await startup.evaluate((element) => getComputedStyle(element).pointerEvents)).toBe("none");
      await page.getByRole("button", { name: "Refresh folder", exact: true }).click();
      const evidence = await exitEvidence(page);
      expect(evidence.effects).toHaveLength(2);
      for (const effect of evidence.effects) {
        expect(effect.duration).toBe(duration);
        expect(effect.iterations).toBe(1);
        for (const property of effect.properties) expect(["opacity", "transform"]).toContain(property);
      }
      await test.info().attach(`startup-override-${duration}.json`, { body: Buffer.from(JSON.stringify(evidence, null, 2)), contentType: "application/json" });
      await page.clock.fastForward(duration - 1);
      await expect(startup).toHaveCount(1);
      await page.clock.fastForward(1);
      await expect(startup).toHaveCount(0);
      expect(await resources(page)).toEqual({ keys: 0, media: 0, timeouts: 0 });
    });
  }

  test("a real connection failure releases recovery without waiting for motion", async ({ page, service }) => {
    const gate = await holdContext(page);
    await freezeClock(page);
    await page.goto(service.url);
    await gate.requested;
    await expect(page.getByRole("status", { name: "Opening inout", exact: true })).toBeVisible();
    // Shut down the actual server after the exported app loads. The API fails
    // through the real network boundary, rather than a fabricated API response.
    await service.stop();
    gate.release();
    await expect(page.getByRole("heading", { name: "Cannot connect", exact: true })).toBeVisible();
    await expect(page.locator(".connection-state").getByRole("alert")).toContainText("Cannot reach the sharing device");
    await expect(page.getByRole("button", { name: "Try again", exact: true })).toBeEnabled();
    await expect(page.locator(".startup-screen")).toHaveAttribute("aria-hidden", "true");
    expect(await page.locator(".startup-screen").evaluate((element) => getComputedStyle(element).pointerEvents)).toBe("none");
  });

  test("Escape dismisses presentation while the real connection continues", async ({ page, service }) => {
    const gate = await holdContext(page);
    await freezeClock(page);
    await observeStartupResources(page);
    await page.goto(service.url);
    await gate.requested;
    await expect(page.getByRole("status", { name: "Opening inout", exact: true })).toBeVisible();
    expect((await resources(page)).keys).toBe(1);
    const escaped = await page.evaluate(() => {
      const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
      window.dispatchEvent(event);
      return { consumed: event.defaultPrevented };
    });
    expect(escaped.consumed).toBe(false);
    await expect(page.locator(".startup-screen")).toHaveAttribute("aria-hidden", "true");
    expect((await resources(page)).keys).toBe(0);
    await expect(page.getByRole("heading", { name: "Opening shared files", exact: true })).toBeVisible();
    gate.release();
    await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
    await page.clock.fastForward(500);
    await expect(page.locator(".startup-screen")).toHaveCount(0);
    expect(await resources(page)).toEqual({ keys: 0, media: 0, timeouts: 0 });
  });
});

test.describe("authentication is first readiness", () => {
  test.use({ reducedMotion: "no-preference", serviceOptions: { modules: "files", productName: "inout" } });

  test("real authentication challenge releases the key field immediately", async ({ page, service }) => {
    const gate = await holdContext(page);
    await freezeClock(page);
    await page.goto(service.url);
    await gate.requested;
    gate.release();
    await expect(page.getByRole("heading", { name: "Unlock shared files", exact: true })).toBeVisible();
    await expect(page.locator(".startup-screen")).toHaveAttribute("aria-hidden", "true");
    await page.getByLabel("Access key", { exact: true }).fill(service.key);
    await page.getByRole("button", { name: "Unlock files", exact: true }).click();
    await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
    await expect(page.locator(".startup-screen")).toHaveCount(1);
  });
});

test.describe("long alternate startup brand", () => {
  const name = "NeighborhoodFileStationNeighborhoodFileStationNeighborhoodFileStationStationFile";
  test.use({ reducedMotion: "no-preference", serviceOptions: { writable: false, modules: "files", productName: name } });
  test("real runtime fallback remains readable during the mobile exit", async ({ page, service }) => {
    await page.setViewportSize({ width: 320, height: 900 });
    const gate = await holdContext(page);
    await freezeClock(page);
    await page.goto(service.url);
    await gate.requested;
    await observeExit(page);
    gate.release();
    await expect(page.getByRole("table", { name: "Shared files" })).toBeVisible();
    const evidence = await exitEvidence(page);
    expect(evidence.name).toBe(name);
    expect(evidence.matrix).toBe(0);
    expectCompositedExit(evidence);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await test.info().attach("alternate-startup-motion.json", {
      body: Buffer.from(JSON.stringify(evidence, null, 2)), contentType: "application/json",
    });
    await page.clock.fastForward(500);
    await expect(page.locator(".startup-screen")).toHaveCount(0);
  });
});

test.describe("native first readiness and motion", () => {
  test.use({ reducedMotion: "no-preference", serviceOptions: { writable: false, modules: "files", productName: "inout" } });

  for (const width of [320, 390, 1440]) {
  test(`stopped native setup is ready and motion cannot delay choosing a folder at ${width}px`, async ({ page, service }) => {
    await page.setViewportSize({ width, height: 900 });
    const boundary = await installNativeBoundary(page, service.url, { holdInitialStatus: true });
    await freezeClock(page);
    await page.goto("http://tauri.localhost");
    await expect(page.getByRole("status", { name: "Opening inout", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Choose folder", exact: true })).toHaveCount(0);
    await observeExit(page);
    boundary.releaseInitialStatus();
    await expect(page.getByRole("button", { name: "Choose folder", exact: true })).toBeEnabled();
    await expect(page.locator(".startup-screen")).toHaveAttribute("aria-hidden", "true");
    await page.getByRole("button", { name: "Choose folder", exact: true }).click();
    await page.getByRole("button", { name: "Start sharing", exact: true }).click();
    await expect(page.getByRole("button", { name: "Stop sharing", exact: true })).toBeEnabled();
    await expect(page.locator(".startup-screen")).toHaveCount(1);
    const evidence = await exitEvidence(page);
    expect(evidence.name).toBe("inout");
    expectCompositedExit(evidence);
    await test.info().attach("startup-motion.json", {
      body: Buffer.from(JSON.stringify(evidence, null, 2)), contentType: "application/json",
    });
    await page.clock.fastForward(500);
    await expect(page.locator(".startup-screen")).toHaveCount(0);
    await page.getByRole("button", { name: "Stop sharing", exact: true }).click();
    await expect(page.locator(".startup-screen")).toHaveCount(0);
  });
  }

  test("initial native failure releases an actionable recovery screen", async ({ page, service }) => {
    const boundary = await installNativeBoundary(page, service.url, { holdInitialStatus: true, initialStatusError: "Native controls are unavailable" });
    await freezeClock(page);
    await page.goto("http://tauri.localhost");
    await expect(page.getByRole("status", { name: "Opening inout", exact: true })).toBeVisible();
    boundary.releaseInitialStatus();
    await expect(page.locator(".host-main").getByRole("alert")).toContainText("Native controls are unavailable");
    await expect(page.locator(".startup-screen")).toHaveAttribute("aria-hidden", "true");
    await page.getByRole("button", { name: "Try again", exact: true }).click();
    await expect(page.getByRole("button", { name: "Choose folder", exact: true })).toBeEnabled();
    await expect(page.locator(".host-main").getByRole("alert")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Choose folder", exact: true })).toBeFocused();
  });

  test("native recovery preserves focus chosen while its reply is pending", async ({ page, service }) => {
    const boundary = await installNativeBoundary(page, service.url, { initialStatusError: "Native controls are unavailable" });
    await freezeClock(page);
    await page.goto("http://tauri.localhost");
    await expect(page.getByRole("button", { name: "Try again", exact: true })).toBeEnabled();
    boundary.holdHostStatus = true;
    await page.getByRole("button", { name: "Try again", exact: true }).click();
    await expect(page.getByRole("button", { name: "Connecting…", exact: true })).toBeDisabled();
    const brand = page.getByRole("button", { name: "inout", exact: true });
    await brand.focus();
    boundary.holdHostStatus = false;
    boundary.releaseHostStatus();
    await expect(page.getByRole("button", { name: "Choose folder", exact: true })).toBeEnabled();
    await expect(brand).toBeFocused();
    await page.clock.fastForward(500);
    await expect(page.locator(".startup-screen")).toHaveCount(0);
  });

  test("initial reduced motion removes presentation immediately at readiness", async ({ page, service }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    const boundary = await installNativeBoundary(page, service.url, { holdInitialStatus: true });
    await freezeClock(page);
    await observeStartupResources(page);
    await page.goto("http://tauri.localhost");
    const startup = page.getByRole("status", { name: "Opening inout", exact: true });
    await expect(startup).toBeVisible();
    expect(await startup.evaluate((element) => getComputedStyle(element).transitionDuration)).toBe("0s");
    expect(await startup.locator(".startup-wordmark").evaluate((element) => getComputedStyle(element).transform)).toBe("none");
    boundary.releaseInitialStatus();
    await expect(page.getByRole("button", { name: "Choose folder", exact: true })).toBeEnabled();
    await expect(page.locator(".startup-screen")).toHaveCount(0);
    expect(await resources(page)).toEqual({ keys: 0, media: 0, timeouts: 0 });
  });

  test("live reduced motion cancels the exit and releases its timer and listeners", async ({ page, service }) => {
    await page.emulateMedia({ reducedMotion: "no-preference" });
    const boundary = await installNativeBoundary(page, service.url, { holdInitialStatus: true });
    await freezeClock(page);
    await observeStartupResources(page);
    await page.goto("http://tauri.localhost");
    await expect(page.getByRole("status", { name: "Opening inout", exact: true })).toBeVisible();
    boundary.releaseInitialStatus();
    await expect(page.locator(".startup-screen")).toHaveAttribute("aria-hidden", "true");
    expect((await resources(page)).keys).toBe(0);
    expect((await resources(page)).media).toBe(1);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expect(page.locator(".startup-screen")).toHaveCount(0);
    expect(await resources(page)).toEqual({ keys: 0, media: 0, timeouts: 0 });
    await page.clock.fastForward(5_000);
    await expect(page.locator(".startup-screen")).toHaveCount(0);
    await page.getByRole("button", { name: "Choose folder", exact: true }).click();
    await expect(page.getByRole("button", { name: "Start sharing", exact: true })).toBeEnabled();
  });
});
