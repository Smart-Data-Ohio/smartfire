import { describe, expect, it } from "vitest";
import { meFixture, messageFixture } from "../../api/testing.ts";
import type { MessageDTO } from "../../gen/MessageDTO.ts";
import { emptyTimeline, initialState, type State } from "../../store/state.ts";
import { lastEditableMessage } from "./edit-last.ts";

function stateWith(
  messages: readonly MessageDTO[],
  change: { readonly after?: number | null; readonly threadId?: number } = {},
): State {
  const ids = messages.map((message) => message.id);
  const timeline = { ...emptyTimeline, ids, after: change.after ?? null, status: "ready" as const };

  return {
    ...initialState,
    me: meFixture,
    messages: Object.fromEntries(messages.map((message) => [message.id, message])),
    timelines: change.threadId === undefined ? { 12: timeline } : {},
    threadTimelines: change.threadId === undefined ? {} : { [change.threadId]: timeline },
  };
}

describe("lastEditableMessage", () => {
  it("picks the viewer's newest message, skipping others and system notes", () => {
    const state = stateWith([
      messageFixture(1, 12),
      messageFixture(2, 12, { creatorId: 8 }),
      messageFixture(3, 12, { systemNote: true }),
      messageFixture(4, 12, { creatorId: 8 }),
    ]);

    expect(lastEditableMessage(state, 12, null)?.id ?? null).toBe(1);
  });

  it("returns null when the viewer has nothing here", () => {
    expect(
      lastEditableMessage(stateWith([messageFixture(1, 12, { creatorId: 8 })]), 12, null)?.id ??
        null,
    ).toBe(null);
    expect(lastEditableMessage(stateWith([]), 99, null)?.id ?? null).toBeNull();
  });

  it("returns null when the loaded window doesn't reach the present", () => {
    expect(
      lastEditableMessage(stateWith([messageFixture(1, 12)], { after: 1 }), 12, null)?.id ?? null,
    ).toBeNull();
  });

  it("reads a thread's own timeline", () => {
    const state = stateWith(
      [
        messageFixture(5, 12, { threadId: 40 }),
        messageFixture(6, 12, { threadId: 40, creatorId: 8 }),
      ],
      { threadId: 40 },
    );

    expect(lastEditableMessage(state, 12, 40)?.id ?? null).toBe(5);
    expect(lastEditableMessage(state, 12, null)?.id ?? null).toBeNull();
  });
});
