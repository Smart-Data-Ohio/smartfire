import { afterEach, describe, expect, it } from "vitest";
import { userFixture } from "../api/testing.ts";
import type { PersonProfile } from "../gen/PersonProfile.ts";
import type { Settings } from "../gen/Settings.ts";
import type { User } from "../gen/User.ts";
import { nextObservation, observeResponse } from "../lib/request-observation.ts";
import { mutations, store } from "../store/store.ts";
import { type PeopleRequests, PROFILE_OUT_OF_DATE, peoplePagesOver } from "./people-pages.ts";

const SAM = 5;

/** Sam as the server had him at `minute` past ten. */
function sam(minute: number, status: User["status"] = "active"): User {
  const at = `2026-10-07T10:${String(minute).padStart(2, "0")}:00.000000Z`;

  return { ...userFixture(SAM, "Sam Whitfield"), status, updatedAt: at };
}

/** His page as the server answers it with `user`. */
function page(user: User, dndAllowed: boolean | null = false): PersonProfile {
  const active = user.status === "active";

  return {
    user,
    status: { presence: active ? "online" : "offline", statusText: null },
    dndAllowed: active ? dndAllowed : null,
    emailAddress: null,
    transferUrl: active ? "https://chat.example/session/transfers/t" : null,
    transferQrSvg: active ? "<svg/>" : null,
    canBan: true,
  };
}

/** A reply the test hands out when it chooses. */
interface Held<A> {
  readonly promise: Promise<A>;
  readonly release: (value: A) => void;
}

function held<A>(): Held<A> {
  let release: (value: A) => void = () => undefined;

  const promise = new Promise<A>((resolve) => {
    release = resolve;
  });

  return { promise, release };
}

/** Requests that answer from queues the test fills, one reply per call. */
function fakeRequests() {
  const profiles: Promise<PersonProfile>[] = [];
  const bans: Promise<PersonProfile>[] = [];
  const allowances: Promise<Settings>[] = [];
  let profileCalls = 0;

  const next = <A>(queue: Promise<A>[]): Promise<A> => {
    const reply = queue.shift();

    if (reply === undefined) throw new Error("no reply queued");

    return reply;
  };

  const requests: PeopleRequests = {
    directory: () => Promise.reject(new Error("unused")),
    profile: () => {
      profileCalls += 1;

      return next(profiles);
    },
    setBanned: () => next(bans),
    setDndAllowance: () => next(allowances),
  };

  return { requests, profiles, bans, allowances, profileCalls: () => profileCalls };
}

// SAFETY: the people pages never read a DND reply's body, only when it arrives.
const SETTINGS = {} as Settings;

describe("the people pages", () => {
  afterEach(() => mutations.reset());

  it("answers the canonical presentation after a delayed profile at the same user revision", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);
    const stale = held<PersonProfile>();

    const captured = page({
      ...sam(1),
      customStatus: { emoji: "🌴", text: "Away", expiresAt: null },
    });

    observeResponse(captured, nextObservation());
    fake.profiles.push(stale.promise);

    const loading = pages.profile(SAM);

    mutations.mergeUsers([sam(1)]);
    stale.release(captured);

    const profile = await loading;

    expect(profile.user.customStatus).toBeNull();
    expect(profile.user).toBe(store.getState().users[SAM]);
  });

  it("fetch a held banned page again once a later unban reached the store", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);
    const stale = held<PersonProfile>();

    fake.profiles.push(stale.promise, Promise.resolve(page(sam(3))));

    const loading = pages.profile(SAM);

    // A resync lands the unban (10:03) while the page's request still holds the ban (10:02).
    mutations.mergeUsers([sam(3)]);
    stale.release(page(sam(2, "banned")));

    const profile = await loading;

    expect(profile.user.status).toBe("active");
    expect(profile.transferUrl).not.toBeNull();
    expect(fake.profileCalls()).toBe(2);
    expect(store.getState().users[SAM]?.status).toBe("active");
  });

  it("keep an unban over a held banned page at the same user revision", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);
    const stale = held<PersonProfile>();

    // A repeated server clock stamps the ban and its unban with one revision, 10:02.
    mutations.mergeUsers([sam(2, "banned")]);

    const staleBanned = page(sam(2, "banned"));
    const unbanned = page(sam(2));
    const fresh = page(sam(2));

    observeResponse(staleBanned, nextObservation());
    fake.profiles.push(stale.promise);

    const loading = pages.profile(SAM);

    observeResponse(unbanned, nextObservation());
    fake.bans.push(Promise.resolve(unbanned));
    expect((await pages.setBanned(SAM, false)).user.status).toBe("active");

    observeResponse(fresh, nextObservation());
    fake.profiles.push(Promise.resolve(fresh));
    stale.release(staleBanned);

    const profile = await loading;

    expect(profile.user.status).toBe("active");
    expect(profile.transferUrl).not.toBeNull();
    expect(fake.profileCalls()).toBe(2);
    expect(store.getState().users[SAM]?.status).toBe("active");
  });

  it("fetch the page again when a ban's reply is older than the store's copy", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);

    mutations.mergeUsers([sam(4)]);
    fake.bans.push(Promise.resolve(page(sam(2, "banned"))));
    fake.profiles.push(Promise.resolve(page(sam(4))));

    expect((await pages.setBanned(SAM, true)).user.updatedAt).toBe(sam(4).updatedAt);
    expect(fake.profileCalls()).toBe(1);
  });

  it("land a DND change that finishes after its page has gone, over a read it overlapped", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);
    const write = held<Settings>();

    fake.allowances.push(write.promise);

    const saving = pages.setDndAllowance(SAM, true);

    // You leave and come back: the new page's load reads the old allowance mid-write.
    fake.profiles.push(Promise.resolve(page(sam(1), false)));
    await pages.profile(SAM);
    expect(store.getState().dndAllowances[SAM]).toBeUndefined();

    write.release(SETTINGS);
    await saving;
    expect(store.getState().dndAllowances[SAM]).toBe(true);
  });

  it("keep a DND change over a read that started before it and answers after it", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);
    const read = held<PersonProfile>();

    fake.profiles.push(read.promise);

    const loading = pages.profile(SAM);

    fake.allowances.push(Promise.resolve(SETTINGS));
    await pages.setDndAllowance(SAM, true);
    read.release(page(sam(1), false));
    await loading;

    expect(store.getState().dndAllowances[SAM]).toBe(true);
  });

  it("keep a DND change over a read that started while it was in flight and answers after it", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);
    const write = held<Settings>();
    const read = held<PersonProfile>();

    fake.allowances.push(write.promise);
    fake.profiles.push(read.promise);

    const saving = pages.setDndAllowance(SAM, true);
    const loading = pages.profile(SAM);

    write.release(SETTINGS);
    await saving;
    read.release(page(sam(1), false));
    await loading;

    expect(store.getState().dndAllowances[SAM]).toBe(true);
  });

  it("give up after three refetches when every reply is older, keeping the newer user", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);

    mutations.mergeUsers([sam(9)]);

    for (let reply = 0; reply < 4; reply += 1) {
      fake.profiles.push(Promise.resolve(page(sam(2, "banned"))));
    }

    await expect(pages.profile(SAM)).rejects.toThrow(PROFILE_OUT_OF_DATE);
    expect(fake.profileCalls()).toBe(4);
    expect(store.getState().users[SAM]?.updatedAt).toBe(sam(9).updatedAt);
  });

  it("stop fetching again once the page has gone", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);
    const controller = new AbortController();

    mutations.mergeUsers([sam(9)]);
    fake.profiles.push(Promise.resolve(page(sam(2))));
    controller.abort();

    await expect(pages.profile(SAM, controller.signal)).rejects.toThrow();
    expect(fake.profileCalls()).toBe(1);
  });

  it("let the latest of two DND changes win, whichever answers last", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);
    const first = held<Settings>();

    fake.allowances.push(first.promise, Promise.resolve(SETTINGS));

    const allowing = pages.setDndAllowance(SAM, true);

    await pages.setDndAllowance(SAM, false);
    first.release(SETTINGS);
    await allowing;

    expect(store.getState().dndAllowances[SAM]).toBe(false);
  });

  it("land a read's allowance when no change to it is under way", async () => {
    const fake = fakeRequests();
    const pages = peoplePagesOver(fake.requests);

    fake.profiles.push(Promise.resolve(page(sam(1), true)));
    await pages.profile(SAM);

    expect(store.getState().dndAllowances[SAM]).toBe(true);
  });
});
