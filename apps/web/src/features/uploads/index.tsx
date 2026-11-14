import { Check, Upload, X } from "lucide-react";
import { createContext, useContext, useEffect, useRef, useState } from "react";
import { ApiError, sendFile } from "@/lib/api";
import { formatBytes, joinPath } from "@/lib/format";
import type { FeatureModule, ModuleProps, ModuleProviderProps } from "@/ui/modules";
import { captureTaskFocus } from "@/ui/focus";

type Phase = "queued" | "sending" | "finalizing" | "saved" | "error" | "canceled";
interface Job { id: number; file: File; path: string; phase: Phase; percent: number; error?: string }
interface UploadState { jobs: Job[]; add: (files: File[]) => void; cancel: (id: number) => void; clear: () => void }
const UploadContext = createContext<UploadState | null>(null);

function useUploads(): UploadState {
  const state = useContext(UploadContext);
  if (!state) throw new Error("Upload contribution needs its module provider");
  return state;
}

function UploadProvider({ runtime, children }: ModuleProviderProps) {
  const [jobs, setJobs] = useState<Job[]>([]);
  const runtimeRef = useRef(runtime);
  runtimeRef.current = runtime;
  const queue = useRef<Job[]>([]);
  const active = useRef(new Map<number, ReturnType<typeof sendFile>>());
  const phases = useRef(new Map<number, Phase>());
  const canceled = useRef(new Set<number>());
  const sequence = useRef(0);
  const running = useRef(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      queue.current = [];
      for (const transfer of active.current.values()) transfer.cancel();
    };
  }, []);

  function patch(id: number, values: Partial<Job>) {
    if (values.phase) phases.current.set(id, values.phase);
    if (mounted.current) setJobs((current) => current.map((job) => job.id === id ? { ...job, ...values } : job));
  }

  async function pump() {
    if (running.current) return;
    running.current = true;
    try {
      while (mounted.current && queue.current.length) {
        const job = queue.current.shift()!;
        if (canceled.current.has(job.id)) continue;
        patch(job.id, { phase: "sending" });
        const transfer = sendFile(job.file, job.path, (progress) => patch(job.id, {
          phase: progress.finalizing ? "finalizing" : "sending",
          percent: progress.total ? Math.min(100, Math.round(progress.sent / progress.total * 100)) : 0,
        }));
        active.current.set(job.id, transfer);
        try {
          await transfer.promise;
          patch(job.id, { phase: "saved", percent: 100 });
          runtimeRef.current.refresh();
        } catch (error) {
          patch(job.id, error instanceof ApiError && error.code === "canceled"
            ? { phase: "canceled" }
            : { phase: "error", error: error instanceof Error ? error.message : "Upload failed. Choose the file again to retry." });
        } finally { active.current.delete(job.id); }
      }
    } finally { running.current = false; }
  }

  function add(files: File[]) {
    const limit = runtimeRef.current.context.limits.max_upload_bytes;
    const additions = files.map((file): Job => ({
      id: ++sequence.current, file, path: joinPath(runtimeRef.current.path, file.name), percent: 0,
      phase: file.size > limit ? "error" : "queued",
      error: file.size > limit ? `This file exceeds the ${formatBytes(limit)} upload limit.` : undefined,
    }));
    for (const job of additions) phases.current.set(job.id, job.phase);
    setJobs((current) => [...current, ...additions]);
    queue.current.push(...additions.filter((job) => job.phase === "queued"));
    void pump();
  }

  function cancel(id: number) {
    if (phases.current.get(id) === "finalizing") return;
    canceled.current.add(id);
    if (active.current.has(id)) active.current.get(id)!.cancel();
    else patch(id, { phase: "canceled" });
  }

  function clear() {
    setJobs((current) => current.filter((job) => ["queued", "sending", "finalizing"].includes(job.phase)));
  }

  return <UploadContext.Provider value={{ jobs, add, cancel, clear }}>{children}</UploadContext.Provider>;
}

function UploadToolbar() {
  const { add } = useUploads();
  return <label className="button button-primary upload-control"><Upload size={18} aria-hidden="true" /><span>Upload files</span>
    <input className="visually-hidden" type="file" aria-label="Upload files" multiple onChange={(event) => {
      add(Array.from(event.target.files ?? [])); event.target.value = "";
    }} />
  </label>;
}

const phaseLabels: Record<Phase, string> = { queued: "Queued", sending: "Uploading", finalizing: "Saving…", saved: "Saved", error: "Failed", canceled: "Canceled" };

function UploadPanel({ runtime }: ModuleProps) {
  const { jobs, cancel, clear } = useUploads();
  const heading = useRef<HTMLHeadingElement>(null);
  return <section className="transfer-panel" aria-label="Transfers">
    <div className="panel-heading"><h3 ref={heading} tabIndex={-1}>Transfers</h3>{jobs.some((job) => ["saved", "error", "canceled"].includes(job.phase)) && <button className="text-button" onClick={(event) => { const finishFocus = captureTaskFocus(event.currentTarget); clear(); finishFocus(() => heading.current?.focus()); }}>Clear finished</button>}</div>
    {jobs.length === 0 ? <p className="muted">Upload up to {formatBytes(runtime.context.limits.max_upload_bytes)} per file. Files are saved after the device confirms the transfer.</p>
      : <ul className="transfer-list">{jobs.map((job) => <li key={job.id}>
        <div className="transfer-name"><span title={job.path}>{job.file.name}</span>{job.phase === "saved" ? <Check className="saved-mark" size={18} aria-hidden="true" />
          : ["queued", "sending", "finalizing"].includes(job.phase) && <button className="icon-button" onClick={() => cancel(job.id)} disabled={job.phase === "finalizing"} aria-label={`Cancel ${job.file.name}`}><X size={16} aria-hidden="true" /></button>}</div>
        <div className="transfer-status"><span className={job.phase === "error" ? "error-text" : ""} role="status">{phaseLabels[job.phase]}</span><span>{job.phase === "sending" ? `${job.percent}%` : formatBytes(job.file.size)}</span></div>
        {job.phase === "sending" && <progress max={100} value={job.percent} aria-label={`Upload progress for ${job.file.name}`} />}
        {job.phase === "finalizing" && <p className="muted transfer-note">Sent. Waiting for the device to confirm it is saved.</p>}
        {job.error && <p role="alert" className="error-text transfer-note">{job.error}</p>}
        {job.phase === "canceled" && <p className="muted transfer-note">Stopped sending. Unfinished files are removed by the device.</p>}
      </li>)}</ul>}
  </section>;
}

export const uploadsModule: FeatureModule = {
  id: "uploads", requires: ["files"], enabled: (context) => context.permissions.upload,
  Provider: UploadProvider, Toolbar: UploadToolbar, Panel: UploadPanel,
};
