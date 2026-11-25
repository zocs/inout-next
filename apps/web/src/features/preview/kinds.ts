// Mirrors the server's inline allow-list. The server decides what is actually
// presented inline; this only picks the element used to show it.
export type PreviewKind = "image" | "audio" | "video" | "text";

const kinds: Readonly<Record<PreviewKind, readonly string[]>> = {
  image: ["png", "jpg", "jpeg", "gif", "webp", "avif", "bmp", "ico"],
  audio: ["mp3", "m4a", "aac", "ogg", "opus", "wav", "flac"],
  video: ["mp4", "m4v", "webm", "ogv", "mov"],
  text: ["txt", "text", "md", "markdown", "log", "json", "jsonl", "csv", "tsv", "xml", "yaml", "yml", "toml", "ini", "conf", "cfg",
    "properties", "env", "srt", "vtt", "diff", "patch", "sql", "sh", "bash", "zsh", "fish", "bat", "cmd", "ps1", "py", "rs", "go",
    "c", "h", "cc", "cpp", "hpp", "cs", "java", "kt", "kts", "swift", "dart", "js", "mjs", "cjs", "jsx", "ts", "tsx", "css", "scss",
    "less", "vue", "svelte", "lua", "rb", "php", "pl", "r", "tex", "gradle", "slint"],
};

const byExtension = new Map<string, PreviewKind>(
  (Object.entries(kinds) as [PreviewKind, readonly string[]][]).flatMap(([kind, extensions]) => extensions.map((extension) => [extension, kind] as const)));

export function previewKind(name: string): PreviewKind | null {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? byExtension.get(name.slice(dot + 1).toLowerCase()) ?? null : null;
}
