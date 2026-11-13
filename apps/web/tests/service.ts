import { test as base } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

const projectRoot = resolve(__dirname, "../../..");
const fixtureKey = "browser-fixture-key-32-characters-minimum";

export interface ServiceOptions {
  writable?: boolean;
  modules?: string;
  maxUploadBytes?: number;
  alternateProduct?: boolean;
}

interface Service {
  url: string;
  root: string;
  key: string;
}

export const test = base.extend<{ service: Service; serviceOptions: ServiceOptions }>({
  serviceOptions: [{}, { option: true }],
  service: async ({ serviceOptions }, use) => {
    const directory = mkdtempSync(join(tmpdir(), "share-browser-"));
    const root = join(directory, "files");
    mkdirSync(root);
    writeFileSync(join(root, "welcome.txt"), "Files stay on the sharing device.\n");
    writeFileSync(join(root, "a-long-file-name-".repeat(9) + "资料.txt"), "A long portable name.\n");
    const keyFile = join(directory, "access.key");
    writeFileSync(keyFile, fixtureKey, { mode: 0o600 });
    const args = ["--root", root, "--listen", "127.0.0.1:0", "--web-dir", join(projectRoot, "apps/web/out")];
    if (serviceOptions.writable !== false) args.push("--writable", "--key-file", keyFile);
    if (serviceOptions.modules) args.push("--modules", serviceOptions.modules);
    if (serviceOptions.maxUploadBytes !== undefined) args.push("--max-upload-bytes", String(serviceOptions.maxUploadBytes));
    if (serviceOptions.alternateProduct) {
      const product = JSON.parse(readFileSync(join(projectRoot, "product.json"), "utf8"));
      const productFile = join(directory, "product.json");
      writeFileSync(productFile, JSON.stringify({ ...product, name: "Harbor", slug: "harbor" }));
      args.push("--product-file", productFile);
    }
    const executable = process.env.SHARE_SERVER_BINARY
      ?? join(projectRoot, "target/debug/share-server" + (process.platform === "win32" ? ".exe" : ""));
    const child = spawn(executable, args, { stdio: ["ignore", "pipe", "pipe"] });
    try {
      const url = await waitForAddress(child);
      await use({ url, root, key: fixtureKey });
    } finally {
      if (child.exitCode === null && child.signalCode === null) {
        child.kill(process.platform === "win32" ? "SIGTERM" : "SIGINT");
        await new Promise<void>((resolveExit) => {
          const timer = setTimeout(() => { child.kill("SIGKILL"); resolveExit(); }, 5_000);
          child.once("exit", () => { clearTimeout(timer); resolveExit(); });
        });
      }
      rmSync(directory, { recursive: true, force: true });
    }
  },
});

function waitForAddress(child: ChildProcess): Promise<string> {
  return new Promise((resolveAddress, reject) => {
    let output = "";
    const timer = setTimeout(() => reject(new Error("Rust service did not announce readiness")), 10_000);
    child.once("error", (error) => { clearTimeout(timer); reject(error); });
    child.once("exit", () => { clearTimeout(timer); reject(new Error("Rust service exited before readiness")); });
    child.stdout?.on("data", (data: Buffer) => {
      output += data.toString();
      const match = output.match(/Listening on (http:\/\/127\.0\.0\.1:\d+)/);
      if (match) { clearTimeout(timer); resolveAddress(match[1]); }
    });
  });
}
