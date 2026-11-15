"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import type { DiagnosticReport, DiagnosticStatus } from "./contract";
import styles from "./panel.module.css";
import { useI18n } from "@/i18n/provider";
import { formatNumber } from "@/lib/format";
import { diagnosticMessage, diagnosticText, type DiagnosticMessage } from "./catalog";

function message(error: unknown) {
  return typeof error === "string" ? error : error instanceof Error ? error.message : "The local action failed. Try again.";
}

export default function DiagnosticPanel({ serviceRunning, onClose }: { serviceRunning: boolean; onClose: () => void }) {
  const { locale } = useI18n();
  const t = (key: Parameters<typeof diagnosticText>[1], ...args: (string | number)[]) => diagnosticText(locale, key, ...args);
  const [status, setStatus] = useState<DiagnosticStatus | null>(null);
  const [buffer, setBuffer] = useState("128");
  const [connections, setConnections] = useState("1");
  const [path, setPath] = useState("");
  const [pending, setPending] = useState(false);
  const [testing, setTesting] = useState(false);
  const [closing, setClosing] = useState(false);
  const [error, setError] = useState<DiagnosticMessage>("");
  const [notice, setNotice] = useState<DiagnosticMessage>("");
  const heading = useRef<HTMLHeadingElement>(null);
  const mounted = useRef(false);
  const busy = useRef(false);
  const revision = useRef(0);
  const didClose = useRef(false);

  const refresh = useCallback(async () => {
    if (busy.current) return;
    const version = revision.current;
    try {
      const next = await invoke<DiagnosticStatus>("diagnostic_status");
      if (mounted.current && version === revision.current) setStatus(next);
    } catch (error) { if (mounted.current && version === revision.current) setError(message(error)); }
  }, []);

  useEffect(() => {
    mounted.current = true;
    heading.current?.focus();
    return () => {
      mounted.current = false;
      revision.current += 1;
      if (!didClose.current) void invoke("set_diagnostic_mode", { enabled: false }).catch(() => {});
    };
  }, []);

  useEffect(() => {
    let active = true;
    revision.current += 1;
    const version = revision.current;
    // A session change releases the old action immediately. Its reply and
    // finalizer no longer own this revision, including any new busy action.
    busy.current = false;
    setPending(false); setTesting(false); setClosing(false);
    setError(""); setNotice(""); setBuffer("128");
    setStatus(null);
    void (async () => {
      try {
        const next = await invoke<DiagnosticStatus>(serviceRunning ? "set_diagnostic_mode" : "diagnostic_status", serviceRunning ? { enabled: true } : undefined);
        if (mounted.current && active && version === revision.current) { setStatus(next); setBuffer(String(next.bufferKib)); }
      } catch (error) { if (mounted.current && active && version === revision.current) setError(message(error)); }
    })();
    const timer = window.setInterval(() => void refresh(), 1_000);
    return () => { active = false; window.clearInterval(timer); };
  }, [serviceRunning, refresh]);

  async function settings(command: "set_download_buffer" | "reset_diagnostic_settings") {
    if (busy.current) return;
    busy.current = true; revision.current += 1; setPending(true); setError(""); setNotice("");
    const version = revision.current;
    try {
      const next = await invoke<DiagnosticStatus>(command, command === "set_download_buffer" ? { kib: Number(buffer) } : undefined);
      if (mounted.current && version === revision.current) { setStatus(next); setBuffer(String(next.bufferKib)); setNotice(command === "set_download_buffer" ? "Buffer applied to subsequent requests." : "Default buffer restored."); }
    } catch (error) { if (mounted.current && version === revision.current) setError(message(error)); }
    finally { if (mounted.current && version === revision.current) { busy.current = false; setPending(false); } }
  }

  async function run(event: FormEvent) {
    event.preventDefault();
    if (busy.current || !path || !status?.enabled) return;
    busy.current = true; revision.current += 1; setTesting(true); setError(""); setNotice("");
    const version = revision.current;
    setStatus((current) => current ? { ...current, running: true, probeState: "running", result: null } : current);
    try {
      const result = await invoke<DiagnosticReport>("run_diagnostic_test", { path, connections: Number(connections) });
      if (mounted.current && version === revision.current) setStatus((current) => current ? { ...current, result, running: false, probeState: "idle" } : current);
    } catch (error) {
      if (mounted.current && version === revision.current) {
        const text = message(error);
        if (text === "Local test canceled") setNotice(text); else setError(text);
        setStatus((current) => current ? { ...current, running: false, probeState: "idle" } : current);
      }
    } finally { if (mounted.current && version === revision.current) { busy.current = false; setTesting(false); } }
  }

  async function cancel() {
    const version = revision.current;
    setPending(true); setError("");
    try { await invoke("cancel_diagnostic_test"); }
    catch (error) { if (mounted.current && version === revision.current) setError(message(error)); }
    finally { if (mounted.current && version === revision.current) setPending(false); }
  }

  async function close() {
    if (closing) return;
    setClosing(true); revision.current += 1;
    const version = revision.current;
    busy.current = true; setPending(false); setTesting(false);
    try {
      await invoke("set_diagnostic_mode", { enabled: false });
      if (mounted.current && version === revision.current) { didClose.current = true; onClose(); }
    } catch (error) { if (mounted.current && version === revision.current) { busy.current = false; setError(message(error)); setClosing(false); } }
  }

  const running = testing || !!status?.running;
  const available = serviceRunning && !!status?.enabled;
  const locked = !available || pending || running || closing;
  const result = status?.result;

  return <section className={styles.panel} aria-labelledby="diagnostics-title">
    <div className={styles.heading}><h2 id="diagnostics-title" ref={heading} tabIndex={-1}>{t("Local diagnostics")}</h2>
      <button type="button" className="button button-secondary" disabled={closing} onClick={() => void close()}>{closing ? t("Closing…") : t("Close diagnostics")}</button></div>
    <p className="muted">{t("Temporary settings for this sharing session. Buffer changes apply to subsequent downloads; closing diagnostics or stopping sharing restores the default.")}</p>
    {!serviceRunning && <p role="status">{t("Start sharing to use local diagnostics.")}</p>}
    {serviceRunning && !status && <p role="status">{t("Connecting to local diagnostics…")}</p>}
    {error && <p className="notice error" role="alert">{diagnosticMessage(locale, error)}</p>}
    <p role="status" aria-live="polite" aria-atomic="true">{running ? t("Running a read-only local test…") : diagnosticMessage(locale, notice)}</p>
    <form onSubmit={(event) => void run(event)}>
      <div className={styles.fields}>
        <label className="field">{t("Download buffer")}<select value={buffer} disabled={locked} onChange={(event) => setBuffer(event.target.value)}>{[16,32,48,64,80,96,112,128].map((kib) => <option key={kib} value={kib}>{kib} KiB</option>)}</select></label>
        <label className="field">{t("Test connections")}<select value={connections} disabled={locked} onChange={(event) => setConnections(event.target.value)}>{[1,2,4,8].map((count) => <option key={count} value={count}>{count}</option>)}</select></label>
        <label className={`field ${styles.path}`}>{t("Relative shared-file path")}<input value={path} disabled={locked} maxLength={4096} required autoComplete="off" placeholder="folder/file.bin" onChange={(event) => setPath(event.target.value)} aria-describedby="diagnostics-read-only" /></label>
      </div>
      {status && <p className="muted">{t("Current buffer: {0} KiB", status.bufferKib)}</p>}
      <div className={styles.actions}>
        <button type="button" className="button button-secondary" disabled={locked} onClick={() => void settings("set_download_buffer")}>{pending && !running ? t("Working…") : t("Apply buffer")}</button>
        <button type="button" className="button button-secondary" disabled={locked} onClick={() => void settings("reset_diagnostic_settings")}>{t("Restore default buffer")}</button>
        <button type="submit" className="button button-primary" disabled={locked || !path}>{t("Run read-only test")}</button>
        {running && <button type="button" className="button button-secondary" disabled={pending || closing} onClick={() => void cancel()}>{t("Cancel test")}</button>}
      </div>
    </form>
    <p id="diagnostics-read-only" className="muted">{t("Reads up to the first 64 MiB through this device’s shared-file service without saving data. The test ends within 30 seconds. This same-device timing sample measures local throughput; it does not prove file integrity or external network speed.")}</p>
    {result && <div className={styles.result}><h3>{t("Local throughput sample")}</h3><dl>
      <div><dt>{t("Local throughput")}</dt><dd>{formatNumber(result.mibPerSecond, locale, 2, true)} MiB/s</dd></div>
      <div><dt>{t("Bytes read")}</dt><dd>{formatNumber(result.bytes, locale)}</dd></div>
      <div><dt>{t("Duration")}</dt><dd>{formatNumber(result.durationMs, locale, 1, true)} ms</dd></div>
      <div><dt>{t("Sample settings")}</dt><dd>{t("{0} KiB buffer with {1} connections", result.bufferKib, result.connections)}</dd></div>
    </dl></div>}
  </section>;
}
