"use client";

import { invoke } from "@tauri-apps/api/core";
import { Eye, EyeOff, FolderOpen, Play, Square } from "lucide-react";
import { lazy, Suspense, useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import product from "@/generated/product.json";
import { Header } from "./file-manager";
import { Wordmark } from "./branding";

const DiagnosticPanel = lazy(() => import("@/features/diagnostics/panel"));

interface HostStatus {
  root: string | null;
  running: boolean;
  managerUrl: string | null;
  addresses: string[];
  activeSettings: {
    network: boolean;
    port: number;
    uploads: boolean;
    manage: boolean;
    authRequired: boolean;
  } | null;
  capabilities?: {
    storageKind: "folder" | "app-private";
    chooseFolder: boolean;
    openManager: boolean;
    foregroundOnly: boolean;
  };
}

const desktopCapabilities = {
  storageKind: "folder", chooseFolder: true, openManager: true, foregroundOnly: false,
} as const;

function nativeError(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : "The native action failed. Try again.";
}

export default function NativeHost({ onReady }: { onReady?: () => void }) {
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
  const activations = useRef<number[]>([]);
  const diagnosticOrigin = useRef<HTMLButtonElement | null>(null);
  const activateBrand = useCallback((origin: HTMLButtonElement) => {
    const now = performance.now();
    activations.current = activations.current.filter((time) => now - time <= 5_000);
    activations.current.push(now);
    if (activations.current.length >= 5) {
      activations.current = []; diagnosticOrigin.current = origin; setDiagnosticsOpen(true);
    }
  }, []);
  const closeDiagnostics = useCallback(() => {
    setDiagnosticsOpen(false);
    diagnosticOrigin.current?.focus();
  }, []);
  const [status, setStatus] = useState<HostStatus | null>(null);
  const [network, setNetwork] = useState(false);
  const [port, setPort] = useState("5000");
  const [uploads, setUploads] = useState(false);
  const [manage, setManage] = useState(false);
  const [key, setKey] = useState("");
  const [activeKey, setActiveKey] = useState<{ managerUrl: string | null; value: string } | null>(null);
  const [showKey, setShowKey] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const readinessReported = useRef(false);
  const inFlight = useRef(false);
  const revision = useRef(0);
  const mounted = useRef(true);
  const folderControl = useRef<HTMLButtonElement>(null);
  const retryControl = useRef<HTMLButtonElement>(null);
  const startControl = useRef<HTMLButtonElement>(null);
  const stopControl = useRef<HTMLButtonElement>(null);
  const focusAfterChange = useRef<"start" | "stop" | null>(null);
  const retryFocus = useRef<{ origin: HTMLButtonElement; destination: "folder" | "start" | "stop" | "retry" } | null>(null);

  useEffect(() => {
    // A stopped host (including no selected folder) is usable setup. An initial
    // IPC failure must expose recovery rather than hold the startup screen.
    if ((!status && !error) || readinessReported.current) return;
    readinessReported.current = true;
    onReady?.();
  }, [status, error, onReady]);

  const refresh = useCallback(async () => {
    if (inFlight.current) return;
    const current = revision.current;
    try {
      const next = await invoke<HostStatus>("host_status");
      if (mounted.current && current === revision.current) {
        setStatus(next);
        if (!next.running) setActiveKey(null);
      }
    } catch (error) {
      if (mounted.current && current === revision.current) setError(nativeError(error));
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    const timer = window.setInterval(() => void refresh(), 2_000);
    const focused = () => void refresh();
    window.addEventListener("focus", focused);
    return () => { mounted.current = false; window.clearInterval(timer); window.removeEventListener("focus", focused); };
  }, [refresh]);

  useEffect(() => {
    const target = focusAfterChange.current;
    if (busy || !target || (target === "stop") !== !!status?.running) return;
    (target === "stop" ? stopControl : startControl).current?.focus();
    focusAfterChange.current = null;
  }, [busy, status?.running]);

  useEffect(() => {
    const task = retryFocus.current;
    if (busy || !task) return;
    retryFocus.current = null;
    if (document.activeElement !== document.body && !task.origin.contains(document.activeElement)) return;
    const control = task.destination === "folder" ? folderControl : task.destination === "start" ? startControl : task.destination === "stop" ? stopControl : retryControl;
    control.current?.focus();
  }, [busy, status, error]);

  async function act(action: () => Promise<void>) {
    if (inFlight.current) return;
    inFlight.current = true; revision.current += 1; setBusy(true); setError("");
    try { await action(); }
    catch (error) { if (mounted.current) setError(nativeError(error)); }
    finally { inFlight.current = false; if (mounted.current) setBusy(false); }
  }

  async function retry(origin: HTMLButtonElement) {
    if (inFlight.current) return;
    const owned = origin.contains(document.activeElement);
    await act(async () => {
      let destination: "folder" | "start" | "stop" | "retry" = "retry";
      try {
        const next = await invoke<HostStatus>("host_status");
        if (mounted.current) {
          setStatus(next);
          destination = next.running ? "stop" : next.capabilities?.chooseFolder === false ? "start" : "folder";
        }
      } finally {
        if (owned && mounted.current) retryFocus.current = { origin, destination };
      }
    });
  }

  async function change(command: "choose_folder" | "stop_service") {
    await act(async () => {
      const next = await invoke<HostStatus>(command);
      if (mounted.current) {
        if (command === "stop_service") {
          setActiveKey(null); setShowKey(false);
          if (!next.running) focusAfterChange.current = "start";
        }
        setStatus(next);
      }
    });
  }

  const validPort = /^\d+$/.test(port) && Number(port) <= 65_535;
  const keyNeeded = uploads || manage;
  const capabilities = status?.capabilities ?? desktopCapabilities;
  const privateStorage = capabilities.storageKind === "app-private";
  const ready = !!status?.root && !status.running && validPort && (!keyNeeded || !!key.trim());
  const settings = status?.running ? status.activeSettings : null;
  const shownNetwork = settings?.network ?? (capabilities.openManager ? network : true);
  const shownPort = settings ? String(settings.port) : port;
  const shownUploads = settings?.uploads ?? uploads;
  const shownManage = settings?.manage ?? manage;
  const shownKey = status?.running
    ? settings?.authRequired && activeKey?.managerUrl === status.managerUrl ? activeKey?.value ?? "" : ""
    : key;

  async function start(event: FormEvent) {
    event.preventDefault();
    if (!ready) return;
    await act(async () => {
      const next = await invoke<HostStatus>("start_service", { options: {
        network: capabilities.openManager ? network : true,
        port: Number(port), uploads, manage, key: key.trim() ? key : null,
      } });
      if (mounted.current) {
        setActiveKey(key.trim() ? { managerUrl: next.managerUrl, value: key } : null);
        if (next.running) focusAfterChange.current = "stop";
        setStatus(next); setShowKey(false);
      }
    });
  }

  return <>
    <Header product={product} onBrandActivate={activateBrand} touchFriendly={capabilities.foregroundOnly} />
    <main className={`host-main${capabilities.foregroundOnly ? " host-touch" : ""}`}>
      <div className="host-heading"><h2>{privateStorage ? "Share files" : "Share a folder"}</h2><p>{privateStorage
        ? "Use this app’s storage to share files with another device on your local network."
        : "Choose the files this device makes available, then open the file manager or connect from another device."}</p></div>
      {capabilities.foregroundOnly && <p className="notice" role="note">Keep this app on screen. Sharing stops when it goes into the background. Return here and start sharing again.</p>}
      <p className="visually-hidden" role="status" aria-live="polite" aria-atomic="true">{status ? status.running ? "Sharing is active." : "Sharing is stopped." : ""}</p>
      {error && <p className="notice error" role="alert">{error}</p>}
      {!status ? <>{error || busy ? <button ref={retryControl} type="button" className="button button-secondary" disabled={busy} onClick={(event) => void retry(event.currentTarget)}>{busy ? "Connecting…" : "Try again"}</button>
        : <p role="status">Connecting to native controls…</p>}</> : <div className="host-layout">
        <form className="host-settings" onSubmit={start}>
          <section className="host-section" aria-labelledby="host-folder-title"><h3 id="host-folder-title">{privateStorage ? "App storage" : "Shared folder"}</h3>
            <p className="host-root">{privateStorage ? "Files uploaded here stay in this app. Uninstalling removes them." : status.root ?? "No folder selected"}</p>
            {capabilities.chooseFolder && <button ref={folderControl} type="button" className="button button-secondary" disabled={busy || status.running} onClick={() => void change("choose_folder")}><FolderOpen size={18} aria-hidden="true" />Choose folder</button>}
            {status.running && <p className="muted">{capabilities.chooseFolder ? "Stop sharing to change the folder or access settings." : "Stop sharing to change access settings."}</p>}
          </section>
          <fieldset className="host-section host-options" disabled={busy || status.running}><legend>Access settings</legend>
            {status.running && !settings ? <p className="muted">Active settings are unavailable. Stop sharing to configure a new session.</p> : <>
            <label className="checkbox-field"><input type="checkbox" checked={shownNetwork} disabled={!capabilities.openManager} onChange={(event) => setNetwork(event.target.checked)} />Share on the local network</label>
            <p className="muted">{shownNetwork ? "Other devices can connect using this device’s local network address." : "Only browsers on this device can connect."}</p>
            <label className="field host-port">Port<input type="number" min="0" max="65535" step="1" required value={shownPort} onChange={(event) => setPort(event.target.value)} /></label>
            <p className="muted">Use 0 to choose an available port.</p>
            <div className="host-permissions"><label className="checkbox-field"><input type="checkbox" checked={shownUploads} onChange={(event) => setUploads(event.target.checked)} />Allow uploads</label>
              <label className="checkbox-field"><input type="checkbox" checked={shownManage} onChange={(event) => setManage(event.target.checked)} />Allow file management</label></div>
            </>}
          </fieldset>
          <section className="host-section" aria-labelledby="host-key-title"><h3 id="host-key-title">Access key</h3>
            <p className="muted">Required for uploads and file management; optional for read-only sharing.</p>
            {settings?.authRequired && !shownKey && <p className="muted">An access key is configured on this device. Stop sharing to enter or generate a new key.</p>}
            {settings && !settings.authRequired && <p className="muted">No access key is set for this sharing session.</p>}
            <div className="host-key-row"><label className="field"><span className="visually-hidden">Access key</span><input type={showKey ? "text" : "password"} autoComplete="off" value={shownKey} disabled={busy} readOnly={status.running} onChange={(event) => setKey(event.target.value)} /></label>
              <button type="button" className="icon-button" aria-label={showKey ? "Hide access key" : "Show access key"} aria-pressed={showKey} disabled={busy || (status.running && !shownKey)} onClick={() => setShowKey((value) => !value)}>{showKey ? <EyeOff size={18} aria-hidden="true" /> : <Eye size={18} aria-hidden="true" />}</button></div>
            <button type="button" className="button button-secondary" disabled={busy || status.running} onClick={() => void act(async () => { const next = await invoke<string>("create_access_key"); if (mounted.current) setKey(next); })}>Generate access key</button>
          </section>
          <div className="form-actions">{status.running
            ? <button key="stop" ref={stopControl} type="button" className="button button-secondary" disabled={busy} onClick={() => void change("stop_service")}><Square size={16} aria-hidden="true" />{busy ? "Working…" : "Stop sharing"}</button>
            : <button key="start" ref={startControl} type="submit" className="button button-primary" disabled={busy || !ready}><Play size={17} aria-hidden="true" />{busy ? "Working…" : "Start sharing"}</button>}
          </div>
        </form>
        <aside className="host-connection" aria-labelledby="connection-title"><h3 id="connection-title">{status.running ? "Sharing is active" : "Ready when you are"}</h3>
          {status.running ? <><p>{capabilities.openManager ? "Open the file manager on this device to browse and transfer files." : "Open a local network address on another device to upload, browse and manage files."}</p>
            {capabilities.openManager && <button className="button button-primary" disabled={busy} onClick={() => void act(async () => { await invoke("open_manager"); })}>Open file manager</button>}
            <dl className="host-addresses">{capabilities.openManager && <div><dt>This device</dt><dd>{status.managerUrl}</dd></div>}{status.addresses.length > 0 && <div><dt>Local network</dt><dd><ul>{status.addresses.map((address) => <li key={address}>{address}</li>)}</ul></dd></div>}</dl>
            <p className="muted">{capabilities.openManager ? "Share a local network address and the access key with the person connecting. Closing the file manager stops sharing." : "Share the address and any access key with the person connecting. Keep this app on screen while transferring."}</p>
          </> : <p>{privateStorage ? "Start sharing, then upload files from another device. Files stay in this app." : "Select a folder and start sharing. Files stay on this device."}</p>}
        </aside>
      </div>}
      {diagnosticsOpen && <Suspense fallback={<p role="status">Loading local diagnostics…</p>}><DiagnosticPanel serviceRunning={!!status?.running} onClose={closeDiagnostics} /></Suspense>}
    </main>
    <footer className="app-footer"><span><Wordmark name={product.name} /> {product.version}</span><span>Local file sharing</span></footer>
  </>;
}
