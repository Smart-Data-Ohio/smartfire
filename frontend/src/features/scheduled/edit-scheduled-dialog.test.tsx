import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import { EditScheduledDialog } from "./edit-scheduled-dialog.tsx";

const ITEM: ScheduledMessage = {
  id: 1,
  roomId: 12,
  threadId: null,
  replyToMessageId: null,
  markdownSource: "Hello",
  sendAt: new Date(2026, 9, 7, 9, 0).toISOString(),
  state: "pending",
  sendable: true,
  sentAt: null,
  sentMessageId: null,
  droppedAt: null,
  dropReason: null,
  createdAt: new Date(2026, 9, 6, 8, 0).toISOString(),
};

afterEach(() => {
  vi.useRealTimers();
});

describe("the scheduled message editor", () => {
  it("keeps its earliest time current while it stays open", () => {
    vi.useFakeTimers({
      toFake: ["setInterval", "clearInterval", "Date"],
      now: new Date(2026, 9, 6, 12, 0, 10),
    });
    render(<EditScheduledDialog item={ITEM} onClose={() => undefined} onSave={async () => {}} />);

    const field = screen.getByLabelText("Send at");

    expect(field.getAttribute("min")).toBe("2026-10-06T12:00");

    act(() => vi.advanceTimersByTime(3 * 60_000));

    expect(field.getAttribute("min")).toBe("2026-10-06T12:03");
  });
});
