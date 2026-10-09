import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { emptyTimeline, initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { REPLIES_FAILED, ThreadTimeline } from "./thread-timeline.tsx";

beforeEach(() => {
  // These failure states draw no list; jsdom still needs the anchor's observer API.
  vi.stubGlobal(
    "ResizeObserver",
    class implements ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

describe("the thread timeline", () => {
  it("says the replies failed, with Try again, once the header has loaded", async () => {
    store.setState({ threadTimelines: { 1: { ...emptyTimeline, status: "error" } } });

    const reload = vi.spyOn(actions.threads, "reload").mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(
      <ThreadTimeline
        threadId={1}
        parent={null}
        replyCount={2}
        ready
        focusMessageId={5}
        intro={<p>Post work</p>}
      />,
    );

    expect(screen.getByRole("alert").textContent).toContain(REPLIES_FAILED);
    expect(screen.getByText("Post work")).toBeDefined();

    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(reload).toHaveBeenCalledWith(1, 5);
  });

  it("keeps the skeleton while the header is still loading", () => {
    store.setState({ threadTimelines: { 1: { ...emptyTimeline, status: "error" } } });

    render(
      <ThreadTimeline
        threadId={1}
        parent={null}
        replyCount={2}
        ready={false}
        focusMessageId={null}
      />,
    );

    expect(screen.queryByRole("alert")).toBeNull();
  });
});
