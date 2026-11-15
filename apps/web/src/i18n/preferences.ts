import { locales, type Locale } from "./catalog";
export type LocalePreference = "system" | Locale;
export const localeKey = "share-ui.locale";
export function isLocale(value: unknown): value is Locale { return locales.some(locale => locale === value); }
export function isLocalePreference(value: unknown): value is LocalePreference { return value === "system" || isLocale(value); }
export function resolveLocale(languages: readonly string[], preference: LocalePreference): Locale {
  if (preference !== "system") return preference;
  for (const language of languages) {
    let tag: Intl.Locale;
    try { tag = new Intl.Locale(language.replaceAll("_", "-")); } catch { continue; }
    if (tag.language === "zh") {
      if (tag.script === "Hans") return "zh-Hans";
      if (tag.script === "Hant") return "zh-Hant";
      return tag.region === "TW" || tag.region === "HK" || tag.region === "MO" ? "zh-Hant" : "zh-Hans";
    }
    if (tag.language === "en") return "en";
  }
  return "en";
}
export function readPreference<T extends string>(key: string, valid: (value: unknown) => value is T, fallback: T): T {
  try { const stored = window.localStorage.getItem(key); return valid(stored) ? stored : fallback; } catch { return fallback; }
}
export function savePreference(key: string, value: string) { try { window.localStorage.setItem(key, value); } catch { /* Storage may be denied; the current session still works. */ } }
