"use client";

import { invoke } from "@tauri-apps/api/core";
import { Eye, EyeOff, FolderOpen, Play, Square } from "lucide-react";
import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import product from "@/generated/product.json";
import { Header } from "./file-manager";

interface HostStatus {
  root: string | null;
  running: boolean;
  managerUrl: string | null;
  addresses: string[];
}

function nativeError(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : "The native action failed. Try again.";
}

export default function NativeHost() {
  const [status, setStatus] = useState<HostStatus | null>(null);
  const [network, setNetwork] = useState(false);
  const [port, setPort] = useState("5000");
  const [uploads, setUploads] = useState(false);
  const [manage, setManage] = useState(false);
  const [key, setKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const inFlight = useRef(false);
  const revision = useRef(0);
  const mounted = useRef(true);

  const refresh = useCallback(async () => {
    if (inFlight.current) return;
    const current = revision.current;
    try {
      const next = await invoke<HostStatus>("host_status");
      if (mounted.current && current === revision.current) { setStatus(next); }
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

  async function act(action: () => Promise<void>) {
    if (inFlight.current) return;
    inFlight.current = true; revision.current += 1; setBusy(true); setError("");
    try { await action(); }
    catch (error) { if (mounted.current) setError(nativeError(error)); }
    finally { inFlight.current = false; if (mounted.current) setBusy(false); }
  }

  async function change(command: "choose_folder" | "stop_service") {
    await act(async () => { const next = await invoke<HostStatus>(command); if (mounted.current) setStatus(next); });
  }

  const validPort = /^\d+$/.test(port) && Number(port) <= 65_535;
  const keyNeeded = uploads || manage;
  const ready = !!status?.root && validPort && (!keyNeeded || !!key.trim());

  async function start(event: FormEvent) {
    event.preventDefault();
    if (!ready) return;
    await act(async () => {
      const next = await invoke<HostStatus>("start_service", { options: {
        network, port: Number(port), uploads, manage, key: key.trim() ? key : null,
      } });
      if (mounted.current) { setStatus(next); setShowKey(false); }
    });
  }

  return <>
    <Header product={product} />
    <main className="host-main">
      <div className="host-heading"><h2>Share a folder</h2><p>Choose the files this device makes available, then open the file manager or connect from another device.</p></div>
      {error && <p className="notice error" role="alert">{error}</p>}
      {!status ? <p role="status">Connecting to native controls…</p> : <div className="host-layout">
        <form className="host-settings" onSubmit={start}>
          <section className="host-section" aria-labelledby="host-folder-title"><h3 id="host-folder-title">Shared folder</h3>
            <p className="host-root">{status.root ?? "No folder selected"}</p>
            <button type="button" className="button button-secondary" disabled={busy || status.running} onClick={() => void change("choose_folder")}><FolderOpen size={18} aria-hidden="true" />Choose folder</button>
            {status.running && <p className="muted">Stop sharing to change the folder or access settings.</p>}
          </section>
          <fieldset className="host-section host-options" disabled={busy || status.running}><legend>Access settings</legend>
            <label className="checkbox-field"><input type="checkbox" checked={network} onChange={(event) => setNetwork(event.target.checked)} />Share on the local network</label>
            <p className="muted">{network ? "Other devices can connect using this device’s local network address." : "Only browsers on this device can connect."}</p>
            <label className="field host-port">Port<input type="number" min="0" max="65535" step="1" required value={port} onChange={(event) => setPort(event.target.value)} /></label>
            <p className="muted">Use 0 to choose an available port.</p>
            <div className="host-permissions"><label className="checkbox-field"><input type="checkbox" checked={uploads} onChange={(event) => setUploads(event.target.checked)} />Allow uploads</label>
              <label className="checkbox-field"><input type="checkbox" checked={manage} onChange={(event) => setManage(event.target.checked)} />Allow file management</label></div>
          </fieldset>
          <section className="host-section" aria-labelledby="host-key-title"><h3 id="host-key-title">Access key</h3>
            <p className="muted">Required for uploads and file management; optional for read-only sharing.</p>
            <div className="host-key-row"><label className="field"><span className="visually-hidden">Access key</span><input type={showKey ? "text" : "password"} autoComplete="off" value={key} disabled={busy} readOnly={status.running} onChange={(event) => setKey(event.target.value)} /></label>
              <button type="button" className="icon-button" aria-label={showKey ? "Hide access key" : "Show access key"} aria-pressed={showKey} onClick={() => setShowKey((value) => !value)}>{showKey ? <EyeOff size={18} aria-hidden="true" /> : <Eye size={18} aria-hidden="true" />}</button></div>
            <button type="button" className="button button-secondary" disabled={busy || status.running} onClick={() => void act(async () => { const next = await invoke<string>("create_access_key"); if (mounted.current) setKey(next); })}>Generate access key</button>
          </section>
          <div className="form-actions">{status.running
            ? <button key="stop" type="button" className="button button-secondary" disabled={busy} onClick={() => void change("stop_service")}><Square size={16} aria-hidden="true" />{busy ? "Working…" : "Stop sharing"}</button>
            : <button key="start" type="submit" className="button button-primary" disabled={busy || !ready}><Play size={17} aria-hidden="true" />{busy ? "Working…" : "Start sharing"}</button>}
          </div>
        </form>
        <aside className="host-connection" aria-labelledby="connection-title"><h3 id="connection-title">{status.running ? "Sharing is active" : "Ready when you are"}</h3>
          {status.running ? <><p>Open the file manager on this device to browse and transfer files.</p><button className="button button-primary" disabled={busy} onClick={() => void act(async () => { await invoke("open_manager"); })}>Open file manager</button>
            <dl className="host-addresses"><div><dt>This device</dt><dd>{status.managerUrl}</dd></div>{status.addresses.length > 0 && <div><dt>Local network</dt><dd><ul>{status.addresses.map((address) => <li key={address}>{address}</li>)}</ul></dd></div>}</dl>
            <p className="muted">Share a local network address and the access key with the person connecting. Closing the file manager stops sharing.</p>
          </> : <p>Select a folder and start sharing. Files stay on this device.</p>}
        </aside>
      </div>}
    </main>
    <footer className="app-footer"><span>{product.name} {product.version}</span><span>Local file sharing</span></footer>
  </>;
}
