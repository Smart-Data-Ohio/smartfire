import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ChannelDirectory } from "../../gen/ChannelDirectory.ts";
import type { DirectoryChannel } from "../../gen/DirectoryChannel.ts";
import { actions } from "../../sync/runtime.ts";
import { useChannelDirectory } from "./channel-directory.ts";

function channel(roomId: number, name: string): DirectoryChannel {
  return {
    roomId,
    name,
    topic: null,
    memberCount: 2,
    lastActiveAt: "2026-10-19T12:00:00.000Z",
    joined: false,
  };
}

interface Sent {
  readonly query: string;
  readonly cursor: string | null;
  readonly answer: (page: ChannelDirectory) => void;
}

function holdBrowse(): Sent[] {
  const sent: Sent[] = [];

  vi.spyOn(actions, "browseChannels").mockImplementation(
    (query, cursor) =>
      new Promise((resolve) => {
        sent.push({ query, cursor, answer: resolve });
      }),
  );

  return sent;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useChannelDirectory", () => {
  it("never appends an older search's next page to a newer search", async () => {
    const sent = holdBrowse();

    const view = renderHook(({ query }) => useChannelDirectory(true, query), {
      initialProps: { query: "alpha" },
    });

    await waitFor(() => expect(sent).toHaveLength(1));
    await act(async () => sent[0]?.answer({ channels: [channel(1, "alpha-1")], nextCursor: "a1" }));
    expect(view.result.current.nextCursor).toBe("a1");

    view.rerender({ query: "beta" });
    act(() => view.result.current.loadMore?.());
    await waitFor(() => expect(sent.some((call) => call.query === "beta")).toBe(true));

    const search = sent.find((call) => call.query === "beta" && call.cursor === null);

    await act(async () => search?.answer({ channels: [channel(2, "beta-1")], nextCursor: null }));

    for (const call of sent.filter((each) => each.cursor !== null)) {
      await act(async () => call.answer({ channels: [channel(3, "alpha-2")], nextCursor: "a2" }));
    }

    expect(view.result.current.query).toBe("beta");
    expect(view.result.current.channels.map((each) => each.name)).toEqual(["beta-1"]);
    expect(view.result.current.nextCursor).toBeNull();
  });

  it("pages the search it answers with that search's cursor", async () => {
    const sent = holdBrowse();

    const view = renderHook(() => useChannelDirectory(true, "beta"));

    await waitFor(() => expect(sent).toHaveLength(1));
    await act(async () => sent[0]?.answer({ channels: [channel(2, "beta-1")], nextCursor: "b1" }));
    act(() => view.result.current.loadMore?.());

    expect(sent.map((call) => [call.query, call.cursor])).toEqual([
      ["beta", null],
      ["beta", "b1"],
    ]);

    await act(async () => sent[1]?.answer({ channels: [channel(4, "beta-2")], nextCursor: null }));
    expect(view.result.current.channels.map((each) => each.name)).toEqual(["beta-1", "beta-2"]);
  });
});
