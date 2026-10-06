import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import { notePosted, useFollowPosted } from "./follow-posted.ts";
import type { TimelineItem } from "./timeline-items.ts";

function items(ids: readonly number[]): TimelineItem[] {
  return ids.map((id) => ({
    kind: "message",
    key: `c-${id}`,
    message: messageFixture(id, 12),
    groupStart: true,
  }));
}

interface Scroll {
  readonly index: number;
  readonly align: string | undefined;
}

interface ListStub {
  readonly scrollToIndex: (index: number, options?: { align?: string }) => void;
}

interface RecordingList {
  readonly scrolls: Scroll[];
  readonly ref: { readonly current: ListStub };
}

/** A list that only records where it was asked to scroll. */
function recordingList(): RecordingList {
  const scrolls: Scroll[] = [];

  return {
    scrolls,
    ref: {
      current: {
        scrollToIndex: (index, options) => scrolls.push({ index, align: options?.align }),
      },
    },
  };
}

describe("useFollowPosted", () => {
  it("goes to the message a command posted once it lands in the window, then forgets it", () => {
    const list = recordingList();

    const view = renderHook(
      ({ ids }: { readonly ids: readonly number[] }) =>
        useFollowPosted("room:12", items(ids), list.ref),
      { initialProps: { ids: [1, 2] } },
    );

    act(() => notePosted("room:12", 9));
    expect(list.scrolls).toEqual([]);

    view.rerender({ ids: [1, 2, 9] });
    expect(list.scrolls).toEqual([{ index: 2, align: "end" }]);

    // Forgotten: the window moving on doesn't pull the reader back to it.
    view.rerender({ ids: [2, 9, 10] });
    expect(list.scrolls).toEqual([{ index: 2, align: "end" }]);
  });

  it("ignores messages that arrive without a post from this composer", () => {
    const list = recordingList();

    const view = renderHook(
      ({ ids }: { readonly ids: readonly number[] }) =>
        useFollowPosted("room:12", items(ids), list.ref),
      { initialProps: { ids: [1, 2] } },
    );

    view.rerender({ ids: [1, 2, 3, 4] });
    expect(list.scrolls).toEqual([]);
  });

  it("leaves other conversations alone", () => {
    const list = recordingList();

    renderHook(() => useFollowPosted("thread:5", items([1, 2, 9]), list.ref));
    act(() => notePosted("room:12", 9));

    expect(list.scrolls).toEqual([]);
  });
});
