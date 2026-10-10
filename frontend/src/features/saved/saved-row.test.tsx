import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { messageFixture } from "../threads/test-fixtures.ts";
import { SavedRow } from "./saved-row.tsx";

const handlers = {
  onOpen: vi.fn(),
  onToggleDone: vi.fn(),
  onRemind: vi.fn(),
  onCustomRemind: vi.fn(),
  onRemove: vi.fn(),
  onMenu: vi.fn(),
};

describe("SavedRow", () => {
  it("reveals a spoiler from the keyboard without opening the row", async () => {
    const user = userEvent.setup();

    const message = messageFixture(3, {
      bodyHtml: '<p>see <span class="spoiler" data-spoiler="">the ending</span></p>',
    });

    render(
      <SavedRow
        item={{
          id: 1,
          messageId: 3,
          status: "in_progress",
          remindAt: null,
          remindedAt: null,
          createdAt: "2026-10-06T09:00:00.000Z",
        }}
        message={message}
        conversation={null}
        now={Date.parse("2026-10-06T16:00:00.000Z")}
        motion={undefined}
        celebrate={false}
        handlers={handlers}
      />,
    );

    const spoiler = screen.getByRole("button", { name: "Spoiler, activate to reveal" });

    expect(spoiler.textContent).toBe("the ending");

    spoiler.focus();
    await user.keyboard("{Enter}");

    expect(spoiler.hasAttribute("data-revealed")).toBe(true);
    expect(spoiler.textContent).toBe("the ending");
    expect(handlers.onOpen).not.toHaveBeenCalled();
  });
});
