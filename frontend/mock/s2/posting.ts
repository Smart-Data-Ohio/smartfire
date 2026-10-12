/**
 * Reading a `CreateMessage` body, shared by the root timeline, thread replies and new threads.
 */
import type { Attachment } from "../../src/gen/Attachment.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessageSticker } from "../../src/gen/MessageSticker.ts";
import { validation } from "../http.ts";
import {
  field,
  intField,
  isBoolean,
  isRecord,
  type Json,
  stringArrayField,
  stringField,
} from "../json.ts";
import { SOURCE_LIMIT } from "./model.ts";

/** A `CreateMessage` body that passed validation. */
export interface ParsedMessage {
  readonly clientMessageId: string;
  readonly sticker: MessageSticker | undefined;
  readonly markdown: string;
  readonly replyToMessageId: number | null;
  /** The legacy single file, or a grouped message's first file. */
  readonly attachment: Attachment | null;
  /** A grouped message's files (`attachmentSignedIds`), in order; `null` on the legacy path. */
  readonly attachments: readonly Attachment[] | null;
  readonly driveFileIds: readonly string[];
}

/** How many files one message can carry (`ATTACHMENTS_PER_MESSAGE`). */
export const MAX_MESSAGE_FILES = 10;

/**
 * The grouped files of a `CreateMessage` body, checked as `grouped_signed_ids` and
 * `require_grouped_uploads` do: at most ten, not with the legacy slot, no repeats, each finished.
 */
export function groupedFiles(
  body: Json | undefined,
  single: string | null,
  attach: (signedId: string) => Attachment,
): readonly Attachment[] | null {
  const ids = stringArrayField(body, "attachmentSignedIds") ?? [];
  const gif = (stringField(body, "gifSlug") ?? "").trim();

  if (ids.length === 0) return null;

  if (ids.length + Number(gif !== "") > MAX_MESSAGE_FILES) {
    throw validation("attachmentSignedIds", "has too many files (maximum is 10)");
  }

  if (single !== null && single !== "") {
    throw validation("attachmentSignedIds", "cannot be combined with attachmentSignedId");
  }

  if (new Set(ids).size !== ids.length) {
    throw validation("attachmentSignedIds", "includes a duplicate file");
  }

  const files = ids.map((signedId) => attach(signedId));

  return gif === "" ? withSpoilers(body, files) : files;
}

/** The `clientMessageId`, checked first so a retry is answered before anything else. */
export function clientMessageIdOf(body: Json | undefined): string {
  const clientMessageId = stringField(body, "clientMessageId");

  if (clientMessageId === null || clientMessageId === "") {
    throw validation("clientMessageId", "Client message can't be blank");
  }

  return clientMessageId;
}

/** Checks Markdown the way the message model does: present unless there's a file, not too long. */
export function checkMarkdown(markdown: string | null, hasAttachment: boolean): string {
  const text = markdown ?? "";

  if (text.trim() === "" && !hasAttachment) throw validation("body", "Body can't be blank");

  if (text.length > SOURCE_LIMIT) {
    throw validation("body", `Body is too long (maximum is ${SOURCE_LIMIT} characters)`);
  }

  return text;
}

/**
 * Validates a `CreateMessage` body. `attach` resolves `attachmentSignedId` and each of
 * `attachmentSignedIds` (422 for one that isn't a finished upload).
 */
export function parseMessage(
  body: Json | undefined,
  attach: (signedId: string) => Attachment,
  resolveSticker: (id: number) => MessageSticker = () => {
    throw validation("stickerId", "must exist");
  },
  resolveGif: (slug: string) => Attachment = () => {
    throw validation("gifSlug", "must exist");
  },
): ParsedMessage {
  const clientMessageId = clientMessageIdOf(body);
  const signedId = stringField(body, "attachmentSignedId");
  const gifSlug = (stringField(body, "gifSlug") ?? "").trim();

  if (gifSlug !== "" && signedId !== null && signedId !== "")
    throw validation("gifSlug", "cannot be combined with attachmentSignedId");

  const grouped = groupedFiles(body, signedId, attach);
  const attachments =
    grouped !== null || gifSlug !== ""
      ? withSpoilers(body, [...(grouped ?? []), ...(gifSlug === "" ? [] : [resolveGif(gifSlug)])])
      : null;

  const single =
    signedId === null || signedId === ""
      ? null
      : (withSpoilers(body, [attach(signedId)])[0] ?? null);

  if (attachments === null && single === null) withSpoilers(body, []);
  const attachment = attachments?.[0] ?? single;
  const driveFileIds = stringArrayField(body, "driveFileIds") ?? [];

  const stickerId = intField(body, "stickerId");
  const sticker = stickerId === null ? undefined : resolveSticker(stickerId);

  const markdown = checkMarkdown(
    stringField(body, "markdownSource"),
    attachment !== null || driveFileIds.length > 0 || sticker !== undefined,
  );

  return {
    clientMessageId,
    sticker,
    markdown,
    replyToMessageId: intField(body, "replyToMessageId"),
    attachment,
    attachments,
    driveFileIds,
  };
}

/** A message's files: its grouped list, else its one legacy file. */
export function filesOf(message: MessageDTO): readonly Attachment[] {
  return message.attachments ?? (message.attachment === null ? [] : [message.attachment]);
}

/** Files and sticker from a parsed message (`attachments` only when it was grouped). */
export function draftFiles(parsed: ParsedMessage): {
  readonly attachment: Attachment | null;
  readonly attachments?: readonly Attachment[];
  readonly sticker?: MessageSticker;
} {
  const sticker = parsed.sticker === undefined ? {} : { sticker: parsed.sticker };

  return parsed.attachments === null
    ? { attachment: parsed.attachment, ...sticker }
    : { attachment: parsed.attachment, attachments: parsed.attachments, ...sticker };
}

/** Drive chips for ids the composer pinned. The name isn't stored, matching the message card. */
export function driveCards(ids: readonly string[]): MessageDTO["cards"] {
  return ids.map((fileId) => ({
    kind: "drive",
    data: { fileId, url: `https://drive.google.com/open?id=${fileId}` },
  }));
}

/** The nested `message` of a `CreateThread` body. */
export function nestedMessage(body: Json | undefined): Json | undefined {
  const message = field(body, "message");

  return isRecord(message) ? message : undefined;
}

/** Flags are owned by the attachment, never written back to the uploaded blob. */
export function withSpoilers(
  body: Json | undefined,
  files: readonly Attachment[],
  retained: readonly Attachment[] = [],
): readonly Attachment[] {
  const raw = field(body, "attachmentSpoilers");

  if (
    raw !== undefined &&
    (!Array.isArray(raw) || raw.length !== files.length || !raw.every(isBoolean))
  ) {
    throw validation("attachmentSpoilers", "must have one flag per file");
  }

  const flags = Array.isArray(raw) ? raw.filter(isBoolean) : null;

  return files.map((file, index) => ({
    ...file,
    spoiler: flags?.[index] ?? retained.find((kept) => kept.url === file.url)?.spoiler ?? false,
  }));
}
