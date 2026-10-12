import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { emitSyncEvents, messageFixture } from "../../api/testing.ts";
import type { FileList } from "../../gen/FileList.ts";
import type { MessageDTO } from "../../gen/MessageDTO.ts";
import type { RoomFile } from "../../gen/RoomFile.ts";
import { mutations, store } from "../../store/store.ts";
import { panes } from "../../sync/panes.ts";
import { FilesPane } from "./files-pane.tsx";

const ROOM = 12;

const file: RoomFile = {
  messageId: 1,
  threadId: null,
  creatorId: 7,
  createdAt: "2026-10-06T00:00:01.000Z",
  attachment: {
    filename: "image.png",
    contentType: "image/png",
    byteSize: 100,
    width: 100,
    height: 100,
    preview: "image",
    spoiler: false,
    url: "/files/image.png",
    downloadUrl: "/files/image.png?disposition=attachment",
    thumbnailUrl: "/thumbs/image.png",
  },
};

const publicList: FileList = { files: [file], users: [], nextPage: null };

async function mount() {
  const root = createRootRoute({ component: () => <FilesPane roomId={ROOM} /> });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  const mounted = render(<RouterProvider router={router} />);

  await act(() => router.load());

  return mounted;
}

function updated(message: MessageDTO) {
  const events = [
    { seq: 1, topic: `room:${message.roomId}`, type: "message.updated", data: message },
  ] satisfies Parameters<typeof emitSyncEvents>[0];

  mutations.applyEvents(events, Date.now());
  emitSyncEvents(events);
}

function edited(roomId = ROOM) {
  updated(
    messageFixture(file.messageId, roomId, {
      attachment: { ...file.attachment, spoiler: true },
      updatedAt: "2026-10-06T00:00:02.000Z",
    }),
  );
}

function paginatedFiles() {
  const entries = Array.from(
    { length: 60 },
    (_, index): RoomFile => ({
      ...file,
      messageId: index + 1,
      attachment: {
        ...file.attachment,
        url: `/files/${index}.png`,
        thumbnailUrl: `/thumbs/${index}.png`,
        filename: `${index}.png`,
      },
    }),
  );

  const files = vi.spyOn(panes, "files").mockImplementation(async (_roomId, query) => ({
    ...publicList,
    files: entries.slice((query.page - 1) * 30, query.page * 30),
    nextPage: query.page === 1 ? 2 : null,
  }));

  return { entries, files };
}

async function loadBothPages() {
  const mounted = await mount();

  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(30));
  fireEvent.click(screen.getByRole("button", { name: "Load more" }));
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(60));

  return mounted;
}

afterEach(() => {
  cleanup();
  mutations.reset();
  vi.restoreAllMocks();
});

it.each(["All", "Images"])(
  "covers a cached public image after a live spoiler edit in %s, even outside the timeline",
  async (tab) => {
    const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
    const { container, unmount } = await mount();

    if (tab === "Images") {
      fireEvent.click(screen.getByRole("tab", { name: tab }));
    }

    await waitFor(() =>
      expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull(),
    );
    expect(store.getState().messages[file.messageId]).toBeUndefined();
    const calls = files.mock.calls.length;

    act(edited);

    expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
    expect(files).toHaveBeenCalledTimes(calls);

    expect(screen.getByRole("button", { name: "Spoiler, activate to reveal" })).toBeTruthy();
    expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Spoiler, activate to reveal" }));
    expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull();
    unmount();
    act(edited);
    expect(files).toHaveBeenCalledTimes(calls);
  },
);

it("reconciles an older public files response when a spoiler edit arrives during the read", async () => {
  const stale = Promise.withResolvers<FileList>();
  const files = vi.spyOn(panes, "files").mockImplementation(() => stale.promise);

  const { container } = await mount();

  await waitFor(() => expect(files).toHaveBeenCalledTimes(1));
  act(edited);

  await act(async () => stale.resolve(publicList));

  expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
  expect(files).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("button", { name: "Spoiler, activate to reveal" })).toBeTruthy();
  expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
});

it("keeps the current files when a message in another room changes", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  const { container } = await mount();

  await waitFor(() =>
    expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull(),
  );
  act(() => edited(ROOM + 1));

  expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull();
  expect(files).toHaveBeenCalledTimes(1);
});

it("keeps both loaded pages and their scroll position after a text-only edit", async () => {
  const { files } = paginatedFiles();
  const { container } = await loadBothPages();

  const rows = screen.getAllByRole("listitem");
  const body = container.querySelector(".pane-body");

  expect(body).not.toBeNull();

  if (body !== null) {
    body.scrollTop = 720;
  }

  await act(async () =>
    updated(messageFixture(1000, ROOM, { markdownSource: "Edited text", attachment: null })),
  );

  const held = screen.getAllByRole("listitem");

  expect(held).toHaveLength(60);
  held.forEach((row, index) => {
    expect(row).toBe(rows[index]);
  });
  expect(body?.scrollTop).toBe(720);
  expect(files.mock.calls.map(([, query]) => query.page)).toEqual([1, 2]);
});

it("reconciles grouped spoiler flags and removals without losing either loaded page", async () => {
  const { entries, files } = paginatedFiles();
  const retained = entries[40];
  const removed = entries[41];

  if (retained === undefined || removed === undefined) throw new Error("Missing page-two files");
  entries[41] = { ...removed, messageId: retained.messageId };
  const { container } = await loadBothPages();
  const rows = screen.getAllByRole("listitem");

  expect(store.getState().messages[retained.messageId]).toBeUndefined();
  act(() =>
    updated(
      messageFixture(retained.messageId, ROOM, {
        attachment: retained.attachment,
        attachments: [{ ...retained.attachment, spoiler: true }],
      }),
    ),
  );

  const held = screen.getAllByRole("listitem");

  expect(held).toHaveLength(59);
  held.forEach((row, index) => {
    expect(row).toBe(rows[index < 41 ? index : index + 1]);
  });
  expect(container.querySelector('img[src="/thumbs/40.png"]')).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Spoiler, activate to reveal" }));
  expect(container.querySelector('img[src="/thumbs/40.png"]')).not.toBeNull();
  expect(files.mock.calls.map(([, query]) => query.page)).toEqual([1, 2]);
});

it("removes a cleared attachment from a pending page without restarting pagination", async () => {
  const { files } = paginatedFiles();
  const pending = Promise.withResolvers<FileList>();
  const { container } = await mount();

  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(30));
  files.mockImplementationOnce(() => pending.promise);
  fireEvent.click(screen.getByRole("button", { name: "Load more" }));
  act(() => updated(messageFixture(file.messageId, ROOM, { attachment: null })));
  await act(async () => pending.resolve(publicList));

  expect(screen.getAllByRole("listitem")).toHaveLength(29);
  expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
  expect(files.mock.calls.map(([, query]) => query.page)).toEqual([1, 2]);
});

it("makes a spoiler public again without refetching or replacing its row", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue({
    ...publicList,
    files: [{ ...file, attachment: { ...file.attachment, spoiler: true } }],
  });

  const { container } = await mount();

  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Spoiler, activate to reveal" })).toBeTruthy(),
  );
  const row = screen.getByRole("listitem");

  act(() => updated(messageFixture(file.messageId, ROOM, { attachment: file.attachment })));

  expect(screen.getByRole("listitem")).toBe(row);
  expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull();
  expect(files).toHaveBeenCalledTimes(1);
});

it("rejects a pending response from the previous file filter", async () => {
  const stale = Promise.withResolvers<FileList>();

  const files = vi
    .spyOn(panes, "files")
    .mockImplementationOnce(() => stale.promise)
    .mockResolvedValue({ ...publicList, files: [] });

  const { container } = await mount();

  fireEvent.click(screen.getByRole("tab", { name: "Images" }));
  await waitFor(() => expect(screen.getByText("No images shared here yet.")).toBeTruthy());
  await act(async () => stale.resolve(publicList));

  expect(screen.getByText("No images shared here yet.")).toBeTruthy();
  expect(container.querySelector("img")).toBeNull();
  expect(files).toHaveBeenCalledTimes(2);
});
