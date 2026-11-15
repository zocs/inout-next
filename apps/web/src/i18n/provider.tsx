"use client";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { ApiError } from "@/lib/api";
import { catalogs, interpolate, type Catalog, type Locale, type MessageKey, type PresentationMessage } from "./catalog";
import { isLocalePreference, localeKey, readPreference, resolveLocale, savePreference, type LocalePreference } from "./preferences";
const knownErrors: Record<string, MessageKey> = {
  target_exists: "The destination already exists. Choose another name.", not_empty: "The folder is not empty. Confirm removal of all files and subfolders to delete it.", invalid_range: "The requested byte range is unavailable. Start the download again.", io_error: "The device could not complete the storage operation. Refresh and try again.",
  unauthorized: "Check the access key and unlock files again.", forbidden: "This action is not allowed by the sharing device.", invalid_path: "This path is invalid. Use a path inside the shared folder.", not_found: "The entry was not found. Refresh the folder.", conflict: "The destination already exists or the entry is busy. Refresh and try again.", unsupported: "This operation is not supported by the sharing device.", too_large: "The upload exceeds the device’s file size limit.", request_failed: "The request failed. Refresh the folder and try again.", connection_lost: "Cannot reach the sharing device. Check the connection and try again.", invalid_response: "The server did not confirm that the file was saved.", canceled: "Canceled",
};
interface I18nState { locale: Locale; preference: LocalePreference; catalog: Catalog; setPreference: (value: LocalePreference) => void; t: (key: MessageKey, ...args: (string | number)[]) => string; renderMessage: (message: PresentationMessage) => string }
const I18nContext = createContext<I18nState | null>(null);
export function I18nProvider({ children }: { children: ReactNode }) {
  const [preference, updatePreference] = useState<LocalePreference>("system");
  const [languages, setLanguages] = useState<readonly string[]>(["en"]);
  useEffect(() => {
    updatePreference(readPreference(localeKey, isLocalePreference, "system"));
    const update = () => setLanguages([...navigator.languages]); update();
    window.addEventListener("languagechange", update); return () => window.removeEventListener("languagechange", update);
  }, []);
  const locale = resolveLocale(languages, preference);
  useEffect(() => { document.documentElement.lang = locale; }, [locale]);
  const setPreference = useCallback((value: LocalePreference) => { if (isLocalePreference(value)) { updatePreference(value); savePreference(localeKey, value); } }, []);
  const state = useMemo<I18nState>(() => {
    const catalog = catalogs[locale];
    const t = (key: MessageKey, ...args: (string | number)[]) => interpolate(catalog[key], args);
    const renderMessage = (message: PresentationMessage): string => {
      if (typeof message === "string") return message;
      if (message instanceof ApiError && Object.hasOwn(knownErrors, message.code)) return t(knownErrors[message.code]);
      if (message instanceof Error) return message.message;
      return interpolate(catalog[message.key], message.args);
    };
    return { locale, preference, catalog, setPreference, t, renderMessage };
  }, [locale, preference, setPreference]);
  return <I18nContext.Provider value={state}>{children}</I18nContext.Provider>;
}
export function useI18n(): I18nState { const state = useContext(I18nContext); if (!state) throw new Error("Missing I18nProvider"); return state; }
