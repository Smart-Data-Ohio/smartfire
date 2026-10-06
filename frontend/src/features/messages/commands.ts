/**
 * What the hover bar, the context menu and the row's keys do. Each runs one action and says how
 * it went with a toast when the row itself doesn't show it (copies, unread, failures).
 */
import type { EmojiChoice } from "../../lib/emoji/recent.ts";
import { recordRecentEmoji } from "../../lib/emoji/recent.ts";
import type { MessageDTO } from "../../store/model.ts";
import { store } from "../../store/store.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { queueBurst } from "./burst.ts";
import { startEditing } from "./editing-store.ts";
import { holdUnread } from "./unread-hold.ts";

const failed = (title: string) => (error: ActionError) =>
  toast({ title, description: error.message, tone: "danger" });

/** The message's permalink, absolute, for the clipboard. */
export function permalink(message: MessageDTO): string {
  const path =
    message.threadId === null
      ? `r/${message.roomId}/m/${message.id}`
      : `r/${message.roomId}/t/${message.threadId}?m=${message.id}`;

  return new URL(`${import.meta.env.BASE_URL}${path}`, window.location.origin).href;
}

/** The message as plain text: its Markdown when known, else the rendered body's text. */
export function plainText(message: MessageDTO): string {
  if (message.markdownSource !== null) {
    return message.markdownSource;
  }

  return htmlText(message.bodyHtml);
}

/** The text content of server-rendered HTML (parsed inert: nothing runs or loads). */
export function htmlText(html: string): string {
  const document = new DOMParser().parseFromString(html, "text/html");

  return (document.body.textContent ?? "").replace(/\s+\n/g, "\n").trim();
}

function copy(text: string, done: string): void {
  void navigator.clipboard.writeText(text).then(
    () => toast({ title: done, tone: "success" }),
    () => toast({ title: "Couldn't copy to the clipboard", tone: "danger" }),
  );
}

export function copyLink(message: MessageDTO): void {
  copy(permalink(message), "Link copied");
}

export function copyText(message: MessageDTO): void {
  copy(plainText(message), "Text copied");
}

/** Adds or removes the viewer's reaction (optimistic in the store), remembering the emoji. */
export function toggleReaction(message: MessageDTO, choice: EmojiChoice): void {
  const viewerId = store.getState().me?.user.id ?? null;
  const existing = message.reactions.find((reaction) => reaction.content === choice.content);

  recordRecentEmoji(choice);

  if (viewerId !== null && !(existing?.reactorIds.includes(viewerId) ?? false)) {
    queueBurst(message.id, choice.content);
  }

  void actions.messages
    .toggleReaction(message.id, choice.content, { title: choice.title, imageUrl: choice.imageUrl })
    .catch(failed("Couldn't update the reaction"));
}

export function addBoost(message: MessageDTO, text: string): Promise<void> {
  return actions.messages.boost(message.id, text);
}

export function removeBoost(message: MessageDTO, boostId: number): void {
  void actions.messages.removeBoost(message.id, boostId).catch(failed("Couldn't remove the boost"));
}

export function togglePin(message: MessageDTO): void {
  const pinning = !message.pinned;

  void actions.messages
    .setPinned(message.id, pinning)
    .then(
      () => toast({ title: pinning ? "Pinned to the conversation" : "Unpinned", tone: "success" }),
      failed(pinning ? "Couldn't pin the message" : "Couldn't unpin the message"),
    );
}

export function toggleSave(message: MessageDTO): void {
  const saved = store.getState().saved[message.id] !== undefined;

  if (saved) {
    void actions.messages.unsave(message.id).catch(failed("Couldn't remove it from Saved"));

    return;
  }

  void actions.messages
    .save(message.id)
    .then(() => toast({ title: "Saved for later", tone: "success" }), failed("Couldn't save it"));
}

export function markUnread(message: MessageDTO): void {
  holdUnread(message.roomId);

  void actions.messages
    .markUnreadFrom(message.roomId, message.id)
    .then(
      () => toast({ title: "Marked unread from here", tone: "success" }),
      failed("Couldn't mark it unread"),
    );
}

export function edit(message: MessageDTO): void {
  startEditing(message.id);
}

/** Deletes it; the row has already played its collapse. */
export function remove(message: MessageDTO): Promise<void> {
  return actions.messages.remove(message.id).catch((error: ActionError) => {
    failed("Couldn't delete the message")(error);

    throw error;
  });
}

let iconsLoading: Promise<readonly EmojiChoice[]> | null = null;

/** The workspace's icons as picker choices, fetched once per session. */
export function loadCustomIcons(): Promise<readonly EmojiChoice[]> {
  iconsLoading ??= actions.messages.customIcons().then(
    // The list holds brand and workspace icons, each with an image; anything else is skipped.
    (list) =>
      list.flatMap((icon) =>
        icon.imageUrl === null
          ? []
          : [{ content: `:${icon.name}:`, title: icon.title, imageUrl: icon.imageUrl }],
      ),
    (error: ActionError) => {
      iconsLoading = null;

      throw error;
    },
  );

  return iconsLoading;
}
