/** Sections and words for the scheduled-messages page (the classic page's three groups). */
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import { sendAtLabel } from "../composer/schedule/presets.ts";

/** Upcoming (pending, sendable), stranded (pending, can't be sent) or past (sent or dropped). */
export type ScheduledSection = "upcoming" | "stranded" | "past";

export function scheduledSection(item: ScheduledMessage): ScheduledSection {
  if (item.state === "sent" || item.state === "dropped") {
    return "past";
  }

  return item.sendable ? "upcoming" : "stranded";
}

/** The Markdown as quiet text: emphasis and heading markers dropped, whitespace folded. */
export function markdownExcerpt(markdown: string): string {
  return markdown
    .replace(/[*_~`>#]+/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

/** "today at 3:00 PM", "Mon, Oct 12 at 9:00 AM": a time inside a sentence. */
export function inlineWhen(at: string, now: number): string {
  return sendAtLabel(new Date(at), new Date(now)).replace(/^(Today|Tomorrow)/, (word) =>
    word.toLowerCase(),
  );
}

/** A past row's outcome: "Sent today at 3:00 PM", or why it wasn't. */
export function outcomeLabel(item: ScheduledMessage, now: number): string {
  if (item.state === "sent") {
    return item.sentAt === null ? "Sent" : `Sent ${inlineWhen(item.sentAt, now)}`;
  }

  return item.dropReason === null
    ? "Not sent: you lost access to the conversation"
    : `Not sent: ${item.dropReason}`;
}
