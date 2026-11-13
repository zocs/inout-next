"use client";

import { ArrowUpDown, ChevronRight, Download, Folder, LockKeyhole, LogOut, RefreshCw, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import productDefaults from "@/generated/product.json";
import { featureRegistry } from "@/features/registry";
import { ApiError, FileApi } from "@/lib/api";
import type { Entry, Product, ServerContext, ServerStatus } from "@/lib/contracts";
import { formatBytes, formatDate } from "@/lib/format";
import { activeModules, ModuleProviders, ModuleSlot, type ModuleRuntime } from "./modules";

type Message = { text: string; kind: "success" | "error" } | null;

export default function FileManager() {
  const [api] = useState(() => new FileApi());
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [context, setContext] = useState<ServerContext | null>(null);
  const [phase, setPhase] = useState<"loading" | "unlock" | "ready" | "error">("loading");
  const [path, setPath] = useState("");
  const [entries, setEntries] = useState<Entry[]>([]);
  const [selected, select] = useState<Entry | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshToken, setRefreshToken] = useState(0);
  const [message, setMessage] = useState<Message>(null);
  const product = context?.product ?? status?.product ?? productDefaults;
  const refresh = useCallback(() => setRefreshToken((value) => value + 1), []);
  const notify = useCallback((text: string, kind: "success" | "error" = "success") => setMessage({ text, kind }), []);

  const connect = useCallback(async () => {
    setPhase("loading"); setMessage(null);
    try {
      const status = await api.status(); setStatus(status);
      const context = await api.context(); setContext(context); setPhase("ready");
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) { setContext(null); setPhase("unlock"); }
      else { setPhase("error"); notify(error instanceof Error ? error.message : "Cannot open the shared folder.", "error"); }
    }
  }, [api, notify]);

  useEffect(() => {
    setPath(new URLSearchParams(window.location.search).get("path") ?? "");
    void connect();
    const onBack = () => { setPath(new URLSearchParams(window.location.search).get("path") ?? ""); select(null); };
    window.addEventListener("popstate", onBack);
    return () => window.removeEventListener("popstate", onBack);
  }, [connect]);

  useEffect(() => { document.title = path ? `${path.split("/").at(-1)} · ${product.name}` : product.name; }, [path, product.name]);

  useEffect(() => {
    if (!context || phase !== "ready") return;
    if (!context.permissions.list || !context.modules.includes("files")) { setEntries([]); setLoading(false); return; }
    const controller = new AbortController();
    setLoading(true);
    api.entries(path, controller.signal).then((listing) => {
      setEntries(listing.entries);
      select((current) => current ? listing.entries.find((entry) => entry.path === current.path) ?? null : null);
    }).catch((error) => {
      if (controller.signal.aborted) return;
      if (error instanceof ApiError && error.status === 401) { setContext(null); setPhase("unlock"); }
      else notify(error instanceof Error ? error.message : "Could not list this folder.", "error");
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [api, context, path, phase, refreshToken, notify]);

  const navigate = useCallback((nextPath: string) => {
    select(null); setMessage(null); setPath(nextPath);
    const query = nextPath ? "?path=" + encodeURIComponent(nextPath) : "";
    window.history.pushState(null, "", window.location.pathname + query);
    refresh();
  }, [refresh]);

  async function logout() {
    try { await api.logout(); setContext(null); setPhase("unlock"); select(null); setMessage(null); }
    catch (error) { notify(error instanceof Error ? error.message : "Could not lock the files. Try again.", "error"); }
  }

  const modules = useMemo(() => context ? activeModules(featureRegistry, context) : [], [context]);
  const runtime: ModuleRuntime | null = context ? { context, api, path, entries, selected, select, loading, navigate, refresh, notify } : null;
  return <>
    <Header product={product} context={context} canLock={phase === "ready" && !!status?.auth_required} lock={() => void logout()} />
    {phase === "unlock" ? <Unlock api={api} product={product} unlocked={(next) => { setContext(next); setPhase("ready"); setMessage(null); }} />
      : phase !== "ready" || !runtime ? <main className="connection-state"><h2>{phase === "error" ? "Cannot connect" : "Opening shared files"}</h2>
        {message && <p role="alert" className="error-text">{message.text}</p>}
        {phase === "error" ? <button className="button button-primary" onClick={() => void connect()}>Try again</button> : <div className="loading-rows" aria-hidden="true"><div /><div /><div /></div>}
      </main> : <ModuleProviders modules={modules} runtime={runtime}>
        <div className="workspace">
          <aside className="side-rail"><nav aria-label="Places"><button className="rail-link current" aria-label="Go to shared folder" onClick={() => navigate("")}><Folder size={19} aria-hidden="true" />Shared folder</button></nav>
            <div className="rail-note"><p>On the sharing device</p><p>Files stay in the selected folder. Available actions follow its access settings.</p></div>
          </aside>
          <main className="manager-main">
            <nav className="breadcrumbs" aria-label="Folder path"><button aria-label="Shared folder" onClick={() => navigate("")}><Folder size={16} aria-hidden="true" /><span>Shared folder</span></button>
              {path.split("/").filter(Boolean).map((part, index, parts) => <span key={index}><ChevronRight size={14} aria-hidden="true" /><button aria-current={index === parts.length - 1 ? "page" : undefined} onClick={() => navigate(parts.slice(0, index + 1).join("/"))} title={part}>{part}</button></span>)}
            </nav>
            <div className="folder-heading"><div><h2>{path.split("/").at(-1) || context?.root_label || "Shared files"}</h2><p>Browse and transfer files from this device.</p></div>
              <div className="toolbar" role="toolbar" aria-label="Folder actions"><ModuleSlot modules={modules} slot="Toolbar" runtime={runtime} /><button className="icon-button refresh-button" aria-label="Refresh folder" title="Refresh folder" onClick={refresh} disabled={loading}><RefreshCw size={18} aria-hidden="true" /></button></div>
            </div>
            {message && <div className={`notice ${message.kind}`} role={message.kind === "error" ? "alert" : "status"}><span>{message.text}</span><button className="icon-button" aria-label="Dismiss message" onClick={() => setMessage(null)}><X size={16} aria-hidden="true" /></button></div>}
            <div className={`content-layout ${selected || modules.some((module) => module.Panel) ? "with-panels" : ""}`}>
              <div className="views">{modules.some((module) => module.View) ? <ModuleSlot modules={modules} slot="View" runtime={runtime} /> : <div className="empty-state"><h3>File browsing is unavailable</h3><p>The sharing device has not enabled file access.</p></div>}</div>
              <aside className="tools-column" aria-label="Folder tools">
                {selected && <section className="selection-panel" aria-label="Selected entry"><div className="panel-heading"><h3>Selected entry</h3><button className="icon-button" aria-label="Clear selection" onClick={() => select(null)}><X size={16} aria-hidden="true" /></button></div><h4 title={selected.name}>{selected.name}</h4><dl><div><dt>Size</dt><dd>{formatBytes(selected.size)}</dd></div><div><dt>Modified</dt><dd>{formatDate(selected.modified)}</dd></div><div><dt>Path</dt><dd className="entry-path">{selected.path}</dd></div></dl>
                  {selected.kind === "file" && context?.permissions.download && <a className="button button-secondary selection-download" href={api.downloadUrl(selected.path)} download={selected.name} aria-label={`Download selected ${selected.name}`}><Download size={17} aria-hidden="true" />Download</a>}
                </section>}
                <ModuleSlot modules={modules} slot="Panel" runtime={runtime} />
              </aside>
            </div>
          </main>
        </div>
      </ModuleProviders>}
    <footer className="app-footer"><span>{product.name} {product.version}</span><span>Local file sharing</span></footer>
  </>;
}

export function Header({ product, context, canLock, lock }: { product: Product; context?: ServerContext | null; canLock?: boolean; lock?: () => void }) {
  return <header className="app-header"><div className="brand"><ArrowUpDown size={23} aria-hidden="true" /><h1>{product.name}</h1><span className="brand-description">{product.description}</span></div><div className="header-actions">{context && <span className="access-mode">{context.permissions.upload || context.permissions.manage ? "Can edit" : "Read only"}</span>}{canLock && <button className="header-button" onClick={lock}><LogOut size={16} aria-hidden="true" />Lock files</button>}</div></header>;
}

function Unlock({ api, product, unlocked }: { api: FileApi; product: Product; unlocked: (context: ServerContext) => void }) {
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function login(event: FormEvent) {
    event.preventDefault(); if (busy) return;
    setBusy(true); setError("");
    try { await api.login(key); setKey(""); unlocked(await api.context()); }
    catch (error) { setError(error instanceof Error ? error.message : "Could not unlock files. Check the access key."); }
    finally { setBusy(false); }
  }
  return <main className="unlock-layout"><section className="unlock-panel"><LockKeyhole size={28} aria-hidden="true" /><h2>Unlock shared files</h2><p>Enter the access key set on the device running {product.name}.</p><form onSubmit={login}>
    <label className="field">Access key<input type="password" autoFocus required autoComplete="off" value={key} onChange={(event) => setKey(event.target.value)} /></label>
    {error && <p role="alert" className="error-text">{error}</p>}
    <button className="button button-primary" disabled={busy || !key}>{busy ? "Unlocking…" : "Unlock files"}</button>
  </form></section></main>;
}
