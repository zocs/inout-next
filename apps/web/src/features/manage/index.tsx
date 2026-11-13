import { ArrowRight, Copy, FolderPlus, Pencil, Trash2 } from "lucide-react";
import { createContext, useContext, useEffect, useRef, useState, type FormEvent } from "react";
import { joinPath, parentPath } from "@/lib/format";
import type { Entry, Operation } from "@/lib/contracts";
import type { FeatureModule, ModuleProps, ModuleProviderProps } from "@/ui/modules";

interface ManageState { creating: boolean; setCreating: (value: boolean) => void }
const ManageContext = createContext<ManageState | null>(null);

function useManage(): ManageState {
  const state = useContext(ManageContext);
  if (!state) throw new Error("Management contribution needs its module provider");
  return state;
}

function ManageProvider({ children }: ModuleProviderProps) {
  const [creating, setCreating] = useState(false);
  return <ManageContext.Provider value={{ creating, setCreating }}>{children}</ManageContext.Provider>;
}

function ManageToolbar({ runtime }: ModuleProps) {
  const { setCreating } = useManage();
  return <button className="button button-secondary" onClick={() => { runtime.select(null); setCreating(true); }}><FolderPlus size={18} aria-hidden="true" />New folder</button>;
}

function validName(name: string): boolean { return !!name.trim() && !/[\/\\]/.test(name); }

function ManagePanel({ runtime }: ModuleProps) {
  const { creating, setCreating } = useManage();
  const [action, setAction] = useState<Operation | "delete" | null>(null);
  const [value, setValue] = useState("");
  const [folder, setFolder] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const selected = runtime.selected;
  useEffect(() => { setAction(null); setValue(""); setError(""); }, [selected?.path]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (busy) return;
    if ((creating || action === "rename") && !validName(creating ? folder : value)) {
      setError("Enter a name without slashes. Use Move to change folders."); return;
    }
    if (!creating && !value.trim()) { setError("Enter a destination relative to the shared folder."); return; }
    setBusy(true); setError("");
    try {
      if (creating) {
        await runtime.api.mkdir(joinPath(runtime.path, folder.trim()));
        setCreating(false); setFolder(""); runtime.notify("Folder created.");
      } else if (selected && action && action !== "delete") {
        const destination = action === "rename" ? joinPath(parentPath(selected.path), value.trim()) : value.trim();
        await runtime.api.operate(action, selected.path, destination);
        runtime.select(null); setAction(null); runtime.notify(action === "copy" ? "File copied." : "Entry updated.");
      }
      runtime.refresh();
    } catch (error) { setError(error instanceof Error ? error.message : "The action failed. Refresh and try again."); }
    finally { setBusy(false); }
  }

  function edit(next: Operation | "delete") {
    setCreating(false); setError(""); setAction(next); setValue(next === "rename" ? selected?.name ?? "" : "");
  }

  const canMove = selected?.kind === "file" ? runtime.context.capabilities.file_move : runtime.context.capabilities.directory_move;
  const canCopy = selected?.kind === "file" || runtime.context.capabilities.directory_copy;
  if (!creating && !selected) return null;
  return <section className="manage-panel" aria-label="File actions">
    {creating ? <><h3>New folder</h3><form onSubmit={submit}>
      <label className="field">Folder name<input autoFocus required value={folder} onChange={(event) => setFolder(event.target.value)} /></label>
      <p className="muted">In {runtime.path || "the shared folder"}</p>
      {error && <p role="alert" className="error-text">{error}</p>}
      <div className="form-actions"><button className="button button-primary" disabled={busy}>{busy ? "Creating…" : "Create folder"}</button><button className="button button-secondary" type="button" disabled={busy} onClick={() => { setCreating(false); setError(""); }}>Cancel</button></div>
    </form></> : <>
      <div className="entry-actions">
        {canMove && <button className="button button-secondary" disabled={busy} onClick={() => edit("rename")}><Pencil size={16} aria-hidden="true" />Rename</button>}
        {canMove && <button className="button button-secondary" disabled={busy} onClick={() => edit("move")}><ArrowRight size={16} aria-hidden="true" />Move</button>}
        {canCopy && <button className="button button-secondary" disabled={busy} onClick={() => edit("copy")}><Copy size={16} aria-hidden="true" />Copy</button>}
        <button className="button button-danger-quiet" disabled={busy} onClick={() => edit("delete")}><Trash2 size={16} aria-hidden="true" />Delete</button>
      </div>
      {action && action !== "delete" && <form className="operation-form" onSubmit={submit}>
        <label className="field">{action === "rename" ? "New name" : "Destination path"}<input key={action} autoFocus required value={value} onChange={(event) => setValue(event.target.value)} /></label>
        {action !== "rename" && <p className="muted">Relative to the shared folder, including the file name. Existing files are preserved.</p>}
        {error && <p role="alert" className="error-text">{error}</p>}
        <div className="form-actions"><button className="button button-primary" disabled={busy}>{busy ? "Working…" : action === "rename" ? "Save name" : `${action === "move" ? "Move" : "Copy"} ${selected?.kind === "directory" ? "folder" : "file"}`}</button>
          <button type="button" className="button button-secondary" disabled={busy} onClick={() => { setAction(null); setError(""); }}>Cancel</button></div>
      </form>}
      {action === "delete" && selected && <DeleteDialog entry={selected} runtime={runtime} close={() => setAction(null)} />}
    </>}
  </section>;
}

function DeleteDialog({ entry, runtime, close }: ModuleProps & { entry: Entry; close: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [recursive, setRecursive] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => element?.close();
  }, []);
  async function remove(event: FormEvent) {
    event.preventDefault(); setBusy(true); setError("");
    try { await runtime.api.remove(entry.path, recursive); runtime.select(null); runtime.refresh(); runtime.notify("Entry deleted."); close(); }
    catch (error) { setError(error instanceof Error ? error.message : "Could not delete this entry."); }
    finally { setBusy(false); }
  }
  return <dialog ref={dialog} aria-labelledby="delete-title" onCancel={(event) => { event.preventDefault(); if (!busy) close(); }}>
    <form onSubmit={remove}><h2 id="delete-title">Delete {entry.name}?</h2><p>This permanently removes the {entry.kind === "directory" ? "folder" : "file"} from the sharing device.</p>
      {entry.kind === "directory" && <label className="checkbox-field"><input type="checkbox" checked={recursive} onChange={(event) => setRecursive(event.target.checked)} disabled={busy} />Include all files and subfolders</label>}
      {error && <p role="alert" className="error-text">{error}</p>}
      <div className="form-actions"><button className="button button-secondary" type="button" autoFocus disabled={busy} onClick={close}>Cancel</button><button className="button button-danger" disabled={busy}>{busy ? "Deleting…" : "Delete permanently"}</button></div>
    </form>
  </dialog>;
}

export const manageModule: FeatureModule = {
  id: "manage", requires: ["files"], enabled: (context) => context.permissions.manage,
  Provider: ManageProvider, Toolbar: ManageToolbar, Panel: ManagePanel,
};
