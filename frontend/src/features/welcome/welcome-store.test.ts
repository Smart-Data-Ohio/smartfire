import { describe, expect, it } from "vitest";
import type { WorkspaceWelcome } from "../../gen/WorkspaceWelcome.ts";
import { createWelcomeStore, hasWelcomeCard } from "./welcome-store.ts";

const welcome = (message: string | null): WorkspaceWelcome => ({
  name: "Smart Data",
  description: "",
  messageHtml: message,
  rooms: [],
  guide: [],
  canAdminister: false,
});

interface Deferred {
  readonly resolve: (value: WorkspaceWelcome) => void;
  readonly reject: (error: Error) => void;
}

/** A source whose reads wait until the test answers them, and whose stale signal it fires. */
function source() {
  const reads: Deferred[] = [];
  const listeners: (() => void)[] = [];

  return {
    reads,
    stale: () => {
      for (const listener of listeners) listener();
    },
    read: () =>
      new Promise<WorkspaceWelcome>((resolve, reject) => {
        reads.push({ resolve, reject });
      }),
    onStale: (listener: () => void) => {
      listeners.push(listener);

      return () => undefined;
    },
  };
}

describe("the welcome store", () => {
  it("loads once asked, and again whenever the welcome may have changed", async () => {
    const fake = source();
    const { store, ensure } = createWelcomeStore(fake);

    expect(store.getState().status).toBe("idle");
    ensure();
    expect(store.getState().status).toBe("loading");
    fake.reads[0]?.resolve(welcome("<p>Hi</p>"));
    await Promise.resolve();
    expect(store.getState()).toEqual({ status: "ready", welcome: welcome("<p>Hi</p>") });

    fake.stale();
    // A refresh keeps the copy on screen until its answer arrives.
    expect(store.getState().status).toBe("ready");
    fake.reads[1]?.resolve(welcome(null));
    await Promise.resolve();
    expect(store.getState().welcome?.messageHtml).toBeNull();
  });

  it("follows changes once however often it's asked", () => {
    const fake = source();
    const { ensure } = createWelcomeStore(fake);

    ensure();
    ensure();
    fake.stale();
    expect(fake.reads).toHaveLength(3);
  });

  it("keeps the newest answer when reads overlap", async () => {
    const fake = source();
    const { store, refresh } = createWelcomeStore(fake);

    const first = refresh();
    const second = refresh();

    fake.reads[1]?.resolve(welcome("<p>new</p>"));
    fake.reads[0]?.resolve(welcome("<p>old</p>"));
    await Promise.all([first, second]);
    expect(store.getState().welcome?.messageHtml).toBe("<p>new</p>");
  });

  it("fails only while it has nothing to show", async () => {
    const fake = source();
    const { store, refresh } = createWelcomeStore(fake);

    const failing = refresh();

    fake.reads[0]?.reject(new Error("offline"));
    await failing;
    expect(store.getState().status).toBe("failed");

    const loading = refresh();

    fake.reads[1]?.resolve(welcome("<p>Hi</p>"));
    await loading;

    const again = refresh();

    fake.reads[2]?.reject(new Error("offline"));
    await again;
    expect(store.getState()).toEqual({ status: "ready", welcome: welcome("<p>Hi</p>") });
  });

  it("has a card for a message or a suggested room, not for a guide alone", () => {
    expect(hasWelcomeCard(null)).toBe(false);
    expect(hasWelcomeCard(welcome(null))).toBe(false);
    expect(hasWelcomeCard(welcome("<p>Hi</p>"))).toBe(true);
    expect(
      hasWelcomeCard({
        ...welcome(null),
        rooms: [{ roomId: 1, name: "general", emoji: null, description: "", joined: true }],
      }),
    ).toBe(true);
  });
});
