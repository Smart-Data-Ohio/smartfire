import { Schema } from "effect";
import type { Attachment as GeneratedAttachment } from "../../gen/Attachment.ts";
import type { AttachmentPreview as GeneratedAttachmentPreview } from "../../gen/AttachmentPreview.ts";
import type { CreateUpload as GeneratedCreateUpload } from "../../gen/CreateUpload.ts";
import type { DirectUpload as GeneratedDirectUpload } from "../../gen/DirectUpload.ts";
import type { Assert, Pinned } from "./pin.ts";

export const AttachmentPreview = Schema.Literals(["image", "video", "file"]);

export type AttachmentPreview = typeof AttachmentPreview.Type;

export type AttachmentPreviewPin = Assert<
  Pinned<typeof AttachmentPreview, GeneratedAttachmentPreview>
>;

/** A message's one file. URLs are same-origin and need the session cookie. */
export const Attachment = Schema.Struct({
  filename: Schema.String,
  contentType: Schema.String,
  byteSize: Schema.Int,
  width: Schema.NullOr(Schema.Int),
  height: Schema.NullOr(Schema.Int),
  preview: AttachmentPreview,
  url: Schema.String,
  downloadUrl: Schema.String,
  thumbnailUrl: Schema.NullOr(Schema.String),
});

export type Attachment = typeof Attachment.Type;

export type AttachmentPin = Assert<Pinned<typeof Attachment, GeneratedAttachment>>;

/** The body of `POST /api/v1/uploads`: the blob to create before `PUT`ting its bytes. */
export const CreateUpload = Schema.Struct({
  filename: Schema.String,
  byteSize: Schema.Int,
  checksum: Schema.String,
  contentType: Schema.String,
});

export type CreateUpload = typeof CreateUpload.Type;

export type CreateUploadPin = Assert<Pinned<typeof CreateUpload, GeneratedCreateUpload>>;

/** `PUT` the bytes to `uploadUrl` (5 minutes), then post with `attachmentSignedId`. */
export const DirectUpload = Schema.Struct({
  signedId: Schema.String,
  uploadUrl: Schema.String,
});

export type DirectUpload = typeof DirectUpload.Type;

export type DirectUploadPin = Assert<Pinned<typeof DirectUpload, GeneratedDirectUpload>>;
