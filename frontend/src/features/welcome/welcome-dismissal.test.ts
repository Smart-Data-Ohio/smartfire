import { describe, expect, it } from "vitest";
import { createDismissal, type DismissalSource } from "./welcome-dismissal.ts";

/** A promise and the functions that settle it. */
function deferred() {
  let resolve: () => void = () => undefined;
  let reject: (error: Error) => void = () => undefined;

  const promise = new Promise<void>((settle, fail) => {
    resolve = settle;
    reject = fail;
  });

  return { promise, resolve, reject };
}

/** A preference that shows a saved change only when `show` is called, as a view transition does. */
function preference(initial: boolean) {
  let shown = initial;
  const listeners = new Set<() => void>();

  const saves: { dismissed: boolean; reply: ReturnType<typeof deferred> }[] = [];

  const source: DismissalSource = {
    save: (dismissed) => {
      const reply = deferred();

      saves.push({ dismissed, reply });

      return reply.promise;
    },
    saved: () => shown,
    onSavedChange: (listener) => {
      listeners.add(listener);

      return () => listeners.delete(listener);
    },
  };

  const show = (value: boolean) => {
    shown = value;

    for (const listener of listeners) listener();
  };

  return { source, saves, show, listeners };
}

describe("the welcome dismissal", () => {
  it("shows a change at once, before the save answers", async () => {
    const { source, saves, show } = preference(true);
    const dismissal = createDismissal(source);
    const done = dismissal.set(false);

    // Synchronously: a navigation right after the click already sees the card back.
    expect(dismissal.store.getState().pending).toBe(false);

    saves[0]?.reply.resolve();
    await done;

    // Saved, but the preference doesn't show it yet: the change holds until it does.
    expect(dismissal.store.getState().pending).toBe(false);
    show(false);
    expect(dismissal.store.getState().pending).toBeNull();
  });

  it("gives way at once when the preference already shows the saved change", async () => {
    const { source, saves, show, listeners } = preference(false);
    const dismissal = createDismissal(source);
    const done = dismissal.set(true);

    show(true);
    saves[0]?.reply.resolve();
    await done;

    expect(dismissal.store.getState().pending).toBeNull();
    expect(listeners.size).toBe(0);
  });

  it("takes the change back when the save is refused", async () => {
    const { source, saves } = preference(true);
    const dismissal = createDismissal(source);
    const done = dismissal.set(false);

    expect(dismissal.store.getState().pending).toBe(false);
    saves[0]?.reply.reject(new Error("offline"));

    await expect(done).rejects.toThrow("offline");
    expect(dismissal.store.getState().pending).toBeNull();
  });

  it("lets the newest change win when an older save settles late", async () => {
    const { source, saves, show } = preference(false);
    const dismissal = createDismissal(source);
    const first = dismissal.set(true);
    const second = dismissal.set(false);

    saves[0]?.reply.reject(new Error("late"));
    await expect(first).rejects.toThrow("late");
    expect(dismissal.store.getState().pending).toBe(false);

    saves[1]?.reply.resolve();
    await second;
    show(false);
    expect(dismissal.store.getState().pending).toBeNull();
  });
});
