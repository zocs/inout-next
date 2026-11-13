import type { Entry, EntryList, Operation, ServerContext, ServerStatus } from "./contracts";

export class ApiError extends Error {
  constructor(public code: string, message: string, public status = 0) {
    super(message);
    this.name = "ApiError";
  }
}

function errorFromBody(body: unknown, status: number): ApiError {
  const envelope = body as { error?: { code?: unknown; message?: unknown } } | null;
  if (typeof envelope?.error?.code === "string" && typeof envelope.error.message === "string") {
    return new ApiError(envelope.error.code, envelope.error.message, status);
  }
  return new ApiError("request_failed", "The request failed. Refresh the folder and try again.", status);
}

export class FileApi {
  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    let response: Response;
    try {
      response = await fetch("/api/v1" + path, { ...options, credentials: "same-origin" });
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") throw error;
      throw new ApiError("connection_lost", "Cannot reach the sharing device. Check the connection and try again.");
    }
    const body: unknown = response.status === 204 ? undefined : await response.json().catch(() => null);
    if (!response.ok) throw errorFromBody(body, response.status);
    return body as T;
  }

  status(): Promise<ServerStatus> { return this.request("/status"); }
  context(): Promise<ServerContext> { return this.request("/context"); }
  login(key: string): Promise<void> { return this.json("/session", "POST", { key }); }
  logout(): Promise<void> { return this.request("/session", { method: "DELETE" }); }
  entries(path: string, signal?: AbortSignal): Promise<EntryList> {
    return this.request("/entries?path=" + encodeURIComponent(path), { signal });
  }
  mkdir(path: string): Promise<{ entry: Entry }> { return this.json("/directories", "POST", { path }); }
  operate(action: Operation, source: string, destination: string): Promise<{ entry: Entry }> {
    return this.json("/operations", "POST", { action, source, destination });
  }
  remove(path: string, recursive: boolean): Promise<void> {
    return this.request("/entries?path=" + encodeURIComponent(path) + "&recursive=" + recursive, { method: "DELETE" });
  }
  downloadUrl(path: string): string { return "/api/v1/content?path=" + encodeURIComponent(path); }

  private json<T>(path: string, method: string, body: unknown): Promise<T> {
    return this.request(path, { method, headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
  }
}

export interface UploadProgress { sent: number; total: number; finalizing: boolean }

export function sendFile(file: File, path: string, onProgress: (progress: UploadProgress) => void) {
  const xhr = new XMLHttpRequest();
  const promise = new Promise<Entry>((resolve, reject) => {
    xhr.open("PUT", "/api/v1/content?path=" + encodeURIComponent(path));
    xhr.setRequestHeader("content-type", "application/octet-stream");
    xhr.withCredentials = true;
    xhr.upload.onprogress = (event) => onProgress({ sent: event.loaded, total: file.size, finalizing: false });
    xhr.upload.onload = () => onProgress({ sent: file.size, total: file.size, finalizing: true });
    xhr.onload = () => {
      let body: unknown;
      try { body = JSON.parse(xhr.responseText); } catch { body = null; }
      if (xhr.status !== 201) { reject(errorFromBody(body, xhr.status)); return; }
      const entry = (body as { entry?: Entry } | null)?.entry;
      if (!entry) { reject(new ApiError("invalid_response", "The server did not confirm that the file was saved.")); return; }
      resolve(entry);
    };
    xhr.onerror = () => reject(new ApiError("connection_lost", "Upload interrupted. Refresh the folder to check whether it was saved, then try again."));
    xhr.onabort = () => reject(new ApiError("canceled", "Canceled"));
    xhr.send(file);
  });
  return { promise, cancel: () => xhr.abort() };
}
