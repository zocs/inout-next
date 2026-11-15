"use client";

import { useI18n } from "@/i18n/provider";

import { useCallback, useEffect, useState } from "react";
import FileManager from "@/ui/file-manager";
import NativeHost from "@/ui/native-host";
import Startup from "@/ui/startup";
import productDefaults from "@/generated/product.json";
import type { Product } from "@/lib/contracts";

export default function Page() {
  const { t } = useI18n();
  const [native, setNative] = useState<boolean | null>(null);
  const [ready, setReady] = useState(false);
  const [openingProduct, setOpeningProduct] = useState<Product>(productDefaults);
  const finishOpening = useCallback((product?: Product) => {
    if (product) setOpeningProduct(product);
    setReady(true);
  }, []);
  useEffect(() => {
    const url = new URL(window.location.href);
    const internal = url.protocol === "tauri:" || url.hostname === "tauri.localhost";
    const development = url.protocol === "http:" && url.hostname === "localhost" && url.port === "3100"
      && "__TAURI_INTERNALS__" in window;
    setNative(internal || development);
  }, []);
  return <>
    <Startup ready={ready} name={openingProduct.name} />
    {native === null ? <main className="connection-state" aria-busy="true"><h2>{t("Opening…")}</h2></main>
      : native ? <NativeHost onReady={finishOpening} /> : <FileManager onReady={finishOpening} />}
  </>;
}
