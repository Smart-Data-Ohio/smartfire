import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { MessageDTO } from "../../store/model.ts";
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

  afterEach(() => {
    delete document.documentElement.dataset.motion;
  });

  it("shows an animated workspace icon's first frame under reduced motion", () => {
    document.documentElement.dataset.motion = "reduce";

    const message = messageFixture(3, {
      bodyHtml:
        '<p>go <img class="icon icon--custom" src="/icons/dance" alt=":dance:" title="Dance" draggable="false"></p>',
    });

    const { container } = render(
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

    expect(container.querySelector(".saved-body img")?.getAttribute("src")).toBe(
      "/icons/dance?still=1",
    );
  });
});

const WAVE = {
  kind: "available",
  id: 1,
  name: "Wave",
  imageUrl: "/stickers/1",
  stillUrl: "/stickers/1",
  animated: false,
} as const;

const DANCE = {
  kind: "available",
  id: 2,
  name: "Dance",
  imageUrl: "/stickers/2",
  stillUrl: "/stickers/2?still=1",
  animated: true,
} as const;

function renderSaved(message: MessageDTO) {
  return render(
    <SavedRow
      item={{
        id: 1,
        messageId: message.id,
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
}

describe("a saved sticker message", () => {
  afterEach(() => {
    cleanup();
    delete document.documentElement.dataset.motion;
  });

  it("shows a sticker-only message's sticker instead of an empty row", () => {
    renderSaved(messageFixture(3, { bodyHtml: "", markdownSource: "", sticker: WAVE }));

    const image = screen.getByRole("img", { name: "Sticker: Wave" });

    expect(image.getAttribute("src")).toBe("/stickers/1");
    expect(image.getAttribute("width")).toBe("160");
  });

  it("shows a captioned sticker under its caption", () => {
    const { container } = renderSaved(
      messageFixture(3, { bodyHtml: "<p>hi all</p>", markdownSource: "hi all", sticker: WAVE }),
    );

    expect(container.querySelector(".saved-body")?.textContent).toBe("hi all");
    expect(screen.getByRole("img", { name: "Sticker: Wave" })).toBeTruthy();
  });

  it("rests an animated sticker on its first frame under reduced motion", () => {
    document.documentElement.dataset.motion = "reduce";

    renderSaved(messageFixture(3, { bodyHtml: "", markdownSource: "", sticker: DANCE }));

    expect(screen.getByRole("img", { name: "Sticker: Dance" }).getAttribute("src")).toBe(
      "/stickers/2?still=1",
    );
  });

  it("leaves the deleted-sticker placeholder with its name", () => {
    const { container } = renderSaved(
      messageFixture(3, {
        bodyHtml: "",
        markdownSource: "",
        sticker: { kind: "deleted", name: "Dance" },
      }),
    );

    expect(container.textContent).toContain("Dance · Sticker deleted");
    expect(screen.queryByRole("img", { name: "Sticker: Dance" })).toBeNull();
  });
});
