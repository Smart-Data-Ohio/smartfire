/**
 * Reading a `CreateMessage` body, shared by the root timeline, thread replies and new threads.
 */
import type { Attachment } from "../../src/gen/Attachment.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import { validation } from "../http.ts";
import { field, intField, isRecord, type Json, stringArrayField, stringField } from "../json.ts";
import { SOURCE_LIMIT } from "./model.ts";

/** A `CreateMessage` body that passed validation. */
export interface ParsedMessage {
  readonly clientMessageId: string;
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
function groupedFiles(
  body: Json | undefined,
  single: string | null,
  attach: (signedId: string) => Attachment,
): readonly Attachment[] | null {
  const ids = stringArrayField(body, "attachmentSignedIds") ?? [];

  if (ids.length === 0) return null;

  if (ids.length > MAX_MESSAGE_FILES) {
    throw validation("attachmentSignedIds", "has too many files (maximum is 10)");
  }

  if (single !== null && single !== "") {
    throw validation("attachmentSignedIds", "cannot be combined with attachmentSignedId");
  }

  if (new Set(ids).size !== ids.length) {
    throw validation("attachmentSignedIds", "includes a duplicate file");
  }

  return ids.map(attach);
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
): ParsedMessage {
  const clientMessageId = clientMessageIdOf(body);
  const signedId = stringField(body, "attachmentSignedId");
  const attachments = groupedFiles(body, signedId, attach);
  const single = signedId === null || signedId === "" ? null : attach(signedId);
  const attachment = attachments?.[0] ?? single;
  const driveFileIds = stringArrayField(body, "driveFileIds") ?? [];

  const markdown = checkMarkdown(
    stringField(body, "markdownSource"),
    attachment !== null || driveFileIds.length > 0,
  );

  return {
    clientMessageId,
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

/** The draft fields for a parsed message's files (`attachments` only when it was grouped). */
export function draftFiles(parsed: ParsedMessage): {
  readonly attachment: Attachment | null;
  readonly attachments?: readonly Attachment[];
} {
  return parsed.attachments === null
    ? { attachment: parsed.attachment }
    : { attachment: parsed.attachment, attachments: parsed.attachments };
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
