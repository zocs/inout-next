import { locales, type Locale } from "./catalog";
export type LocalePreference = "system" | Locale;
export const localeKey = "share-ui.locale";
export function isLocale(value: unknown): value is Locale { return locales.some(locale => locale === value); }
export function isLocalePreference(value: unknown): value is LocalePreference { return value === "system" || isLocale(value); }
export function resolveLocale(languages: readonly string[], preference: LocalePreference): Locale {
  if (preference !== "system") return preference;
  for (const language of languages) {
    const tag = language.toLowerCase().replaceAll("_", "-");
    if (/^zh(?:-|$)/.test(tag)) {
      if (/(?:^|-)hans(?:-|$)/.test(tag)) return "zh-Hans";
      if (/(?:^|-)hant(?:-|$)/.test(tag)) return "zh-Hant";
      return /(?:^|-)(?:tw|hk|mo)(?:-|$)/.test(tag) ? "zh-Hant" : "zh-Hans";
    }
    if (/^en(?:-|$)/.test(tag)) return "en";
  }
  return "en";
}
export function readPreference<T extends string>(key: string, valid: (value: unknown) => value is T, fallback: T): T {
  try { const stored = window.localStorage.getItem(key); return valid(stored) ? stored : fallback; } catch { return fallback; }
}
export function savePreference(key: string, value: string) { try { window.localStorage.setItem(key, value); } catch { /* Storage may be denied; the current session still works. */ } }
