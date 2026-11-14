/** Main-window IPC only; these records contain no credential or host path. */
export interface DiagnosticReport {
  bytes: number;
  durationMs: number;
  mibPerSecond: number;
  bufferKib: number;
  connections: number;
}
export interface DiagnosticStatus {
  enabled: boolean;
  running: boolean;
  bufferKib: number;
  defaultBufferKib: number;
  probeState: "idle" | "running";
  result: DiagnosticReport | null;
}
