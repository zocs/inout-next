"use client";

import { useEffect, useState } from "react";
import FileManager from "@/ui/file-manager";
import NativeHost from "@/ui/native-host";

export default function Page() {
  const [native, setNative] = useState<boolean | null>(null);
  useEffect(() => {
    const url = new URL(window.location.href);
    const internal = url.protocol === "tauri:" || url.hostname === "tauri.localhost";
    const development = url.protocol === "http:" && url.hostname === "localhost" && url.port === "3100"
      && "__TAURI_INTERNALS__" in window;
    setNative(internal || development);
  }, []);
  if (native === null) return <main className="connection-state" aria-busy="true"><h2>Opening…</h2></main>;
  return native ? <NativeHost /> : <FileManager />;
}
