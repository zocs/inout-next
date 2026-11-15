import { catalogs, type Locale } from "@/i18n/catalog";
const numbers = new Map<string, Intl.NumberFormat>();
const dates = new Map<Locale, Intl.DateTimeFormat>();
export function formatNumber(value: number, locale: Locale = "en", maximumFractionDigits = 0, fixed = false): string {
  const digits = Math.max(0, Math.min(2, maximumFractionDigits));
  const key = `${locale}:${digits}:${fixed}`;
  let formatter = numbers.get(key);
  if (!formatter) { formatter = new Intl.NumberFormat(locale, { maximumFractionDigits: digits, minimumFractionDigits: fixed ? digits : 0 }); numbers.set(key, formatter); }
  return formatter.format(value);
}
export function formatBytes(bytes: number | null, locale: Locale = "en"): string {
  if (bytes === null) return catalogs[locale].Folder;
  if (bytes < 1024) return `${formatNumber(bytes, locale)} B`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes / 1024, unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit++; }
  return `${formatNumber(value, locale, value < 10 ? 1 : 0)} ${units[unit]}`;
}
export function formatDate(seconds: number | null, locale: Locale = "en"): string {
  if (seconds === null) return catalogs[locale].Unavailable;
  let formatter = dates.get(locale);
  if (!formatter) { formatter = new Intl.DateTimeFormat(locale, { year: "numeric", month: "short", day: "numeric" }); dates.set(locale, formatter); }
  return formatter.format(new Date(seconds * 1000));
}
export function joinPath(parent: string, name: string): string { return parent ? `${parent}/${name}` : name; }
export function parentPath(path: string): string { return path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : ""; }
