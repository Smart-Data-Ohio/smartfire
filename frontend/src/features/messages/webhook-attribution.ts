import type { MessageWebhook } from "../../gen/MessageWebhook.ts";
import type { MessageDTO } from "../../store/model.ts";

/** The name a webhook message shows: the per-message override, else the webhook's at delivery. */
export function webhookName(webhook: MessageWebhook): string {
  return webhook.username ?? webhook.name;
}

/** Who a message is shown as written by: a room webhook, under the name it posted as, or a person. */
export type DisplayedAuthor =
  | { readonly kind: "webhook"; readonly webhook: MessageWebhook; readonly name: string }
  | { readonly kind: "person"; readonly userId: number };

type Attributed = Pick<MessageDTO, "creatorId" | "webhook" | "forwardedAt">;

function webhookAuthor(webhook: MessageWebhook): DisplayedAuthor {
  return { kind: "webhook", webhook, name: webhookName(webhook) };
}

/**
 * Every "who wrote this" decision goes through here: the header, the avatar, grouping, quotes
 * and previews. The server copies a webhook message's snapshot onto every forward of it, at any
 * depth, but a copy's `creatorId` is the person who forwarded it, so a message with `forwardedAt`
 * set is never the webhook's. Only the message the webhook itself posted is.
 */
export function displayedAuthor(message: Attributed): DisplayedAuthor {
  return message.forwardedAt === null && message.webhook !== null
    ? webhookAuthor(message.webhook)
    : { kind: "person", userId: message.creatorId };
}

/**
 * Who a forwarded copy's block credits. With its source loaded, that's the source's own author,
 * so a forward of a forward credits the person in the middle. Without it, the copy only carries
 * the webhook snapshot: that names the webhook the content came from, and otherwise nobody.
 */
export function forwardedAuthor(
  copy: Pick<MessageDTO, "webhook">,
  source: Attributed | undefined,
): DisplayedAuthor | null {
  if (source !== undefined) {
    return displayedAuthor(source);
  }

  return copy.webhook === null ? null : webhookAuthor(copy.webhook);
}
