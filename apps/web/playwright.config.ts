import { defineConfig, devices } from "@playwright/test";
import { existsSync } from "node:fs";

const executablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE
  ?? (existsSync("/usr/bin/chromium") ? "/usr/bin/chromium" : undefined);

export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  workers: 1,
  timeout: 45_000,
  expect: { timeout: 8_000 },
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: [["list"], ["json", { outputFile: "../../.local/verification/browser/results.json" }]],
  outputDir: "../../.local/verification/browser/artifacts",
  use: { screenshot: "off", video: "off", trace: "off" },
  projects: [{
    name: "chromium",
    use: { ...devices["Desktop Chrome"], launchOptions: { executablePath } },
  }],
});
