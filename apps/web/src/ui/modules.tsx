import type { ComponentType, ReactNode } from "react";
import type { PresentationMessage } from "@/i18n/catalog";
import type { FileApi } from "@/lib/api";
import type { Entry, ServerContext } from "@/lib/contracts";

export interface ModuleRuntime {
  context: ServerContext;
  api: FileApi;
  path: string;
  entries: Entry[];
  /** Advances only after an accepted listing; unchanged metadata cannot validate cached content. */
  contentRevision: number;
  loading: boolean;
  selected: Entry | null;
  /** `reveal` asks the shell to bring the inspector into view; keyboard selection keeps its position. */
  select: (entry: Entry | null, origin?: HTMLElement, reveal?: boolean) => void;
  navigate: (path: string) => void;
  refresh: () => void;
  notify: (message: PresentationMessage, kind?: "success" | "error") => void;
  focusCurrentFolder: () => void;
}

export interface ModuleProps { runtime: ModuleRuntime }
export interface ModuleProviderProps extends ModuleProps { children: ReactNode }

export interface FeatureModule {
  id: string;
  /** Mounted server module supplying this contribution's routes; defaults to `id`. */
  module?: string;
  requires: readonly string[];
  enabled: (context: ServerContext) => boolean;
  Provider?: ComponentType<ModuleProviderProps>;
  View?: ComponentType<ModuleProps>;
  /** Rendered inside the selected-entry inspector, before its actions. */
  Inspector?: ComponentType<ModuleProps>;
  Toolbar?: ComponentType<ModuleProps>;
  SelectionPanel?: ComponentType<ModuleProps>;
  Panel?: ComponentType<ModuleProps>;
}

export function activeModules(registry: readonly FeatureModule[], context: ServerContext): FeatureModule[] {
  return registry.filter((feature) => context.modules.includes(feature.module ?? feature.id)
    && feature.requires.every((id) => context.modules.includes(id)) && feature.enabled(context));
}

export function ModuleSlot({ modules, slot, runtime }: { modules: FeatureModule[]; slot: "View" | "Toolbar" | "Inspector" | "SelectionPanel" | "Panel"; runtime: ModuleRuntime }) {
  return <>{modules.map((feature) => {
    const Component = feature[slot];
    return Component ? <Component key={feature.id} runtime={runtime} /> : null;
  })}</>;
}

export function ModuleProviders({ modules, runtime, children }: { modules: FeatureModule[]; runtime: ModuleRuntime; children: ReactNode }) {
  return modules.reduceRight<ReactNode>((content, feature) => {
    const Provider = feature.Provider;
    return Provider ? <Provider key={feature.id} runtime={runtime}>{content}</Provider> : content;
  }, children);
}
