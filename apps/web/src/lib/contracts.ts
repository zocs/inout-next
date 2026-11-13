export interface Product {
  name: string;
  slug: string;
  description: string;
  version: string;
  application_id: string;
  repository: string;
}

export interface Entry {
  name: string;
  path: string;
  kind: "file" | "directory";
  size: number | null;
  modified: number | null;
}

export interface ServerStatus { product: Product; auth_required: boolean }

export interface ServerContext {
  product: Product;
  root_label: string;
  modules: string[];
  permissions: { list: boolean; download: boolean; upload: boolean; manage: boolean };
  capabilities: { file_move: boolean; directory_move: boolean; directory_copy: boolean };
  limits: { max_upload_bytes: number };
}

export type Operation = "rename" | "move" | "copy";

export interface EntryList { path: string; entries: Entry[] }
