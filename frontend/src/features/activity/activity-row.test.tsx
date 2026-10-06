import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ActivityItem } from "../../gen/ActivityItem.ts";
import { ActivityRow } from "./activity-row.tsx";

const NOW = Date.parse("2026-10-06T12:00:00.000Z");

function item(state: ActivityItem["state"]): ActivityItem {
  return {
    id: 7,
    eventType: "mention",
    state,
    readAt: null,
    handledAt: null,
    createdAt: "2026-10-06T11:00:00.000Z",
    updatedAt: "2026-10-06T11:00:00.000Z",
    source: {
      sourceType: "message",
      sourceId: 9001,
      roomId: 12,
      threadId: null,
      messageId: 9001,
      eventId: null,
      creatorId: null,
      title: "general",
      body: "Can you review the launch plan?",
      occurredAt: "2026-10-06T11:00:00.000Z",
      approvalStatus: null,
      budgetCap: null,
      path: "/rooms/12/@9001",
    },
  };
}

function renderRow(state: ActivityItem["state"]) {
  const handlers = { onOpen: vi.fn(), onAction: vi.fn(), onMenu: vi.fn() };
  const subject = item(state);

  render(
    <ActivityRow
      item={subject}
      now={NOW}
      motion={undefined}
      celebrate={false}
      onOpen={handlers.onOpen}
      onAction={handlers.onAction}
      onMenu={handlers.onMenu}
    />,
  );

  return { handlers, subject, open: screen.getByRole("button", { name: /general/ }) };
}

describe("an activity row", () => {
  it("shows the kind, title and excerpt, and opens on click", () => {
    const { handlers, subject, open } = renderRow("unread");

    expect(screen.getByText("Mention")).toBeTruthy();
    expect(screen.getByText("Can you review the launch plan?")).toBeTruthy();

    fireEvent.click(open);

    expect(handlers.onOpen).toHaveBeenCalledWith(subject);
  });

  it("toggles read with U and handled with E", () => {
    const { handlers, subject, open } = renderRow("unread");

    fireEvent.keyDown(open, { key: "u" });
    fireEvent.keyDown(open, { key: "e" });

    expect(handlers.onAction).toHaveBeenNthCalledWith(1, subject, "read");
    expect(handlers.onAction).toHaveBeenNthCalledWith(2, subject, "handled");
  });

  it("offers the reverse actions once read and handled", () => {
    const { handlers, subject, open } = renderRow("handled");

    expect(screen.getByText("Handled")).toBeTruthy();

    fireEvent.keyDown(open, { key: "u" });
    fireEvent.keyDown(open, { key: "e" });

    expect(handlers.onAction).toHaveBeenNthCalledWith(1, subject, "unread");
    expect(handlers.onAction).toHaveBeenNthCalledWith(2, subject, "unhandled");
  });

  it("opens the menu on Shift+F10", () => {
    const { handlers, subject, open } = renderRow("read");

    fireEvent.keyDown(open, { key: "F10", shiftKey: true });

    expect(handlers.onMenu).toHaveBeenCalledWith(subject, expect.anything());
  });
});
