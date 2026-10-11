import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { ActivityItem } from "../../gen/ActivityItem.ts";
import { mutations } from "../../store/store.ts";
import { ActivityRow } from "./activity-row.tsx";

const NOW = Date.parse("2026-10-06T12:00:00.000Z");

function item(state: ActivityItem["state"], creatorId: number | null = null): ActivityItem {
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
      creatorId,
      title: "general",
      body: "Can you review the launch plan?",
      occurredAt: "2026-10-06T11:00:00.000Z",
      approvalStatus: null,
      budgetCap: null,
      path: "/rooms/12/@9001",
    },
  };
}

function renderRow(state: ActivityItem["state"], creatorId: number | null = null) {
  const handlers = { onOpen: vi.fn(), onAction: vi.fn(), onMenu: vi.fn() };
  const subject = item(state, creatorId);

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

afterEach(() => mutations.reset());

describe("an activity row", () => {
  it("collapses cached reminders when another session changes ignore", () => {
    renderRow("unread", 8);
    act(() =>
      mutations.applyEvents(
        [
          {
            seq: 1,
            topic: "user",
            type: "blocks.updated",
            data: { userId: 8, ignoredByMe: true, revision: 1 },
          },
        ],
        NOW,
      ),
    );
    expect(screen.queryByText("Can you review the launch plan?")).toBeNull();
    act(() =>
      mutations.applyEvents(
        [
          {
            seq: 2,
            topic: "user",
            type: "blocks.updated",
            data: { userId: 8, ignoredByMe: false, revision: 2 },
          },
        ],
        NOW,
      ),
    );
    expect(screen.getByText("Can you review the launch plan?")).toBeTruthy();
  });
  it("collapses an ignored saved reminder until the viewer reveals it", () => {
    const reminder = item("unread", 8);

    const source: ActivityItem["source"] & { readonly ignored: boolean } = {
      ...reminder.source,
      sourceType: "saved_item",
      ignored: true,
    };

    const subject: ActivityItem = { ...reminder, eventType: "message_reminder", source };
    render(
      <ActivityRow
        item={subject}
        now={NOW}
        motion={undefined}
        celebrate={false}
        onOpen={vi.fn()}
        onAction={vi.fn()}
        onMenu={vi.fn()}
      />,
    );
    expect(screen.queryByText("Can you review the launch plan?")).toBeNull();
    expect(screen.queryByRole("button", { name: /Unread/ })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Show ignored message" }));
    expect(screen.getByText("Can you review the launch plan?")).toBeTruthy();
  });
  it("names a live item's creator as soon as their profile lands", () => {
    renderRow("unread", 8);

    expect(screen.queryByText("Lucía Fernández")).toBeNull();

    act(() => mutations.mergeUsers([userFixture(8, "Lucía Fernández")]));

    expect(screen.getByText("Lucía Fernández")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Lucía Fernández/ })).toBeTruthy();
  });

  it("shows the kind, title and excerpt, and opens on click", () => {
    const { handlers, subject, open } = renderRow("unread");

    expect(screen.getByText("Mention")).toBeTruthy();
    expect(screen.getByText("Can you review the launch plan?")).toBeTruthy();

    fireEvent.click(open);

    expect(handlers.onOpen).toHaveBeenCalledWith(subject);
  });

  it("names the row by its summary and describes it with the excerpt", () => {
    renderRow("unread");

    const open = screen.getByRole("button", { name: /Unread, Mention, general/ });

    expect(open.getAttribute("aria-label")).not.toContain("launch plan");
    expect(screen.getByRole("button", { description: "Can you review the launch plan?" })).toBe(
      open,
    );
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
