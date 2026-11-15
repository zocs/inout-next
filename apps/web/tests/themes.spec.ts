import { expect } from "@playwright/test";
import { test } from "./service";
import { installNativeBoundary } from "./native-boundary";
for (const native of [false, true]) {
  test(`${native ? "native" : "manager"} preferences wrap and palettes retain readable controls`, async ({ page, service }) => {
    const measurements: unknown[] = [];
    if (native) { await installNativeBoundary(page, service.url); await page.goto("http://tauri.localhost"); }
    else { await page.goto(service.url); await page.getByLabel("Access key", { exact: true }).fill(service.key); await page.getByRole("button", { name: "Unlock files", exact: true }).click(); }
    for (const [locale, language, themeLabel] of [["en", "Language", "Theme"], ["zh-Hans", "Language", "主题"], ["zh-Hant", "语言", "主題"]]) {
      await page.getByLabel(language, { exact: true }).selectOption(locale);
      for (const theme of ["evergreen", "tangerine", "ocean", "berry"]) {
        await page.getByLabel(themeLabel, { exact: true }).selectOption(theme); await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
        for (const width of [320, 390, 1440]) {
          await page.setViewportSize({ width, height: 900 });
          const result = await page.evaluate(() => {
            const luminance = (color: string) => { const rgb = color.match(/[\d.]+/g)!.slice(0, 3).map(Number).map(v => { const x = v / 255; return x <= .04045 ? x / 12.92 : ((x + .055) / 1.055) ** 2.4; }); return .2126 * rgb[0] + .7152 * rgb[1] + .0722 * rgb[2]; };
            const background = (el: Element): string => { const color = getComputedStyle(el).backgroundColor; return color !== "rgba(0, 0, 0, 0)" ? color : el.parentElement ? background(el.parentElement) : "rgb(255, 255, 255)"; };
            return { overflow: document.documentElement.scrollWidth > innerWidth, controls: [...document.querySelectorAll(".preferences select, .button-primary, .search-field input, .field input, .host-heading p, .access-mode")].filter(el => el.getBoundingClientRect().width > 0 && !(el as HTMLButtonElement).disabled).map(el => { const rect = el.getBoundingClientRect(), fg = luminance(getComputedStyle(el).color), bg = luminance(background(el)); const border = luminance(getComputedStyle(el).borderBottomColor); return { name: el.textContent?.slice(0, 45), boundaryContrast: el.matches("input, select") ? (Math.max(border, bg) + .05) / (Math.min(border, bg) + .05) : null, contrast: (Math.max(fg, bg) + .05) / (Math.min(fg, bg) + .05), left: rect.left, right: rect.right }; }) };
          });
          measurements.push({ native, locale, theme, width, ...result });
          expect(result.overflow, `${native}/${locale}/${theme}/${width}`).toBe(false);
          for (const control of result.controls) { expect(control.contrast, `${locale}/${theme}/${width}/${control.name}`).toBeGreaterThanOrEqual(4.5); if (control.boundaryContrast !== null) expect(control.boundaryContrast).toBeGreaterThanOrEqual(3); expect(control.left).toBeGreaterThanOrEqual(0); expect(control.right).toBeLessThanOrEqual(width); }
        }
      }
    }
    await test.info().attach("palette-dom-measurements.json", { body: JSON.stringify(measurements, null, 2), contentType: "application/json" });
  });
}
