/**
 * The `source` of an inbox item, built the way `presenters::activity` fills it for each source
 * table: ids to route by, who's behind it, the row's heading and body, and the classic page it
 * leads to. Shared by the seed, the reminder and drop hooks, and the ambient loop.
 */
import type { ActivitySource } from "../../src/gen/ActivitySource.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { ScheduledMessage } from "../../src/gen/ScheduledMessage.ts";
import { VIEWER_ID } from "../seed.ts";
import { conversationTitle, type Namer } from "./conversations.ts";
import { plainText, truncateBody } from "./model.ts";

/** What the reminder body starts with (`presenters::activity`). */
export const REMINDER_PREFIX = "You asked to be reminded about this message: ";

/** The classic permalink of a message: `room_at_message`, or the thread pane at it. */
export function messagePath(message: MessageDTO): string {
  return message.threadId === null
    ? `/rooms/${message.roomId}/@${message.id}`
    : `/rooms/${message.roomId}?message_id=${message.id}&thread=${message.threadId}`;
}

/** A message source (mentions, replies, keyword alerts, threads, PR review requests). */
export function messageSource(names: Namer, message: MessageDTO): ActivitySource {
  return {
    sourceType: "message",
    sourceId: message.id,
    roomId: message.roomId,
    threadId: message.threadId,
    messageId: message.id,
    eventId: null,
    creatorId: message.creatorId,
    title: conversationTitle(names, message.roomId, message.threadId),
    body: truncateBody(plainText(message.markdownSource)),
    occurredAt: message.createdAt,
    approvalStatus: null,
    budgetCap: null,
    path: messagePath(message),
  };
}

/** A saved item's reminder: the message, introduced as a reminder. */
export function reminderSource(
  names: Namer,
  savedItemId: number,
  message: MessageDTO,
): ActivitySource {
  return {
    ...messageSource(names, message),
    sourceType: "saved_item",
    sourceId: savedItemId,
    body: truncateBody(REMINDER_PREFIX + plainText(message.markdownSource)),
  };
}

/** A dropped scheduled message: why, and the text that wasn't sent. */
export function droppedSource(names: Namer, message: ScheduledMessage): ActivitySource {
  const reason = message.dropReason?.trim() ?? "";

  const body =
    reason === ""
      ? `You no longer have access to this room, so your scheduled message was not sent: ${message.markdownSource}`
      : `Your scheduled message was not sent (${reason}): ${message.markdownSource}`;

  return {
    sourceType: "scheduled_message",
    sourceId: message.id,
    roomId: message.roomId,
    threadId: message.threadId,
    messageId: null,
    eventId: null,
    creatorId: VIEWER_ID,
    title: conversationTitle(names, message.roomId, null),
    body: truncateBody(body),
    occurredAt: message.createdAt,
    approvalStatus: null,
    budgetCap: null,
    path: "/scheduled_messages",
  };
}

/** A huddle someone started (or the viewer missed). */
export function huddleSource(
  names: Namer,
  grantId: number,
  roomId: number,
  callerId: number,
  missed: boolean,
  at: string,
): ActivitySource {
  const caller = names.world().users.get(callerId)?.name ?? "Someone";

  return {
    sourceType: "huddle_grant",
    sourceId: grantId,
    roomId,
    threadId: null,
    messageId: null,
    eventId: null,
    creatorId: callerId,
    title: conversationTitle(names, roomId, null),
    body: missed ? `You missed a huddle from ${caller}` : `${caller} started a huddle`,
    occurredAt: at,
    approvalStatus: null,
    budgetCap: null,
    path: `/rooms/${roomId}`,
  };
}
