import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { emitResync, emitSyncEvents, messageFixture } from "../../api/testing.ts";
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

function created(message: MessageDTO) {
  const events = [
    { seq: 1, topic: `room:${message.roomId}`, type: "message.created", data: message },
  ] satisfies Parameters<typeof emitSyncEvents>[0];

  mutations.applyEvents(events, Date.now());
  emitSyncEvents(events);
}

function removed(id: number, roomId = ROOM) {
  const events = [
    {
      seq: 1,
      topic: `room:${roomId}`,
      type: "message.removed",
      data: { id, roomId, threadId: null },
    },
  ] satisfies Parameters<typeof emitSyncEvents>[0];

  mutations.applyEvents(events, Date.now());
  emitSyncEvents(events);
}

const freshAttachment = {
  ...file.attachment,
  filename: "fresh.png",
  url: "/files/fresh.png",
  downloadUrl: "/files/fresh.png?disposition=attachment",
  thumbnailUrl: "/thumbs/fresh.png",
};

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
  vi.useRealTimers();
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

it("shows a file uploaded while the pane is open, newest first, without refetching", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  const { container } = await mount();

  await waitFor(() =>
    expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull(),
  );
  act(() => created(messageFixture(2, ROOM, { attachment: freshAttachment })));

  const rows = screen.getAllByRole("listitem");

  expect(rows).toHaveLength(2);
  expect(rows[0]?.querySelector('img[src="/thumbs/fresh.png"]')).not.toBeNull();
  expect(files).toHaveBeenCalledTimes(1);
});

it("reads the attach time when an edit adds a file to a message", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  const { container } = await mount();

  await waitFor(() =>
    expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull(),
  );
  files.mockResolvedValue({
    ...publicList,
    files: [
      { ...file, messageId: 2, attachment: freshAttachment, createdAt: "2026-10-06T00:00:09.000Z" },
      file,
    ],
  });
  vi.useFakeTimers();
  act(() =>
    updated(
      messageFixture(2, ROOM, {
        attachment: freshAttachment,
        updatedAt: "2026-10-06T00:00:09.000Z",
      }),
    ),
  );

  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(screen.getAllByRole("listitem")).toHaveLength(2);
  expect(container.querySelector('img[src="/thumbs/fresh.png"]')).not.toBeNull();
  expect(files).toHaveBeenCalledTimes(2);
});

it("drops every file of a message deleted while the pane is open, without refetching", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  const { container } = await mount();

  await waitFor(() =>
    expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull(),
  );
  act(() => removed(file.messageId));

  expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
  expect(screen.getByText("No files shared here yet.")).toBeTruthy();
  expect(files).toHaveBeenCalledTimes(1);
});

it("keeps a deleted message's files out of a page still being read", async () => {
  const stale = Promise.withResolvers<FileList>();
  const files = vi.spyOn(panes, "files").mockImplementation(() => stale.promise);

  const { container } = await mount();

  await waitFor(() => expect(files).toHaveBeenCalledTimes(1));
  act(() => removed(file.messageId));
  await act(async () => stale.resolve(publicList));

  expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
  expect(files).toHaveBeenCalledTimes(1);
});

it("shows an upload that arrives while the first page is being read", async () => {
  const stale = Promise.withResolvers<FileList>();
  const files = vi.spyOn(panes, "files").mockImplementation(() => stale.promise);

  const { container } = await mount();

  await waitFor(() => expect(files).toHaveBeenCalledTimes(1));
  act(() => created(messageFixture(2, ROOM, { attachment: freshAttachment })));
  await act(async () => stale.resolve(publicList));

  expect(screen.getAllByRole("listitem")).toHaveLength(2);
  expect(container.querySelector('img[src="/thumbs/fresh.png"]')).not.toBeNull();
  expect(files).toHaveBeenCalledTimes(1);
});

it("does not show a new document on the Images tab, but shows a new image", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  const { container } = await mount();

  fireEvent.click(screen.getByRole("tab", { name: "Images" }));
  await waitFor(() =>
    expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull(),
  );
  act(() =>
    created(
      messageFixture(3, ROOM, {
        attachment: {
          ...freshAttachment,
          filename: "notes.txt",
          contentType: "text/plain",
          preview: "file",
          url: "/files/notes.txt",
          downloadUrl: "/files/notes.txt?disposition=attachment",
          thumbnailUrl: null,
        },
      }),
    ),
  );

  expect(screen.getAllByRole("listitem")).toHaveLength(1);
  expect(screen.queryByText("notes.txt")).toBeNull();

  act(() => created(messageFixture(4, ROOM, { attachment: freshAttachment })));

  expect(screen.getAllByRole("listitem")).toHaveLength(2);
  expect(container.querySelector('img[src="/thumbs/fresh.png"]')).not.toBeNull();
  expect(files).toHaveBeenCalledTimes(2);
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

function filesChanged(roomId = ROOM, topic = `room:${roomId}`) {
  emitSyncEvents([{ seq: 1, topic, type: "room.files.changed", data: { roomId, threadId: 8 } }]);
}

function threadRemoved(threadId: number) {
  emitSyncEvents([
    {
      seq: 2,
      topic: "user",
      type: "thread.removed",
      data: { roomId: ROOM, threadId },
    },
  ]);
}

it("debounces reply changes and silently refreshes both loaded pages without moving the scroll", async () => {
  const { entries, files } = paginatedFiles();
  const { container, unmount } = await loadBothPages();
  const body = container.querySelector(".pane-body");

  if (body === null) throw new Error("Missing pane body");
  body.scrollTop = 720;
  const pending = Promise.withResolvers<FileList>();

  files.mockImplementationOnce(() => pending.promise);
  entries.unshift({ ...file, messageId: 1000, threadId: 8, attachment: freshAttachment });
  vi.useFakeTimers();
  act(() => {
    filesChanged();
    filesChanged();
    filesChanged(ROOM + 1);
  });
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(screen.getAllByRole("listitem")).toHaveLength(60);
  expect(body.scrollTop).toBe(720);
  await act(async () =>
    pending.resolve({ ...publicList, files: entries.slice(0, 30), nextPage: 2 }),
  );
  expect(files.mock.calls.map(([, query]) => query.page)).toEqual([1, 2, 1, 2]);
  expect(screen.getAllByRole("listitem")).toHaveLength(60);
  expect(screen.getByText("fresh.png")).toBeTruthy();
  expect(screen.queryByText("59.png")).toBeNull();
  expect(body.scrollTop).toBe(720);
  act(filesChanged);
  unmount();
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(files).toHaveBeenCalledTimes(4);
});

it("follows an active refresh with a read for a newer private reply change", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
  const stale = Promise.withResolvers<FileList>();

  files.mockImplementationOnce(() => stale.promise);
  vi.useFakeTimers();
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  files.mockResolvedValue({ ...publicList, files: [] });
  act(() => filesChanged(ROOM, "user"));
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(files).toHaveBeenCalledTimes(2);
  await act(async () => stale.resolve(publicList));
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  expect(files).toHaveBeenCalledTimes(3);
});

it("drops a removed thread's files and keeps them out of a response already in flight", async () => {
  const threadFile = { ...file, threadId: 8 };
  const pending = Promise.withResolvers<FileList>();
  const files = vi.spyOn(panes, "files").mockResolvedValue({ ...publicList, files: [threadFile] });

  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
  files.mockImplementationOnce(() => pending.promise);
  vi.useFakeTimers();
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  act(() => threadRemoved(8));
  expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  await act(async () => pending.resolve({ ...publicList, files: [threadFile] }));
  expect(screen.queryAllByRole("listitem")).toHaveLength(0);
});

it("keeps visible files after a failed refresh and retries on the next change", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
  vi.useFakeTimers();
  files.mockRejectedValueOnce(new Error("Offline"));
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(screen.getByText("image.png")).toBeTruthy();
  expect(screen.queryByText("The files couldn't be loaded.")).toBeNull();
  files.mockResolvedValue({ ...publicList, files: [] });
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(screen.queryAllByRole("listitem")).toHaveLength(0);
});

it("uses the active type and search when refreshing a reply's files", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  const { unmount } = await mount();

  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
  fireEvent.click(screen.getByRole("tab", { name: "Images" }));
  fireEvent.change(screen.getByRole("searchbox", { name: "Search files by name" }), {
    target: { value: " image " },
  });
  await waitFor(() =>
    expect(files).toHaveBeenLastCalledWith(ROOM, {
      type: "images",
      filename: "image",
      page: 1,
    }),
  );
  const calls = files.mock.calls.length;

  vi.useFakeTimers();
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(files).toHaveBeenCalledTimes(calls + 1);
  expect(files).toHaveBeenLastCalledWith(ROOM, { type: "images", filename: "image", page: 1 });
  unmount();
});

it("uses refreshed thread rows instead of retaining an older thread message overlay", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
  act(() => created(messageFixture(2, ROOM, { threadId: 8, attachment: freshAttachment })));
  files.mockResolvedValue({
    ...publicList,
    files: [
      { ...file, messageId: 2, threadId: 8, attachment: { ...freshAttachment, spoiler: true } },
    ],
  });
  vi.useFakeTimers();
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(screen.getByRole("button", { name: "Spoiler, activate to reveal" })).toBeTruthy();
  files.mockResolvedValue({ ...publicList, files: [] });
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(screen.queryAllByRole("listitem")).toHaveLength(0);
});

it("retains an initial read's error when a pending reply refresh also fails", async () => {
  const stale = Promise.withResolvers<FileList>();
  const files = vi.spyOn(panes, "files").mockImplementationOnce(() => stale.promise);

  await mount();
  await waitFor(() => expect(files).toHaveBeenCalledTimes(1));
  files.mockRejectedValueOnce(new Error("Offline"));
  vi.useFakeTimers();
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(files).toHaveBeenCalledTimes(1);
  await act(async () => stale.reject(new Error("Offline")));
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(screen.getByText("The files couldn't be loaded.")).toBeTruthy();
  expect(files).toHaveBeenCalledTimes(2);
});

it("keeps a room upload that arrives while reply files are being refreshed", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
  const pending = Promise.withResolvers<FileList>();

  files.mockImplementationOnce(() => pending.promise);
  vi.useFakeTimers();
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  await act(async () => {
    threadRemoved(99);
    created(messageFixture(2, ROOM, { attachment: freshAttachment }));
    pending.resolve(publicList);
    await Promise.resolve();
  });
  expect(screen.getByText("fresh.png")).toBeTruthy();
  expect(screen.getAllByRole("listitem")).toHaveLength(2);
});

it("keeps deleted files out after creation, editing and removal arrive in one replay batch", async () => {
  vi.spyOn(panes, "files").mockResolvedValue(publicList);
  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));

  const events = [
    {
      seq: 1,
      topic: `room:${ROOM}`,
      type: "message.created",
      data: messageFixture(2, ROOM, { attachment: freshAttachment }),
    },
    {
      seq: 2,
      topic: `room:${ROOM}`,
      type: "message.updated",
      data: messageFixture(2, ROOM, {
        attachment: freshAttachment,
        updatedAt: "2026-10-06T20:30:00Z",
      }),
    },
    {
      seq: 3,
      topic: `room:${ROOM}`,
      type: "message.removed",
      data: { id: 2, roomId: ROOM, threadId: null },
    },
  ] satisfies Parameters<typeof emitSyncEvents>[0];

  act(() => {
    mutations.applyEvents(events, Date.now());
    emitSyncEvents(events);
  });
  expect(screen.queryByText("fresh.png")).toBeNull();
  expect(screen.getAllByRole("listitem")).toHaveLength(1);
});

it.each(["upload", "delete"])(
  "preserves a reply file refresh when pagination starts during the read (%s)",
  async (change) => {
    const { entries, files } = paginatedFiles();
    const oldest = entries[0];

    if (oldest === undefined) throw new Error("Missing first file");
    entries[0] = { ...oldest, threadId: 8 };
    await mount();
    await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(30));
    const pending = Promise.withResolvers<FileList>();

    files.mockImplementationOnce(() => pending.promise);

    if (change === "upload") {
      entries.unshift({ ...file, messageId: 1000, threadId: 8, attachment: freshAttachment });
    } else {
      entries.shift();
    }

    vi.useFakeTimers();
    act(filesChanged);
    await act(() => vi.advanceTimersByTimeAsync(100));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Load more" })));
    await act(async () =>
      pending.resolve({ ...publicList, files: entries.slice(0, 30), nextPage: 2 }),
    );
    await act(() => vi.advanceTimersByTimeAsync(100));

    if (change === "upload") {
      expect(screen.getByText("fresh.png")).toBeTruthy();
    } else {
      expect(screen.queryByText("0.png")).toBeNull();
    }
  },
);

it("review regression: rejects a deleted message update in a later batch", async () => {
  vi.spyOn(panes, "files").mockResolvedValue(publicList);
  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
  act(() => removed(file.messageId));
  act(() => updated(messageFixture(file.messageId, ROOM, { attachment: file.attachment })));
  expect(screen.queryAllByRole("listitem")).toHaveLength(0);
});

it.each([`room:${ROOM}`, "user", "reply refresh"])(
  "review regression: retires covered overlays on %s resync and keeps concurrent uploads",
  async (topic) => {
    const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
    await mount();
    await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
    act(() => created(messageFixture(2, ROOM, { attachment: freshAttachment })));
    const pending = Promise.withResolvers<FileList>();

    files.mockImplementationOnce(() => pending.promise);
    vi.useFakeTimers();
    act(() => (topic === "reply refresh" ? filesChanged() : emitResync([topic])));
    await act(() => vi.advanceTimersByTimeAsync(100));
    expect(files).toHaveBeenCalledTimes(2);
    act(() =>
      created(
        messageFixture(3, ROOM, {
          attachment: {
            ...freshAttachment,
            filename: "concurrent.png",
            url: "/files/concurrent.png",
          },
        }),
      ),
    );
    await act(async () => pending.resolve(publicList));
    expect(screen.queryByText("fresh.png")).toBeNull();
    expect(screen.getByText("concurrent.png")).toBeTruthy();
  },
);

it("review regression: coalesces changes while a refresh is active", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
  const first = Promise.withResolvers<FileList>();
  const followup = Promise.withResolvers<FileList>();

  files.mockImplementationOnce(() => first.promise).mockImplementationOnce(() => followup.promise);
  vi.useFakeTimers();
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(150));
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(150));
  expect(files).toHaveBeenCalledTimes(2);
  await act(async () => first.resolve({ ...publicList, files: [] }));
  expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(files).toHaveBeenCalledTimes(3);
  await act(async () => followup.resolve(publicList));
  expect(screen.getAllByRole("listitem")).toHaveLength(1);
});

it("review regression: refreshes unknown attachment times on an older text edit", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue({ ...publicList, nextPage: 2 });
  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));

  const old = {
    ...file,
    messageId: 2,
    attachment: freshAttachment,
    createdAt: "2025-01-01T00:00:00.000Z",
  };

  files.mockResolvedValue({ ...publicList, files: [file, old], nextPage: null });
  vi.useFakeTimers();
  act(() =>
    updated(
      messageFixture(2, ROOM, {
        attachment: freshAttachment,
        createdAt: old.createdAt,
        updatedAt: "2026-10-10T00:00:00.000Z",
      }),
    ),
  );
  expect(screen.queryByText("fresh.png")).toBeNull();
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(files).toHaveBeenCalledTimes(2);
  expect(
    screen.getAllByRole("listitem").map((item) => item.querySelector("time")?.dateTime),
  ).toEqual([file.createdAt, old.createdAt]);
});

const pdfAttachment: RoomFile["attachment"] = {
  ...file.attachment,
  filename: "notes.pdf",
  contentType: "application/pdf",
  width: null,
  height: null,
  preview: "file",
  url: "/files/notes.pdf",
  downloadUrl: "/files/notes.pdf?disposition=attachment",
  thumbnailUrl: null,
};

it("retires a mixed image and PDF overlay after a missed deletion and Images resync", async () => {
  const empty: FileList = { files: [], users: [], nextPage: null };
  const files = vi.spyOn(panes, "files").mockResolvedValue(empty);
  await mount();
  fireEvent.click(screen.getByRole("tab", { name: "Images" }));
  await waitFor(() => expect(screen.getByText("No images shared here yet.")).toBeTruthy());
  act(() =>
    created(
      messageFixture(2, ROOM, {
        attachment: null,
        attachments: [freshAttachment, pdfAttachment],
      }),
    ),
  );
  expect(screen.getByRole("link", { name: "Open fresh.png" })).toBeTruthy();
  const pending = Promise.withResolvers<FileList>();
  const calls = files.mock.calls.length;

  files.mockImplementationOnce(() => pending.promise);
  vi.useFakeTimers();
  // The deletion happened offline: only the empty authoritative read reports it.
  act(() => emitResync([`room:${ROOM}`]));
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(files).toHaveBeenCalledTimes(calls + 1);
  act(() =>
    created(
      messageFixture(3, ROOM, {
        attachment: null,
        attachments: [
          { ...freshAttachment, filename: "concurrent.png", url: "/files/concurrent.png" },
          pdfAttachment,
        ],
      }),
    ),
  );
  await act(async () => pending.resolve(empty));
  expect(screen.queryByRole("link", { name: "Open fresh.png" })).toBeNull();
  expect(screen.getByRole("link", { name: "Open concurrent.png" })).toBeTruthy();
  expect(screen.getAllByRole("listitem")).toHaveLength(1);
});

it("starts the dirty follow-up refresh while events continue every 50 ms", async () => {
  const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
  await mount();
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(1));
  const first = Promise.withResolvers<FileList>();
  const followup = Promise.withResolvers<FileList>();

  files.mockImplementationOnce(() => first.promise).mockImplementationOnce(() => followup.promise);
  vi.useFakeTimers();
  act(filesChanged);
  await act(() => vi.advanceTimersByTimeAsync(100));
  let notifications = 0;

  const timer = window.setInterval(() => {
    notifications += 1;
    filesChanged();
  }, 50);

  try {
    await act(() => vi.advanceTimersByTimeAsync(50));
    expect(files).toHaveBeenCalledTimes(2);
    await act(async () => first.resolve(publicList));
    await act(() => vi.advanceTimersByTimeAsync(200));
    expect(notifications).toBe(5);
    expect(files).toHaveBeenCalledTimes(3);
    await act(async () => followup.resolve({ ...publicList, files: [] }));
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  } finally {
    window.clearInterval(timer);
  }
});

it("applies a known image spoiler and removal with an unknown PDF sibling despite refresh failure", async () => {
  const files = vi
    .spyOn(panes, "files")
    .mockResolvedValue({ ...publicList, files: [file, { ...file, attachment: freshAttachment }] });

  const { container } = await mount();

  fireEvent.click(screen.getByRole("tab", { name: "Images" }));
  await waitFor(() => expect(files).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(2));
  expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull();
  expect(container.querySelector('img[src="/thumbs/fresh.png"]')).not.toBeNull();
  files.mockRejectedValueOnce(new Error("Offline"));
  vi.useFakeTimers();
  act(() =>
    updated(
      messageFixture(file.messageId, ROOM, {
        attachment: null,
        attachments: [{ ...file.attachment, spoiler: true }, pdfAttachment],
      }),
    ),
  );
  expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
  expect(container.querySelector('img[src="/thumbs/fresh.png"]')).toBeNull();
  expect(screen.getByRole("button", { name: "Spoiler, activate to reveal" })).toBeTruthy();
  await act(() => vi.advanceTimersByTimeAsync(100));
  expect(files).toHaveBeenCalledTimes(3);
  expect(screen.getAllByRole("listitem")).toHaveLength(1);
  expect(screen.getByRole("button", { name: "Spoiler, activate to reveal" })).toBeTruthy();
  expect(container.querySelector("img")).toBeNull();
});
