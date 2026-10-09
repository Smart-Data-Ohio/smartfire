import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { followSourceRow, useSourceFollower } from "./source-focus.ts";

/** A conversation: a pane (a container, tabIndex -1) holding message rows (tabIndex -1). */
function conversation() {
  const pane = document.createElement("aside");

  pane.tabIndex = -1;

  for (const id of [1, 2, 3]) {
    pane.append(row(id));
  }

  document.body.append(pane);

  return pane;
}

function row(id: number) {
  const element = document.createElement("div");

  element.tabIndex = -1;
  element.dataset.messageId = String(id);

  return element;
}

const find = (id: number) => () => document.querySelector<HTMLElement>(`[data-message-id="${id}"]`);

/** Lets the follower's observer (a microtask) and focus frame run. */
async function settle() {
  await Promise.resolve();
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
}

/** The thread pane reloading: it focuses itself and replaces the rows. */
async function reload(pane: HTMLElement) {
  pane.focus();
  pane.replaceChildren(row(1), row(2), row(3));
  await settle();
}

/** What the follower's owner passes: the dialog's opening, `null` while closed. */
interface OverlayProps {
  readonly opening: number | null;
}

const following: (() => void)[] = [];

afterEach(() => {
  for (const stop of following.splice(0)) {
    stop();
  }

  document.body.replaceChildren();
});

describe("following the source row", () => {
  it("returns focus to the row when the pane takes it and replaces the rows", async () => {
    const pane = conversation();
    const follower = followSourceRow(find(2));

    following.push(follower.stop);
    follower.row?.focus();
    await reload(pane);

    expect(document.activeElement).toBe(find(2)());
  });

  it("lets the viewer move to another row with the keyboard", async () => {
    const pane = conversation();
    const follower = followSourceRow(find(2));

    following.push(follower.stop);
    follower.row?.focus();
    document.activeElement?.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }),
    );
    find(3)()?.focus();
    await settle();

    expect(document.activeElement).toBe(find(3)());

    // A reload afterwards doesn't pull focus back either.
    find(3)()?.focus();
    pane.append(row(4));
    await settle();

    expect(document.activeElement).toBe(find(3)());
  });

  it("lets the viewer move to another row with the pointer", async () => {
    conversation();

    const follower = followSourceRow(find(2));

    following.push(follower.stop);
    follower.row?.focus();
    find(1)()?.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    find(1)()?.focus();
    await settle();

    expect(document.activeElement).toBe(find(1)());
  });

  it("does nothing once stopped", async () => {
    const pane = conversation();
    const follower = followSourceRow(find(2));

    follower.row?.focus();
    follower.stop();
    await reload(pane);

    expect(document.activeElement).toBe(pane);
  });

  it("stops when the dialog opens again", async () => {
    const pane = conversation();
    const closed: OverlayProps = { opening: null };

    const { result, rerender } = renderHook(
      ({ opening }: OverlayProps) => useSourceFollower(opening),
      { initialProps: closed },
    );

    result.current.follow(find(2))?.focus();
    rerender({ opening: 2 });
    await reload(pane);

    expect(document.activeElement).toBe(pane);
  });

  it("cancels the scheduled frame after a mutation, not a frame the mutation forgot", async () => {
    const scheduled: number[] = [];
    const cancelled: number[] = [];
    let next = 1;

    const request = vi.spyOn(window, "requestAnimationFrame").mockImplementation(() => {
      const id = next;

      next += 1;
      scheduled.push(id);

      return id;
    });

    const cancel = vi.spyOn(window, "cancelAnimationFrame").mockImplementation((id) => {
      cancelled.push(id);
    });

    try {
      conversation();

      const follower = followSourceRow(find(2));

      following.push(follower.stop);
      document.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
      document.body.append(document.createElement("span"));
      await Promise.resolve();
      await Promise.resolve();
      follower.stop();

      expect(scheduled.length).toBeGreaterThan(0);
      expect(cancelled).toEqual(scheduled);
    } finally {
      request.mockRestore();
      cancel.mockRestore();
    }
  });

  it("stops when the component that started it unmounts", async () => {
    const pane = conversation();
    const { result, unmount } = renderHook(() => useSourceFollower(null));

    result.current.follow(find(2))?.focus();
    unmount();
    await reload(pane);

    expect(document.activeElement).toBe(pane);
  });
});
