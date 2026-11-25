import { FileQuestion, Maximize2, X } from "lucide-react";
import { useEffect, useId, useRef, useState, type ReactNode, type RefObject } from "react";
import { useI18n } from "@/i18n/provider";
import type { FileApi, TextPreview } from "@/lib/api";
import type { Entry } from "@/lib/contracts";
import { formatBytes } from "@/lib/format";
import { captureTaskFocus } from "@/ui/focus";
import type { FeatureModule, ModuleProps } from "@/ui/modules";
import { previewKind } from "./kinds";

const TEXT_LIMIT = 256 * 1024;
// A photo this large is fetched only on request: showing it costs a whole download.
const AUTOMATIC_IMAGE_LIMIT = 32 * 1024 * 1024;

type Phase = "waiting" | "loading" | "ready" | "error" | "unsupported";

function PreviewInspector({ runtime }: ModuleProps) {
  const entry = runtime.selected;
  if (!entry || entry.kind !== "file") return null;
  // A refreshed file is a new preview, even if a replacement kept its metadata.
  return <FilePreview key={`${entry.path}:${entry.size}:${entry.modified}:${runtime.contentRevision}`} entry={entry} api={runtime.api} revision={runtime.contentRevision} />;
}

function FilePreview({ entry, api, revision }: { entry: Entry; api: FileApi; revision: number }) {
  const { t, locale } = useI18n();
  const kind = previewKind(entry.name);
  const url = api.previewUrl(entry.path, revision);
  const label = t("Preview of {0}", entry.name);
  const [phase, setPhase] = useState<Phase>(!kind ? "unsupported"
    : kind === "image" && (entry.size ?? 0) > AUTOMATIC_IMAGE_LIMIT ? "waiting" : "loading");
  const [text, setText] = useState<TextPreview | null>(null);
  const [enlarged, setEnlarged] = useState(false);
  const enlarge = useRef<HTMLButtonElement>(null);
  const frame = useRef<HTMLDivElement>(null);
  const empty = entry.size === 0;

  useEffect(() => {
    if (kind !== "text" || empty) return;
    const controller = new AbortController();
    api.previewText(entry.path, TEXT_LIMIT, controller.signal, revision).then((preview) => {
      if (controller.signal.aborted) return;
      setText(preview); setPhase(preview ? "ready" : "unsupported");
    }).catch(() => { if (!controller.signal.aborted) setPhase("error"); });
    return () => controller.abort();
  }, [api, entry.path, kind, empty, revision]);

  const media = (large: boolean): ReactNode => kind === "image"
    ? <img src={url} alt={label} decoding="async" draggable={false} onLoad={large ? undefined : () => setPhase("ready")} onError={large ? undefined : () => setPhase("error")}
        // A response that completed before this handler was attached is already decoded.
        ref={large ? undefined : (image) => { if (image?.complete && image.naturalWidth > 0) setPhase("ready"); }} />
    : kind === "video" ? <video src={url} controls playsInline preload="metadata" aria-label={label} onLoadedMetadata={large ? undefined : () => setPhase("ready")} onError={large ? undefined : () => setPhase("error")} />
    : kind === "audio" ? <audio src={url} controls preload="metadata" aria-label={label} onLoadedMetadata={() => setPhase("ready")} onError={() => setPhase("error")} />
    : <pre className="preview-text" tabIndex={0} aria-label={label}>{text?.text}</pre>;
  const truncated = text?.truncated ? <p className="preview-note">{t("Showing the first {0}. Download the file to read all of it.", formatBytes(TEXT_LIMIT, locale))}</p> : null;

  return <div className="preview" role="group" aria-label={t("Preview")} data-kind={kind ?? "none"} data-phase={empty ? "empty" : phase}>
    <p className="preview-label">{t("Preview")}</p>
    {empty ? <p className="preview-note">{t("This file is empty.")}</p>
      : phase === "unsupported" ? <p className="preview-note preview-unavailable"><FileQuestion size={17} aria-hidden="true" /><span>{t("No preview for this file type. Download it instead.")}</span></p>
      : phase === "error" ? <p className="preview-note error-text" role="status">{t("Could not load the preview. Download the file instead.")}</p>
      : phase === "waiting" ? <button type="button" className="button button-secondary preview-load" onClick={() => { setPhase("loading"); requestAnimationFrame(() => frame.current?.focus()); }}>{t("Load preview ({0})", formatBytes(entry.size, locale))}</button>
      : <>
        <div ref={frame} className="preview-frame" tabIndex={-1} aria-busy={phase === "loading"}>
          {phase === "loading" && <p className="preview-loading">{t("Loading preview…")}</p>}
          {(kind !== "text" || text) && media(false)}
        </div>
        {truncated}
        {phase === "ready" && (kind === "image" || kind === "text") && <button ref={enlarge} type="button" className="button button-secondary preview-enlarge" onClick={() => setEnlarged(true)}><Maximize2 size={16} aria-hidden="true" />{t("View larger")}</button>}
      </>}
    {enlarged && <LargePreview title={entry.name} opener={enlarge} close={() => setEnlarged(false)}>{media(true)}{truncated}</LargePreview>}
  </div>;
}

function LargePreview({ title, opener, close, children }: { title: string; opener: RefObject<HTMLButtonElement | null>; close: () => void; children: ReactNode }) {
  const { t } = useI18n();
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => element?.close();
  }, []);
  // Explicit dismissal owns the focus handoff back to the surviving opener.
  function dismiss() {
    const finishFocus = captureTaskFocus(dialog.current);
    close(); finishFocus(() => { if (opener.current?.isConnected) opener.current.focus(); });
  }
  return <dialog ref={dialog} className="preview-dialog" aria-labelledby={titleId} onCancel={(event) => { event.preventDefault(); dismiss(); }}
    // Only the backdrop reports the dialog itself as the click target.
    onClick={(event) => { if (event.target === dialog.current) dismiss(); }}>
    <div className="preview-dialog-heading"><h2 id={titleId} title={title}>{title}</h2><button type="button" className="icon-button" autoFocus aria-label={t("Close preview")} onClick={dismiss}><X size={18} aria-hidden="true" /></button></div>
    <div className="preview-dialog-body">{children}</div>
  </dialog>;
}

// Preview is a presentation of the files module's authorized download route.
export const previewModule: FeatureModule = {
  id: "preview", module: "files", requires: [],
  enabled: (context) => context.permissions.download && !!context.capabilities.inline_preview,
  Inspector: PreviewInspector,
};
