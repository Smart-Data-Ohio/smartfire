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

const hiddenList: FileList = {
  ...publicList,
  files: [{ ...file, attachment: { ...file.attachment, spoiler: true } }],
};

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

function edited(roomId = ROOM) {
  const message = messageFixture(file.messageId, roomId, {
    attachment: { ...file.attachment, spoiler: true },
    updatedAt: "2026-10-06T00:00:02.000Z",
  });

  const events = [
    { seq: 1, topic: `room:${roomId}`, type: "message.updated", data: message },
  ] satisfies Parameters<typeof emitSyncEvents>[0];

  mutations.applyEvents(events, Date.now());
  emitSyncEvents(events);
}

afterEach(() => {
  cleanup();
  mutations.reset();
  vi.restoreAllMocks();
});

it.each(["All", "Images"])(
  "covers a cached public image after a live spoiler edit in %s, even outside the timeline",
  async (tab) => {
    const refresh = Promise.withResolvers<FileList>();
    const files = vi.spyOn(panes, "files").mockResolvedValue(publicList);
    const { container, unmount } = await mount();

    if (tab === "Images") {
      fireEvent.click(screen.getByRole("tab", { name: tab }));
    }

    await waitFor(() =>
      expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull(),
    );
    expect(store.getState().messages[file.messageId]).toBeUndefined();
    files.mockImplementation(() => refresh.promise);
    const calls = files.mock.calls.length;

    act(edited);

    expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
    expect(files).toHaveBeenCalledTimes(calls + 1);

    await act(async () => refresh.resolve(hiddenList));

    expect(screen.getByRole("button", { name: "Spoiler, activate to reveal" })).toBeTruthy();
    expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Spoiler, activate to reveal" }));
    expect(container.querySelector('img[src="/thumbs/image.png"]')).not.toBeNull();
    unmount();
    act(edited);
    expect(files).toHaveBeenCalledTimes(calls + 1);
  },
);

it("rejects an older public files response when a spoiler edit arrives during the read", async () => {
  const stale = Promise.withResolvers<FileList>();
  const fresh = Promise.withResolvers<FileList>();

  const files = vi
    .spyOn(panes, "files")
    .mockImplementationOnce(() => stale.promise)
    .mockImplementationOnce(() => fresh.promise);

  const { container } = await mount();

  await waitFor(() => expect(files).toHaveBeenCalledTimes(1));
  act(edited);

  await act(async () => stale.resolve(publicList));

  expect(container.querySelector('img[src="/thumbs/image.png"]')).toBeNull();
  expect(files).toHaveBeenCalledTimes(2);

  await act(async () => fresh.resolve(hiddenList));

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
