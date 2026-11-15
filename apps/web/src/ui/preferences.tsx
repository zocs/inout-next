"use client";
import { useI18n } from "@/i18n/provider";
import type { LocalePreference } from "@/i18n/preferences";
import { themes, useTheme, type Theme } from "./theme";
const themeNames = { evergreen: "Evergreen", tangerine: "Tangerine", ocean: "Ocean", berry: "Berry" } as const;
export function Preferences() {
  const { preference, setPreference, t } = useI18n(); const { theme, setTheme } = useTheme();
  return <div className="preferences">
    <label>{t("Language")}<select aria-label={t("Language")} value={preference} onChange={event => setPreference(event.target.value as LocalePreference)}>
      <option value="system">{t("System")}</option><option value="en">English</option><option value="zh-Hans">简体中文</option><option value="zh-Hant">繁體中文</option>
    </select></label>
    <label>{t("Theme")}<select aria-label={t("Theme")} value={theme} onChange={event => setTheme(event.target.value as Theme)}>{themes.map(value => <option key={value} value={value}>{t(themeNames[value])}</option>)}</select></label>
  </div>;
}
