"use client";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { readPreference, savePreference } from "@/i18n/preferences";
export const themes = ["evergreen", "tangerine", "ocean", "berry"] as const;
export type Theme = typeof themes[number];
const themeKey = "share-ui.theme";
const isTheme = (value: unknown): value is Theme => themes.some(theme => theme === value);
const ThemeContext = createContext<{ theme: Theme; setTheme: (value: Theme) => void } | null>(null);
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, updateTheme] = useState<Theme>("evergreen");
  useEffect(() => { updateTheme(readPreference(themeKey, isTheme, "evergreen")); }, []);
  useEffect(() => { document.documentElement.dataset.theme = theme; }, [theme]);
  const setTheme = useCallback((value: Theme) => { if (isTheme(value)) { updateTheme(value); savePreference(themeKey, value); } }, []);
  const state = useMemo(() => ({ theme, setTheme }), [theme, setTheme]);
  return <ThemeContext.Provider value={state}>{children}</ThemeContext.Provider>;
}
export function useTheme() { const state = useContext(ThemeContext); if (!state) throw new Error("Missing ThemeProvider"); return state; }
