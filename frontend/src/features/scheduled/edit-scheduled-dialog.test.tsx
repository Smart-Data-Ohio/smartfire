import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import type { MessagePage } from "../../gen/MessagePage.ts";
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import { actions } from "../../sync/runtime.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { snippet } from "../messages/message-content.tsx";
import { EditScheduledDialog } from "./edit-scheduled-dialog.tsx";
import { ScheduledRow } from "./scheduled-row.tsx";

const ITEM: ScheduledMessage = {
  id: 1,
  roomId: 12,
  threadId: null,
  replyToMessageId: null,
  replyTarget: null,
  attachments: [],
  markdownSource: "Hello",
  excerpt: "Hello",
  sendAt: new Date(2030, 9, 7, 9, 0).toISOString(),
  state: "pending",
  sendable: true,
  sentAt: null,
  sentMessageId: null,
  droppedAt: null,
  dropReason: null,
  createdAt: new Date(2026, 9, 6, 8, 0).toISOString(),
};

let network: MockNetwork;

beforeAll(() => {
  network = installMockNetwork();
});

afterAll(() => network.restore());

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("the scheduled message editor", () => {
  it("shows a stored reply excerpt without loading the target's timeline", () => {
    render(
      <ScheduledRow
        item={{
          ...ITEM,
          replyToMessageId: 55,
          replyTarget: {
            messageId: 55,
            roomId: ITEM.roomId,
            threadId: null,
            creatorId: 7,
            authorName: "Maya",
            excerpt: "Can you review the draft?",
            roomLabel: "general",
            createdAt: ITEM.createdAt,
          },
        }}
        conversation={null}
        now={Date.now()}
        motion={undefined}
        sending={false}
        handlers={{
          onEdit: () => {},
          onReschedule: () => {},
          onSendNow: () => {},
          onCancel: () => {},
          onView: () => {},
          onMenu: () => {},
        }}
      />,
    );
    expect(screen.getByText("Replying to Maya: Can you review the draft?")).toBeDefined();
  });

  it.each([
    { roomId: SEED_IDS.rooms.general, threadId: null },
    { roomId: SEED_IDS.rooms.design, threadId: SEED_IDS.threads.design },
  ])("chooses a reply inside room $roomId and thread $threadId", async ({ roomId, threadId }) => {
    const user = userEvent.setup();

    const path =
      threadId === null
        ? `/api/v1/rooms/${roomId}/messages`
        : `/api/v1/threads/${threadId}/messages`;

    const page: MessagePage = await (await fetch(path)).json();
    const target = page.messages.find((message) => !message.systemNote);

    if (target === undefined) throw new Error("Expected a reply candidate");
    const saved: unknown[] = [];
    render(
      <EditScheduledDialog
        item={{ ...ITEM, roomId, threadId }}
        onClose={() => undefined}
        onSave={async (_item, edit) => {
          saved.push(edit);
        }}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Choose reply target" }));
    await screen.findByRole("option", {
      name: `${page.users.find((person) => person.id === target.creatorId)?.name ?? "Someone"}: ${snippet(target)}`,
    });
    await user.selectOptions(screen.getByLabelText("Reply target"), String(target.id));
    await user.click(screen.getByRole("button", { name: "Save changes" }));
    expect(saved).toEqual([{ replyToMessageId: target.id }]);
  });

  it("clears a reply without changing the message or send time", async () => {
    const user = userEvent.setup();
    const saved: unknown[] = [];
    render(
      <EditScheduledDialog
        item={{ ...ITEM, sendAt: "2020-01-01T09:00:00.000Z", replyToMessageId: 55 }}
        onClose={() => undefined}
        onSave={async (_item, edit) => {
          saved.push(edit);
        }}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Clear reply" }));
    await user.click(screen.getByRole("button", { name: "Save changes" }));
    expect(saved).toEqual([{ replyToMessageId: null }]);
  });

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

it("shows scheduled file thumbnails and filenames", () => {
  render(
    <ScheduledRow
      item={{
        ...ITEM,
        attachments: [
          {
            signedId: "signed-one",
            attachment: {
              filename: "one.png",
              contentType: "image/png",
              byteSize: 5,
              width: 2,
              height: 2,
              preview: "image",
              url: "/one.png",
              downloadUrl: "/one.png?download",
              thumbnailUrl: "/thumb.png",
            },
          },
        ],
      }}
      conversation={null}
      now={Date.now()}
      motion={undefined}
      sending={false}
      handlers={{
        onEdit: () => {},
        onReschedule: () => {},
        onSendNow: () => {},
        onCancel: () => {},
        onView: () => {},
        onMenu: () => {},
      }}
    />,
  );
  expect(screen.getByText("one.png")).toBeDefined();
  expect(screen.getByAltText("one.png").getAttribute("src")).toBe("/thumb.png");
});

it("removes a scheduled file while retaining the other signed upload", async () => {
  const user = userEvent.setup();
  const saved: unknown[] = [];

  const attachment = {
    filename: "one.txt",
    contentType: "text/plain",
    byteSize: 5,
    width: null,
    height: null,
    preview: "file",
    url: "/one",
    downloadUrl: "/one?download",
    thumbnailUrl: null,
  } as const;

  render(
    <EditScheduledDialog
      item={{
        ...ITEM,
        attachments: [
          { signedId: "signed-one", attachment },
          { signedId: "signed-two", attachment: { ...attachment, filename: "two.txt" } },
        ],
      }}
      onClose={() => undefined}
      onSave={async (_item, edit) => {
        saved.push(edit);
      }}
    />,
  );
  await user.click(screen.getByRole("button", { name: "Remove one.txt" }));
  await user.click(screen.getByRole("button", { name: "Save changes" }));
  expect(saved).toEqual([{ attachmentSignedIds: ["signed-two"] }]);
});

it("adds an uploaded file after retained files and waits for its upload", async () => {
  const user = userEvent.setup();
  const saved: unknown[] = [];
  const upload = Promise.withResolvers<{ signedId: string; uploadUrl: string }>();
  const startUpload = actions.messages.startUpload;
  vi.spyOn(actions.messages, "startUpload").mockReturnValue(upload.promise);
  render(
    <EditScheduledDialog
      item={{
        ...ITEM,
        attachments: [
          {
            signedId: "retained",
            attachment: {
              filename: "kept.txt",
              contentType: "text/plain",
              byteSize: 5,
              width: null,
              height: null,
              preview: "file",
              url: "/kept",
              downloadUrl: "/kept?download",
              thumbnailUrl: null,
            },
          },
        ],
      }}
      onClose={() => undefined}
      onSave={async (_item, edit) => {
        saved.push(edit);
      }}
    />,
  );
  fireEvent.change(screen.getByLabelText("Add files"), {
    target: { files: [new File(["hello"], "added.txt", { type: "text/plain" })] },
  });
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Save changes" }).hasAttribute("disabled")).toBe(
      true,
    ),
  );

  const meta = document.createElement("meta");
  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);

  const direct = await startUpload({
    filename: "added.txt",
    contentType: "text/plain",
    byteSize: 5,
    checksum: "XUFAKrxLKna5cZ2REBfFkg==",
  });

  meta.remove();
  await act(async () => upload.resolve(direct));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Save changes" }).hasAttribute("disabled")).toBe(
      false,
    ),
  );
  await user.click(screen.getByRole("button", { name: "Save changes" }));
  expect(saved).toEqual([{ attachmentSignedIds: ["retained", direct.signedId] }]);
});
