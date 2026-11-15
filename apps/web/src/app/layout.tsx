import type { Metadata } from "next";
import type { ReactNode } from "react";
import product from "@/generated/product.json";
import "./globals.css";
import { I18nProvider } from "@/i18n/provider";
import { ThemeProvider } from "@/ui/theme";

export const metadata: Metadata = { title: product.name, description: product.description };

export default function RootLayout({ children }: { children: ReactNode }) {
  return <html lang="en"><body><ThemeProvider><I18nProvider>{children}</I18nProvider></ThemeProvider></body></html>;
}
