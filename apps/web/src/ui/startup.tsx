"use client";

import { useCallback, useEffect, useState } from "react";
import { Wordmark } from "./branding";

const exitDuration = 160;

// `ready` means the initial setup, authentication, content or recovery UI can
// be used. It is independent of whether a native sharing service is running.
export default function Startup({ ready, name }: { ready: boolean; name: string }) {
  const [dismissed, setDismissed] = useState(false);
  const [finished, setFinished] = useState(false);
  const dismiss = useCallback(() => setDismissed(true), []);
  const finish = useCallback(() => setFinished(true), []);
  return finished ? null : <Presentation ready={ready || dismissed} name={name} dismiss={dismiss} finish={finish} />;
}

function Presentation({ ready, name, dismiss, finish }: { ready: boolean; name: string; dismiss: () => void; finish: () => void }) {
  useEffect(() => {
    if (ready) return;
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape") dismiss(); };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [ready, dismiss]);

  useEffect(() => {
    if (!ready) return;
    const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
    if (motion.matches) { finish(); return; }
    // This timer only removes an already inactive presentation. Readiness and
    // keyboard release do not wait for it or for transition/animation events.
    const timer = window.setTimeout(finish, exitDuration);
    const reduce = (event: MediaQueryListEvent) => { if (event.matches) finish(); };
    motion.addEventListener("change", reduce);
    return () => { window.clearTimeout(timer); motion.removeEventListener("change", reduce); };
  }, [ready, finish]);

  return <div className="startup-screen" data-state={ready ? "leaving" : "opening"}
    role={ready ? undefined : "status"} aria-label={ready ? undefined : `Opening ${name}`} aria-hidden={ready || undefined}>
    <div className="startup-wordmark"><Wordmark name={name} /></div>
    <p>Opening…</p>
  </div>;
}
