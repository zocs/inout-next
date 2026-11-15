import { describe, presentError, type PresentationMessage } from "@/i18n/catalog";
import { useI18n } from "@/i18n/provider";
import { ArrowRight, Copy, FolderPlus, Pencil, Trash2 } from "lucide-react";
import { createContext, useContext, useEffect, useRef, useState, type FormEvent, type RefObject } from "react";
import { joinPath, parentPath } from "@/lib/format";
import type { Entry, Operation } from "@/lib/contracts";
import type { FeatureModule, ModuleProps, ModuleProviderProps } from "@/ui/modules";
import { captureTaskFocus } from "@/ui/focus";

interface ManageState { creating: boolean; setCreating: (value: boolean) => void; createOpener: RefObject<HTMLButtonElement | null> }
const ManageContext = createContext<ManageState | null>(null);

function useManage(): ManageState {
  const state = useContext(ManageContext);
  if (!state) throw new Error("Management contribution needs its module provider");
  return state;
}

function ManageProvider({ children }: ModuleProviderProps) {
  const [creating, setCreating] = useState(false);
  const createOpener = useRef<HTMLButtonElement>(null);
  return <ManageContext.Provider value={{ creating, setCreating, createOpener }}>{children}</ManageContext.Provider>;
}

function ManageToolbar({ runtime }: ModuleProps) {
  const { t } = useI18n();
  const { setCreating, createOpener } = useManage();
  return <button ref={createOpener} className="button button-secondary" onClick={() => { runtime.select(null); setCreating(true); }}><FolderPlus size={18} aria-hidden="true" />{t("New folder")}</button>;
}

function validName(name: string): boolean { return !!name.trim() && !/[\/\\]/.test(name); }

function ManagePanel({ runtime }: ModuleProps) {
  const { t, renderMessage } = useI18n();
  const { creating, setCreating, createOpener } = useManage();
  const form = useRef<HTMLFormElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const actionOpener = useRef<HTMLButtonElement | null>(null);
  const [action, setAction] = useState<Operation | "delete" | null>(null);
  const [value, setValue] = useState("");
  const [folder, setFolder] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<PresentationMessage>("");
  const selected = runtime.selected;
  useEffect(() => { setAction(null); setValue(""); setError(""); }, [selected?.path]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) return;
    if ((creating || action === "rename") && !validName(creating ? folder : value)) {
      setError(describe("Enter a name without slashes. Use Move to change folders.")); return;
    }
    if (!creating && !value.trim()) { setError(describe("Enter a destination relative to the shared folder.")); return; }
    const finishFocus = captureTaskFocus(event.currentTarget);
    let destination = () => input.current?.focus();
    setBusy(true); setError("");
    try {
      if (creating) {
        await runtime.api.mkdir(joinPath(runtime.path, folder.trim()));
        setCreating(false); setFolder(""); runtime.notify(describe("Folder created."));
      } else if (selected && action && action !== "delete") {
        const destination = action === "rename" ? joinPath(parentPath(selected.path), value.trim()) : value.trim();
        await runtime.api.operate(action, selected.path, destination);
        runtime.select(null); setAction(null); runtime.notify(describe(action === "copy" ? "File copied." : "Entry updated."));
      }
      destination = runtime.focusCurrentFolder;
      runtime.refresh();
    } catch (error) { setError(presentError(error, "The action failed. Refresh and try again.")); }
    finally { setBusy(false); finishFocus(destination); }
  }

  function edit(next: Operation | "delete", opener: HTMLButtonElement) {
    actionOpener.current = opener;
    setCreating(false); setError(""); setAction(next); setValue(next === "rename" ? selected?.name ?? "" : "");
  }

  function cancel() {
    const finishFocus = captureTaskFocus(form.current);
    const opener = creating ? createOpener.current : actionOpener.current;
    setCreating(false); setAction(null); setError("");
    finishFocus(() => { if (opener?.isConnected) opener.focus(); });
  }

  const canMove = selected?.kind === "file" ? runtime.context.capabilities.file_move : runtime.context.capabilities.directory_move;
  const canCopy = selected?.kind === "file" || runtime.context.capabilities.directory_copy;
  if (!creating && !selected) return null;
  return <section className="manage-panel" aria-label={t("File actions")}>
    {creating ? <><h3>{t("New folder")}</h3><form ref={form} onSubmit={submit}>
      <label className="field">{t("Folder name")}<input ref={input} autoFocus required value={folder} onChange={(event) => setFolder(event.target.value)} /></label>
      <p className="muted">{t("In {0}", runtime.path || t("the shared folder"))}</p>
      {error && <p role="alert" className="error-text">{renderMessage(error)}</p>}
      <div className="form-actions"><button className="button button-primary" disabled={busy}>{busy ? t("Creating…") : t("Create folder")}</button><button className="button button-secondary" type="button" disabled={busy} onClick={cancel}>{t("Cancel")}</button></div>
    </form></> : <>
      <div className="entry-actions">
        {canMove && <button className="button button-secondary" disabled={busy} onClick={(event) => edit("rename", event.currentTarget)}><Pencil size={16} aria-hidden="true" />{t("Rename")}</button>}
        {canMove && <button className="button button-secondary" disabled={busy} onClick={(event) => edit("move", event.currentTarget)}><ArrowRight size={16} aria-hidden="true" />{t("Move")}</button>}
        {canCopy && <button className="button button-secondary" disabled={busy} onClick={(event) => edit("copy", event.currentTarget)}><Copy size={16} aria-hidden="true" />{t("Copy")}</button>}
        <button className="button button-danger-quiet" disabled={busy} onClick={(event) => edit("delete", event.currentTarget)}><Trash2 size={16} aria-hidden="true" />{t("Delete")}</button>
      </div>
      {action && action !== "delete" && <form ref={form} className="operation-form" onSubmit={submit}>
        <label className="field">{action === "rename" ? t("New name") : t("Destination path")}<input ref={input} key={action} autoFocus required value={value} onChange={(event) => setValue(event.target.value)} /></label>
        {action !== "rename" && <p className="muted">{t("Relative to the shared folder, including the file name. Existing files are preserved.")}</p>}
        {error && <p role="alert" className="error-text">{renderMessage(error)}</p>}
        <div className="form-actions"><button className="button button-primary" disabled={busy}>{busy ? t("Working…") : action === "rename" ? t("Save name") : t(action === "move" ? selected?.kind === "directory" ? "Move folder" : "Move file" : selected?.kind === "directory" ? "Copy folder" : "Copy file")}</button>
          <button type="button" className="button button-secondary" disabled={busy} onClick={cancel}>{t("Cancel")}</button></div>
      </form>}
      {action === "delete" && selected && <DeleteDialog entry={selected} runtime={runtime} opener={actionOpener.current} close={() => setAction(null)} />}
    </>}
  </section>;
}

function DeleteDialog({ entry, runtime, close, opener }: ModuleProps & { entry: Entry; close: () => void; opener: HTMLButtonElement | null }) {
  const { t, renderMessage } = useI18n();
  const dialog = useRef<HTMLDialogElement>(null);
  const cancelControl = useRef<HTMLButtonElement>(null);
  const [recursive, setRecursive] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<PresentationMessage>("");
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => element?.close();
  }, []);
  function cancel() {
    const finishFocus = captureTaskFocus(dialog.current);
    close(); finishFocus(() => { if (opener?.isConnected) opener.focus(); });
  }
  async function remove(event: FormEvent) {
    event.preventDefault(); if (busy) return;
    const finishFocus = captureTaskFocus(dialog.current);
    let destination = () => cancelControl.current?.focus();
    setBusy(true); setError("");
    try { await runtime.api.remove(entry.path, recursive); runtime.select(null); runtime.refresh(); runtime.notify(describe("Entry deleted.")); close(); destination = runtime.focusCurrentFolder; }
    catch (error) { setError(presentError(error, "Could not delete this entry.")); }
    finally { setBusy(false); finishFocus(destination); }
  }
  return <dialog ref={dialog} aria-labelledby="delete-title" onCancel={(event) => { event.preventDefault(); if (!busy) cancel(); }}>
    <form onSubmit={remove}><h2 id="delete-title">{t("Delete {0}?", entry.name)}</h2><p>{t(entry.kind === "directory" ? "This permanently removes the folder from the sharing device." : "This permanently removes the file from the sharing device.")}</p>
      {entry.kind === "directory" && <label className="checkbox-field"><input type="checkbox" checked={recursive} onChange={(event) => setRecursive(event.target.checked)} disabled={busy} />{t("Include all files and subfolders")}</label>}
      {error && <p role="alert" className="error-text">{renderMessage(error)}</p>}
      <div className="form-actions"><button ref={cancelControl} className="button button-secondary" type="button" autoFocus disabled={busy} onClick={cancel}>{t("Cancel")}</button><button className="button button-danger" disabled={busy}>{busy ? t("Deleting…") : t("Delete permanently")}</button></div>
    </form>
  </dialog>;
}

export const manageModule: FeatureModule = {
  id: "manage", requires: ["files"], enabled: (context) => context.permissions.manage,
  Provider: ManageProvider, Toolbar: ManageToolbar, SelectionPanel: ManagePanel,
};
