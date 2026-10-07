import type { ConversationName } from "../../gen/ConversationName.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";

/** "# general › Launch plan": the room's glyph and name, then the thread's, as a row names it. */
export function ConversationLabel({
  conversation,
}: {
  readonly conversation: ConversationName | null;
}) {
  if (conversation === null) {
    return <span className="conversation-label text-faint">A conversation you can't see</span>;
  }

  const thread = conversation.threadId === null ? null : (conversation.threadName ?? "Thread");

  return (
    <span className="conversation-label">
      <Icon name={ROOM_KIND_ICON[conversation.roomKind]} size={12} className="conversation-glyph" />
      <span className="conversation-room">{conversation.roomName}</span>
      {thread === null ? null : (
        <>
          <Icon name="chevron-right" size={12} className="conversation-glyph" />
          <span className="conversation-thread">{thread}</span>
        </>
      )}
    </span>
  );
}

/** "# general" or "# general › Launch plan" as plain words, for accessible names. */
export function conversationText(conversation: ConversationName | null): string {
  if (conversation === null) {
    return "a conversation";
  }

  const room =
    conversation.roomKind === "direct" ? conversation.roomName : `#${conversation.roomName}`;

  return conversation.threadId === null
    ? room
    : `${room}, thread ${conversation.threadName ?? "Thread"}`;
}
