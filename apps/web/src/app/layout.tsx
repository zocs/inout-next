import type { Metadata } from "next";
import type { ReactNode } from "react";
import product from "@/generated/product.json";
import "./globals.css";

export const metadata: Metadata = { title: product.name, description: product.description };

export default function RootLayout({ children }: { children: ReactNode }) {
  return <html lang="en"><body>{children}</body></html>;
}
