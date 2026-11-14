"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Wordmark } from "./branding";

function milliseconds(cssTime: string) {
  const value = Number.parseFloat(cssTime);
  return Number.isFinite(value) ? value * (cssTime.trim().endsWith("ms") ? 1 : 1_000) : 0;
}

function exitDuration(screen: HTMLDivElement) {
  const elements = [screen, ...screen.querySelectorAll(".startup-wordmark")];
  const timings = elements.flatMap((element) => {
    // Computed transition values resolve the real CSS token (including calc
    // and inherited overrides) rather than keeping a second duration in JS.
    const style = getComputedStyle(element);
    const delays = style.transitionDelay.split(",").map(milliseconds);
    return style.transitionDuration.split(",").map((duration, index) => milliseconds(duration) + delays[index % delays.length]);
  });
  // Even an accidental excessive duration cannot retain the inactive screen
  // indefinitely. Authored state exits fit within this one-second ceiling.
  return Math.min(1_000, Math.max(0, ...timings));
}

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
  const screen = useRef<HTMLDivElement>(null);
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
    const timer = window.setTimeout(finish, screen.current ? exitDuration(screen.current) : 0);
    const reduce = (event: MediaQueryListEvent) => { if (event.matches) finish(); };
    motion.addEventListener("change", reduce);
    return () => { window.clearTimeout(timer); motion.removeEventListener("change", reduce); };
  }, [ready, finish]);

  return <div ref={screen} className="startup-screen" data-state={ready ? "leaving" : "opening"}
    role={ready ? undefined : "status"} aria-label={ready ? undefined : `Opening ${name}`} aria-hidden={ready || undefined}>
    <div className="startup-wordmark"><Wordmark name={name} /></div>
    <p>Opening…</p>
  </div>;
}
