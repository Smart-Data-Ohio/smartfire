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
  readonly attachment: Attachment | null;
  readonly driveFileIds: readonly string[];
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
 * Validates a `CreateMessage` body. `attach` resolves `attachmentSignedId` (422 for one that
 * isn't a finished upload).
 */
export function parseMessage(
  body: Json | undefined,
  attach: (signedId: string) => Attachment,
): ParsedMessage {
  const clientMessageId = clientMessageIdOf(body);
  const signedId = stringField(body, "attachmentSignedId");
  const attachment = signedId === null || signedId === "" ? null : attach(signedId);
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
    driveFileIds,
  };
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
