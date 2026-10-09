import { describe, expect, it } from "vitest";
import type { RoomAccess } from "./message-destination.ts";
import { followPermalink } from "./permalink-follow.ts";

const ROOM = 8;

const MESSAGE = 4;

const THREAD = "/app/r/8/t/9?m=4";

interface PermalinkMessage {
  readonly id: number;
  readonly roomId: number;
  readonly threadId: number | null;
}

interface Deferred<T> {
  readonly promise: Promise<T>;
  resolve: (value: T) => void;
  reject: () => void;
}

interface PendingRead {
  readonly issued: RoomAccess | null;
  readonly result: Deferred<{ readonly message: PermalinkMessage }>;
}

type FollowEvent =
  | { readonly kind: "focus"; readonly messageId: number }
  | { readonly kind: "redirect"; readonly href: string }
  | { readonly kind: "drop" };

interface AccessSource {
  now: () => RoomAccess | null;
  detailReady: () => boolean;
  subscribe: (onChange: () => void) => () => void;
  listening: () => number;
  become: (next: RoomAccess | null) => void;
  join: () => void;
}

interface Harness {
  readonly reads: PendingRead[];
  readonly events: FollowEvent[];
  readonly access: AccessSource;
  readonly cancel: () => void;
}

function deferred<T>(): Deferred<T> {
  let resolvePromise: (value: T) => void = () => {};

  let rejectPromise: () => void = () => {};

  const promise = new Promise<T>((resolve, reject) => {
    resolvePromise = resolve;

    rejectPromise = () => {
      reject(new Error("missing"));
    };
  });

  return {
    promise,
    resolve: (value) => {
      resolvePromise(value);
    },
    reject: () => {
      rejectPromise();
    },
  };
}

/** Membership plus the room-detail flag, notifying once per change. */
function accessSource(initial: RoomAccess | null): AccessSource {
  let role: RoomAccess | null = initial;
  let detail = false;
  const listeners = new Set<() => void>();

  const notify = (): void => {
    listeners.forEach((listener) => {
      listener();
    });
  };

  return {
    now: () => role,
    detailReady: () => detail,
    subscribe: (onChange) => {
      listeners.add(onChange);

      return () => {
        listeners.delete(onChange);
      };
    },
    listening: () => listeners.size,
    become: (next) => {
      role = next;
      notify();
    },
    join: () => {
      role = "member";
      detail = true;
      notify();
    },
  };
}

function message(threadId: number | null): PermalinkMessage {
  return { id: MESSAGE, roomId: ROOM, threadId };
}

function follow(initial: RoomAccess | null): Harness {
  const reads: PendingRead[] = [];
  const events: FollowEvent[] = [];
  const access = accessSource(initial);

  const cancel = followPermalink({
    roomId: ROOM,
    messageId: MESSAGE,
    read: () => {
      const result = deferred<{ readonly message: PermalinkMessage }>();

      reads.push({ issued: access.now(), result });

      return result.promise;
    },
    access: () => access.now(),
    joined: () => access.detailReady(),
    subscribe: (onChange) => access.subscribe(onChange),
    onFocus: (messageId) => {
      events.push({ kind: "focus", messageId });
    },
    onRedirect: (href) => {
      events.push({ kind: "redirect", href });
    },
    onDrop: () => {
      events.push({ kind: "drop" });
    },
  });

  return { reads, events, access, cancel };
}

function turn(): Promise<void> {
  return Promise.resolve();
}

describe("followPermalink", () => {
  it("keeps the anchor and re-reads once when join beats the pre-join 404", async () => {
    const harness = follow("unjoined");

    expect(harness.reads.map((entry) => entry.issued)).toEqual(["unjoined"]);

    harness.access.join();
    harness.reads[0]?.result.reject();
    await turn();

    expect(harness.reads.map((entry) => entry.issued)).toEqual(["unjoined", "member"]);
    expect(harness.events).toEqual([]);

    harness.reads[1]?.result.resolve({ message: message(null) });
    await turn();

    expect(harness.events).toEqual([{ kind: "focus", messageId: MESSAGE }]);
    expect(harness.reads).toHaveLength(2);
  });

  it("holds an unjoined miss, then reads once as a member without focusing again", async () => {
    const harness = follow("unjoined");

    harness.reads[0]?.result.reject();
    await turn();

    expect(harness.events).toEqual([{ kind: "focus", messageId: MESSAGE }]);
    expect(harness.access.listening()).toBe(1);

    harness.access.become("member");
    expect(harness.reads).toHaveLength(1);

    harness.access.join();
    expect(harness.reads.map((entry) => entry.issued)).toEqual(["unjoined", "member"]);

    harness.reads[1]?.result.resolve({ message: message(null) });
    await turn();

    expect(harness.events).toEqual([{ kind: "focus", messageId: MESSAGE }]);
    expect(harness.access.listening()).toBe(0);
  });

  it("drops the anchor when the member re-read is also missing, and does not read again", async () => {
    const harness = follow("unjoined");

    harness.reads[0]?.result.reject();
    await turn();
    harness.access.join();

    harness.reads[1]?.result.reject();
    await turn();

    expect(harness.events).toEqual([{ kind: "focus", messageId: MESSAGE }, { kind: "drop" }]);
    expect(harness.reads.map((entry) => entry.issued)).toEqual(["unjoined", "member"]);

    harness.access.join();
    expect(harness.reads).toHaveLength(2);
  });

  it("drops a member's missing message and does not read again", async () => {
    const harness = follow("member");

    expect(harness.reads.map((entry) => entry.issued)).toEqual(["member"]);

    harness.reads[0]?.result.reject();
    await turn();

    expect(harness.events).toEqual([{ kind: "drop" }]);
    expect(harness.reads).toHaveLength(1);

    harness.access.join();
    expect(harness.reads).toHaveLength(1);
    expect(harness.access.listening()).toBe(0);
  });

  it("redirects a reply to its thread", async () => {
    const harness = follow("member");

    harness.reads[0]?.result.resolve({ message: message(9) });
    await turn();

    expect(harness.events).toEqual([{ kind: "redirect", href: THREAD }]);
    expect(harness.reads).toHaveLength(1);
  });

  it("re-reads once when null access becomes member before the first failure", async () => {
    const harness = follow(null);

    expect(harness.reads.map((entry) => entry.issued)).toEqual([null]);

    harness.access.become("member");
    harness.reads[0]?.result.reject();
    await turn();

    expect(harness.reads.map((entry) => entry.issued)).toEqual([null, "member"]);
    expect(harness.events).toEqual([]);

    harness.reads[1]?.result.resolve({ message: message(null) });
    await turn();

    expect(harness.events).toEqual([{ kind: "focus", messageId: MESSAGE }]);
    expect(harness.reads).toHaveLength(2);
  });

  it("re-reads once when null access becomes member after the first failure", async () => {
    const harness = follow(null);

    harness.reads[0]?.result.reject();
    await turn();

    expect(harness.reads).toHaveLength(1);
    expect(harness.events).toEqual([]);
    expect(harness.access.listening()).toBe(1);

    harness.access.become("member");

    expect(harness.reads.map((entry) => entry.issued)).toEqual([null, "member"]);
    expect(harness.events).toEqual([]);
    expect(harness.access.listening()).toBe(0);

    harness.reads[1]?.result.reject();
    await turn();

    expect(harness.events).toEqual([{ kind: "drop" }]);
    expect(harness.reads).toHaveLength(2);
  });

  it("ignores a late success after cancel", async () => {
    const harness = follow("member");

    harness.cancel();
    harness.reads[0]?.result.resolve({ message: message(null) });
    await turn();

    expect(harness.events).toEqual([]);
    expect(harness.reads).toHaveLength(1);
  });

  it("ignores a late failure after cancel, including one that would have been a member miss", async () => {
    const harness = follow("unjoined");

    harness.cancel();
    harness.access.join();
    harness.reads[0]?.result.reject();
    await turn();

    expect(harness.events).toEqual([]);
    expect(harness.reads).toHaveLength(1);
    expect(harness.access.listening()).toBe(0);
  });

  it("releases the preview watch so a later join does not read", async () => {
    const harness = follow("unjoined");

    harness.reads[0]?.result.reject();
    await turn();
    expect(harness.access.listening()).toBe(1);
    expect(harness.events).toEqual([{ kind: "focus", messageId: MESSAGE }]);

    harness.cancel();
    expect(harness.access.listening()).toBe(0);

    harness.access.join();
    harness.reads[0]?.result.resolve({ message: message(9) });
    await turn();

    expect(harness.reads).toHaveLength(1);
    expect(harness.events).toEqual([{ kind: "focus", messageId: MESSAGE }]);
  });

  it("releases the loading watch so a later membership does not read", async () => {
    const harness = follow(null);

    harness.reads[0]?.result.reject();
    await turn();
    expect(harness.access.listening()).toBe(1);

    harness.cancel();
    expect(harness.access.listening()).toBe(0);

    harness.access.become("member");
    harness.access.join();
    harness.reads[0]?.result.reject();
    await turn();

    expect(harness.reads).toHaveLength(1);
    expect(harness.events).toEqual([]);
  });
});
