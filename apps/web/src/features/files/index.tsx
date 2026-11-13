import { Download, File, Folder, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { formatBytes, formatDate } from "@/lib/format";
import type { FeatureModule, ModuleProps } from "@/ui/modules";

function FileView({ runtime }: ModuleProps) {
  const [filter, setFilter] = useState("");
  const entries = useMemo(() => runtime.entries
    .filter((entry) => entry.name.toLocaleLowerCase().includes(filter.toLocaleLowerCase()))
    .sort((a, b) => a.kind === b.kind ? a.name.localeCompare(b.name, undefined, { numeric: true }) : a.kind === "directory" ? -1 : 1), [runtime.entries, filter]);
  return <section className="file-view" aria-label="Folder contents">
    <div className="list-controls">
      <label className="search-field"><Search size={17} aria-hidden="true" /><span className="visually-hidden">Search this folder</span>
        <input type="search" placeholder="Search this folder" value={filter} onChange={(event) => setFilter(event.target.value)} />
      </label>
      <span className="item-count" aria-live="polite">{runtime.loading ? "Loading files…" : `${entries.length} ${entries.length === 1 ? "item" : "items"}`}</span>
    </div>
    <div className="table-scroll">
      <table aria-label="Shared files" aria-busy={runtime.loading}>
        <thead><tr><th scope="col">Name</th><th scope="col" className="size-column">Size</th><th scope="col" className="modified-column">Modified</th><th scope="col" className="download-column"><span className="visually-hidden">Download</span></th></tr></thead>
        <tbody>{entries.map((entry) => <tr key={entry.path} className={runtime.selected?.path === entry.path ? "selected-row" : ""}>
          <td><button className="entry-name" title={entry.name} aria-label={`${entry.kind === "directory" ? "Open" : "Select"} ${entry.name}`}
            aria-pressed={entry.kind === "file" ? runtime.selected?.path === entry.path : undefined}
            onClick={() => entry.kind === "directory" ? runtime.navigate(entry.path) : runtime.select(entry)}>
            {entry.kind === "directory" ? <Folder size={19} aria-hidden="true" /> : <File size={19} aria-hidden="true" />}<span>{entry.name}</span>
          </button>
          {entry.kind === "directory" && runtime.context.permissions.manage && <button className="folder-details" aria-label={`Select ${entry.name}`} aria-pressed={runtime.selected?.path === entry.path} onClick={() => runtime.select(entry)}>Details</button>}</td>
          <td className="size-column">{formatBytes(entry.size)}</td><td className="modified-column">{formatDate(entry.modified)}</td>
          <td className="download-column">{entry.kind === "file" && runtime.context.permissions.download && <a className="icon-button" href={runtime.api.downloadUrl(entry.path)} download={entry.name} aria-label={`Download ${entry.name}`} title={`Download ${entry.name}`}><Download size={18} aria-hidden="true" /></a>}</td>
        </tr>)}</tbody>
      </table>
    </div>
    {runtime.loading && entries.length === 0 && <div className="loading-rows" aria-hidden="true"><div /><div /><div /></div>}
    {!runtime.loading && entries.length === 0 && <div className="empty-state"><Folder size={30} aria-hidden="true" /><h3>{filter ? "No matching files" : "This folder is empty"}</h3><p>{filter ? "Try another name, or clear the search." : runtime.context.permissions.upload
      ? runtime.context.permissions.manage ? "Upload a file or create a folder to get started." : "Upload a file to get started."
      : runtime.context.permissions.manage ? "Create a folder to get started." : "Files will appear here when the sharing device adds them."}</p></div>}
  </section>;
}

export const filesModule: FeatureModule = {
  id: "files", requires: [], enabled: (context) => context.permissions.list, View: FileView,
};
