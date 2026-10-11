import type { MessageWebhook } from "../../gen/MessageWebhook.ts";
import type { MessageDTO } from "../../store/model.ts";

/** The name a webhook message shows: the per-message override, else the webhook's at delivery. */
export function webhookName(webhook: MessageWebhook): string {
  return webhook.username ?? webhook.name;
}

/**
 * The webhook that posted this message itself, or null. A forwarded copy keeps the source's
 * webhook snapshot for its forwarded block, but `creatorId` is the person who forwarded it, so
 * its own author line (and grouping) is theirs. `forwardedAt` is what marks a forward.
 */
export function postingWebhook(
  message: Pick<MessageDTO, "webhook" | "forwardedAt">,
): MessageWebhook | null {
  return message.forwardedAt === null ? message.webhook : null;
}

/** The webhook a forwarded copy's source was posted by, for its forwarded block; null otherwise. */
export function forwardedWebhook(
  message: Pick<MessageDTO, "webhook" | "forwardedAt">,
): MessageWebhook | null {
  return message.forwardedAt === null ? null : message.webhook;
}
