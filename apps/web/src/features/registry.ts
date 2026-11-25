import { filesModule } from "./files";
import { uploadsModule } from "./uploads";
import { manageModule } from "./manage";
import { previewModule } from "./preview";
import type { FeatureModule } from "@/ui/modules";

export const featureRegistry: readonly FeatureModule[] = [filesModule, previewModule, uploadsModule, manageModule];
