import { Effect } from "effect";
import type { TemplateImport } from "../gen/TemplateImport.ts";
import type { TemplatePreview } from "../gen/TemplatePreview.ts";
import type { WorkspaceTemplate } from "../gen/WorkspaceTemplate.ts";
import { call } from "./call.ts";
import {
  TemplatePreview as PreviewSchema,
  WorkspaceTemplate as TemplateSchema,
} from "./schema/templates.ts";
import { wire } from "./wire.ts";

export const previewTemplate = Effect.fn("api.previewTemplate")(function* (contents: string) {
  return yield* call(
    { method: "POST", path: "/admin/templates/preview", body: { contents } },
    wire<TemplatePreview>(PreviewSchema),
  );
});

export const importTemplate = Effect.fn("api.importTemplate")(function* (body: TemplateImport) {
  return yield* call(
    { method: "POST", path: "/admin/templates/import", body },
    wire<TemplatePreview>(PreviewSchema),
  );
});

export const exportTemplate = Effect.fn("api.exportTemplate")(function* () {
  return yield* call(
    { method: "GET", path: "/admin/templates/export" },
    wire<WorkspaceTemplate>(TemplateSchema),
  );
});
