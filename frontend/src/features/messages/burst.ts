/**
 * Reactions the viewer just added here, so their pill plays the like-button burst once (a pill
 * the click created mounts after the click, so it can't watch the click itself). Other people's
 * reactions arriving live never burst.
 */
const pending = new Set<string>();

const keyOf = (messageId: number, content: string) => `${messageId}:${content}`;

export function queueBurst(messageId: number, content: string): void {
  pending.add(keyOf(messageId, content));
}

/** True once per queued burst. */
export function takeBurst(messageId: number, content: string): boolean {
  return pending.delete(keyOf(messageId, content));
}
