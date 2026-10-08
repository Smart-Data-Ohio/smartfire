import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { type Loaded, useLoad } from "./use-load.ts";

/** Reads the test answers when it chooses, oldest first. */
class Reads {
  readonly pending: PromiseWithResolvers<string>[] = [];

  readonly read = () => {
    const next = Promise.withResolvers<string>();

    this.pending.push(next);

    return next.promise;
  };

  /** Answers read `index` (0 is the first) and lets the hook take it in. */
  async answer(index: number, value: string) {
    await act(async () => {
      this.pending[index]?.resolve(value);
    });
  }
}

const shownValue = (state: Loaded<string>) =>
  state.status === "ready" ? state.value : state.status;

describe("useLoad", () => {
  it("keeps the newer read when an older one lands after it", async () => {
    const reads = new Reads();
    const { result } = renderHook(() => useLoad("a", reads.read));

    act(() => result.current.reload());
    await reads.answer(1, "newer");
    await reads.answer(0, "older");

    expect(shownValue(result.current.state)).toBe("newer");
  });

  it("keeps a later answer's reply when an earlier answer's lands after it", async () => {
    const reads = new Reads();
    const { result } = renderHook(() => useLoad("a", reads.read));

    await reads.answer(0, "none");

    // Going, then Maybe: Maybe's reply comes back first, Going's (an older snapshot) last.
    const going = result.current.begin();
    const maybe = result.current.begin();

    act(() => maybe.land("maybe"));
    act(() => going.land("going"));

    expect(shownValue(result.current.state)).toBe("maybe");

    // The overtaken reply reads again, in case what overtook it was answered before it.
    expect(reads.pending).toHaveLength(2);
    await reads.answer(1, "maybe");
    expect(shownValue(result.current.state)).toBe("maybe");
  });

  it("drops a read that started before a write's reply landed", async () => {
    const reads = new Reads();
    const { result } = renderHook(() => useLoad("a", reads.read));

    await reads.answer(0, "first");
    act(() => result.current.reload());

    const write = result.current.begin();

    act(() => write.land("written"));
    await reads.answer(1, "read before the write");

    expect(shownValue(result.current.state)).toBe("written");
  });

  it("never lands a reply for a screen the viewer has left on the one they went to", async () => {
    const reads = new Reads();

    const { result, rerender } = renderHook(({ key }) => useLoad(key, reads.read), {
      initialProps: { key: "8101" },
    });

    await reads.answer(0, "8101");

    const answer = result.current.begin();

    rerender({ key: "8102" });
    await reads.answer(1, "8102");
    act(() => answer.land("8101 after answering"));
    act(() => answer.reread());

    expect(shownValue(result.current.state)).toBe("8102");
    expect(reads.pending).toHaveLength(2);
  });

  it("never holds up the shown screen's read for a save begun on a screen the viewer left", async () => {
    const reads = new Reads();

    const { result, rerender } = renderHook(({ key }) => useLoad(key, reads.read), {
      initialProps: { key: "8101" },
    });

    await reads.answer(0, "8101");

    // The save's callbacks were made on 8101's page; it finishes while 8102 is still loading.
    const beginOn8101 = result.current.begin;

    rerender({ key: "8102" });
    act(() => beginOn8101().land("8101 saved"));
    await reads.answer(1, "8102");

    expect(shownValue(result.current.state)).toBe("8102");
    expect(reads.pending).toHaveLength(2);
  });

  it("keeps a save's reply off newer answers when its turn was taken as it started", async () => {
    const reads = new Reads();
    const { result } = renderHook(() => useLoad("a", reads.read));

    await reads.answer(0, "before");

    // The save starts, the viewer answers while it's on its way, and the answer comes back first.
    const save = result.current.begin();
    const answer = result.current.begin();

    act(() => answer.land("answered"));
    act(() => save.land("saved, unanswered"));

    expect(shownValue(result.current.state)).toBe("answered");
    expect(reads.pending).toHaveLength(2);
  });

  it("reads once more after a burst of news rather than once per piece", async () => {
    const reads = new Reads();
    const { result } = renderHook(() => useLoad("a", reads.read));

    await reads.answer(0, "first");

    // With nothing on its way, news reads at once.
    act(() => result.current.refresh());
    expect(reads.pending).toHaveLength(2);

    // More news while that read is out: one more read once it lands, not one each.
    act(() => result.current.refresh());
    act(() => result.current.refresh());
    act(() => result.current.refresh());
    expect(reads.pending).toHaveLength(2);

    await reads.answer(1, "second");
    expect(reads.pending).toHaveLength(3);

    await reads.answer(2, "third");
    expect(reads.pending).toHaveLength(3);
    expect(shownValue(result.current.state)).toBe("third");
  });

  it("reads again after a write fails, keeping what it shows meanwhile", async () => {
    const reads = new Reads();
    const { result } = renderHook(() => useLoad("a", reads.read));

    await reads.answer(0, "before");
    act(() => result.current.begin().reread());

    expect(shownValue(result.current.state)).toBe("before");
    await reads.answer(1, "after");
    expect(shownValue(result.current.state)).toBe("after");
  });
});
