// Capture only an explicit task's focus, before disabling or removing its control.
export function captureTaskFocus(owner: HTMLElement | null) {
  let owned = !!owner?.contains(document.activeElement);
  const onFocus = (event: FocusEvent) => {
    if (event.target instanceof Node && !owner?.contains(event.target)) owned = false;
  };
  if (owned) document.addEventListener("focusin", onFocus);
  return (destination: () => void) => {
    const connected = !!owner?.isConnected;
    requestAnimationFrame(() => {
      document.removeEventListener("focusin", onFocus);
      // Disabling a focused button may leave BODY focused without a focusin event.
      // A chosen outside destination or a passively removed task cancels recovery.
      if (owned && connected && (document.activeElement === document.body || owner?.contains(document.activeElement))) destination();
    });
  };
}
