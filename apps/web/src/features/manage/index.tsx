import { describe, presentError, type PresentationMessage } from "@/i18n/catalog";
import { useI18n } from "@/i18n/provider";
import { ArrowRight, Copy, FolderPlus, Pencil, Trash2 } from "lucide-react";
import { createContext, useContext, useEffect, useLayoutEffect, useRef, useState, type FormEvent, type RefObject } from "react";
import { joinPath, parentPath } from "@/lib/format";
import type { Entry, Operation } from "@/lib/contracts";
import type { FeatureModule, ModuleProps, ModuleProviderProps } from "@/ui/modules";
import { captureTaskFocus } from "@/ui/focus";

interface ManageState { creating: boolean; creationId: number; setCreating: (value: boolean) => void; createOpener: RefObject<HTMLButtonElement | null> }
const ManageContext = createContext<ManageState | null>(null);

function useManage(): ManageState {
  const state = useContext(ManageContext);
  if (!state) throw new Error("Management contribution needs its module provider");
  return state;
}

function ManageProvider({ runtime, children }: ModuleProviderProps) {
  const [creating, updateCreating] = useState(false);
  const [creationId, setCreationId] = useState(0);
  const createOpener = useRef<HTMLButtonElement>(null);
  useEffect(() => { updateCreating(false); }, [runtime.path]);
  const setCreating = (value: boolean) => {
    if (value) setCreationId((current) => current + 1);
    updateCreating(value);
  };
  return <ManageContext.Provider value={{ creating, creationId, setCreating, createOpener }}>{children}</ManageContext.Provider>;
}

// A request owns only the form that submitted it. Navigation, a new selection,
// reopening creation and unmounting release that ownership before another task.
function useManagementTask(path: string, entryPath?: string, creationId?: number) {
  const current = useRef<object | null>(null);
  const [busy, setBusy] = useState(false);
  useLayoutEffect(() => {
    current.current = null; setBusy(false);
    return () => { current.current = null; };
  }, [path, entryPath, creationId]);
  return {
    busy,
    begin: () => {
      if (current.current) return null;
      const task = {}; current.current = task; setBusy(true); return task;
    },
    owns: (task: object) => current.current === task,
    finish: (task: object) => { if (current.current === task) { current.current = null; setBusy(false); } },
  };
}

function ManageToolbar({ runtime }: ModuleProps) {
  const { t } = useI18n();
  const { setCreating, createOpener } = useManage();
  if (!runtime.context.permissions.manage) return null;
  return <button ref={createOpener} className="button button-secondary" onClick={() => { runtime.select(null); setCreating(true); }}><FolderPlus size={18} aria-hidden="true" />{t("New folder")}</button>;
}

function validName(name: string): boolean { return !!name.trim() && !/[\/\\]/.test(name); }

function ManagePanel({ runtime }: ModuleProps) {
  const { t, renderMessage } = useI18n();
  const { creating, creationId, setCreating, createOpener } = useManage();
  const form = useRef<HTMLFormElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const actionOpener = useRef<HTMLButtonElement | null>(null);
  const [action, setAction] = useState<Operation | "delete" | null>(null);
  const [value, setValue] = useState("");
  const [folder, setFolder] = useState("");
  const [error, setError] = useState<PresentationMessage>("");
  const selected = runtime.selected;
  const tasks = useManagementTask(runtime.path, selected?.path, creating ? creationId : undefined);
  const busy = tasks.busy;
  useEffect(() => { setAction(null); setValue(""); setError(""); }, [selected?.path]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) return;
    if ((creating || action === "rename") && !validName(creating ? folder : value)) {
      setError(describe("Enter a name without slashes. Use Move to change folders.")); return;
    }
    if (!creating && !value.trim()) { setError(describe("Enter a destination relative to the shared folder.")); return; }
    const task = tasks.begin();
    if (!task) return;
    const finishFocus = captureTaskFocus(event.currentTarget);
    let destination = () => input.current?.focus();
    setError("");
    try {
      if (creating) {
        await runtime.api.mkdir(joinPath(runtime.path, folder.trim()));
        runtime.refresh();
        if (!tasks.owns(task)) return;
        setCreating(false); setFolder(""); runtime.notify(describe("Folder created."));
      } else if (selected && action && action !== "delete") {
        const destination = action === "rename" ? joinPath(parentPath(selected.path), value.trim()) : value.trim();
        await runtime.api.operate(action, selected.path, destination);
        runtime.refresh();
        if (!tasks.owns(task)) return;
        runtime.select(null); setAction(null); runtime.notify(describe(action === "copy" ? "File copied." : "Entry updated."));
      }
      destination = runtime.focusCurrentFolder;
    } catch (error) { if (tasks.owns(task)) setError(presentError(error, "The action failed. Refresh and try again.")); }
    finally { const owned = tasks.owns(task); tasks.finish(task); finishFocus(owned ? destination : () => {}); }
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

  const permissions = runtime.context.permissions;
  const movable = selected?.kind === "file" ? runtime.context.capabilities.file_move : runtime.context.capabilities.directory_move;
  const canRename = movable && (permissions.rename ?? permissions.manage);
  const canMove = movable && permissions.manage;
  const canCopy = permissions.manage && (selected?.kind === "file" || runtime.context.capabilities.directory_copy);
  const canDelete = permissions.delete ?? permissions.manage;
  if (!creating && !selected) return null;
  return <section className="manage-panel" aria-label={t("File actions")}>
    {creating ? <><h3>{t("New folder")}</h3><form ref={form} onSubmit={submit}>
      <label className="field">{t("Folder name")}<input ref={input} autoFocus required disabled={busy} value={folder} onChange={(event) => setFolder(event.target.value)} /></label>
      <p className="muted">{t("In {0}", runtime.path || t("the shared folder"))}</p>
      {error && <p role="alert" className="error-text">{renderMessage(error)}</p>}
      <div className="form-actions"><button className="button button-primary" disabled={busy}>{busy ? t("Creating…") : t("Create folder")}</button><button className="button button-secondary" type="button" disabled={busy} onClick={cancel}>{t("Cancel")}</button></div>
    </form></> : <>
      <div className="entry-actions">
        {canRename && <button className="button button-secondary" disabled={busy} onClick={(event) => edit("rename", event.currentTarget)}><Pencil size={16} aria-hidden="true" />{t("Rename")}</button>}
        {canMove && <button className="button button-secondary" disabled={busy} onClick={(event) => edit("move", event.currentTarget)}><ArrowRight size={16} aria-hidden="true" />{t("Move")}</button>}
        {canCopy && <button className="button button-secondary" disabled={busy} onClick={(event) => edit("copy", event.currentTarget)}><Copy size={16} aria-hidden="true" />{t("Copy")}</button>}
        {canDelete && <button className="button button-danger-quiet" disabled={busy} onClick={(event) => edit("delete", event.currentTarget)}><Trash2 size={16} aria-hidden="true" />{t("Delete")}</button>}
      </div>
      {action && action !== "delete" && <form ref={form} className="operation-form" onSubmit={submit}>
        <label className="field">{action === "rename" ? t("New name") : t("Destination path")}<input ref={input} key={action} autoFocus required disabled={busy} value={value} onChange={(event) => setValue(event.target.value)} /></label>
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
  const tasks = useManagementTask(runtime.path, entry.path);
  const busy = tasks.busy;
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
    const task = tasks.begin();
    if (!task) return;
    const finishFocus = captureTaskFocus(dialog.current);
    let destination = () => cancelControl.current?.focus();
    setError("");
    try {
      await runtime.api.remove(entry.path, recursive); runtime.refresh();
      if (!tasks.owns(task)) return;
      runtime.select(null); runtime.notify(describe("Entry deleted.")); close(); destination = runtime.focusCurrentFolder;
    }
    catch (error) { if (tasks.owns(task)) setError(presentError(error, "Could not delete this entry.")); }
    finally { const owned = tasks.owns(task); tasks.finish(task); finishFocus(owned ? destination : () => {}); }
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
  id: "manage", requires: ["files"], enabled: (context) => context.permissions.manage || !!context.permissions.rename || !!context.permissions.delete,
  Provider: ManageProvider, Toolbar: ManageToolbar, SelectionPanel: ManagePanel,
};
