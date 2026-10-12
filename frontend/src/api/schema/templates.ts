import { Schema } from "effect";
import type { TemplateImport as GeneratedImport } from "../../gen/TemplateImport.ts";
import type { TemplatePreview as GeneratedPreview } from "../../gen/TemplatePreview.ts";
import type { TemplateUpload as GeneratedUpload } from "../../gen/TemplateUpload.ts";
import type { WorkspaceTemplate as GeneratedTemplate } from "../../gen/WorkspaceTemplate.ts";
import type { Assert, Pinned } from "./pin.ts";

export const TemplateBoard = Schema.Struct({
  tags: Schema.Array(Schema.Struct({ name: Schema.String, emoji: Schema.NullOr(Schema.String) })),
  tagsRequired: Schema.Boolean,
  defaultTag: Schema.NullOr(Schema.Number),
});

export const TemplateRoom = Schema.Struct({
  kind: Schema.Literals(["open", "closed", "voice", "stage", "board"]),
  name: Schema.NullOr(Schema.String),
  iconName: Schema.NullOr(Schema.String),
  topic: Schema.NullOr(Schema.String),
  announcement: Schema.Boolean,
  voiceBitrate: Schema.NullOr(Schema.Number),
  voiceUserLimit: Schema.NullOr(Schema.Number),
  board: Schema.NullOr(TemplateBoard),
});

export const WorkspaceTemplate = Schema.Struct({
  format: Schema.Literal("smartfire-template"),
  version: Schema.Number,
  categories: Schema.Array(
    Schema.Struct({ name: Schema.String, rooms: Schema.Array(TemplateRoom) }),
  ),
  rooms: Schema.Array(TemplateRoom),
});

export const TemplatePreview = Schema.Struct({
  template: Schema.NullOr(WorkspaceTemplate),
  changes: Schema.Array(
    Schema.Struct({
      entity: Schema.Literals(["category", "room"]),
      name: Schema.NullOr(Schema.String),
      createdName: Schema.NullOr(Schema.String),
    }),
  ),
  errors: Schema.Array(Schema.String),
  categoryCount: Schema.Number,
  roomCount: Schema.Number,
});

export const TemplateUpload = Schema.Struct({ contents: Schema.String });

export const TemplateImport = Schema.Struct({
  contents: Schema.String,
  expected: WorkspaceTemplate,
});

export type TemplatePin = Assert<Pinned<typeof WorkspaceTemplate, GeneratedTemplate>>;

export type PreviewPin = Assert<Pinned<typeof TemplatePreview, GeneratedPreview>>;

export type UploadPin = Assert<Pinned<typeof TemplateUpload, GeneratedUpload>>;

export type ImportPin = Assert<Pinned<typeof TemplateImport, GeneratedImport>>;
