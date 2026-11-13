export function formatBytes(bytes: number | null): string {
  if (bytes === null) return "Folder";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit++; }
  return `${value.toLocaleString(undefined, { maximumFractionDigits: value < 10 ? 1 : 0 })} ${units[unit]}`;
}

export function formatDate(seconds: number | null): string {
  return seconds === null ? "Unavailable" : new Date(seconds * 1000).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function joinPath(parent: string, name: string): string { return parent ? `${parent}/${name}` : name; }
export function parentPath(path: string): string { return path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : ""; }
