import type { DirectoryPerson } from "../gen/DirectoryPerson.ts";
import type { PeopleDirectory } from "../gen/PeopleDirectory.ts";
import type { PersonProfile } from "../gen/PersonProfile.ts";
import type { Settings } from "../gen/Settings.ts";
import type { User } from "../gen/User.ts";
import { mutations, store } from "../store/store.ts";

/** The requests the people pages make, as promises (the runtime's in the app, fakes in tests). */
export interface PeopleRequests {
  readonly directory: () => Promise<PeopleDirectory>;
  readonly profile: (userId: number) => Promise<PersonProfile>;
  readonly setBanned: (userId: number, banned: boolean) => Promise<PersonProfile>;
  readonly setDndAllowance: (userId: number, allowed: boolean) => Promise<Settings>;
}

/** A person's DND writes: how many are in flight, and the number of the latest to start. */
interface DndWrites {
  readonly inFlight: number;
  readonly latest: number;
}

/**
 * The people directory and a person's page: plain promises over the S7 people endpoints, each
 * reply's users landed in the store first. The store keeps whichever copy of a user has the later
 * `updatedAt`, so replies may land in any order; a person's page whose user a later copy overtook
 * is fetched again, which ends because the server's `updatedAt` only moves forward. Their DND
 * allowance lives in the store too, where its own writes win over reads they overlap. Banning
 * fails as `admin`'s writes do.
 */
export function peoplePagesOver(requests: PeopleRequests) {
  const dndWrites = new Map<number, DndWrites>();
  let dndWritesStarted = 0;

  /** Where someone's DND writes stand as a read starts. */
  function dndMark(userId: number): number {
    return dndWrites.get(userId)?.latest ?? 0;
  }

  /**
   * Lands the DND allowance a read brought, unless a DND write on them is in flight or started
   * since the read did: the write's reply is newer, and lands itself.
   */
  function landReadAllowance(userId: number, mark: number, allowed: boolean | null): void {
    const writes = dndWrites.get(userId);

    if (
      allowed === null ||
      (writes !== undefined && (writes.inFlight > 0 || writes.latest !== mark))
    ) {
      return;
    }

    mutations.setDndAllowance(userId, allowed);
  }

  /** Whether the store holds a later copy of this user than `user`. */
  function overtaken(user: User): boolean {
    const held = store.getState().users[user.id];

    return held !== undefined && held.updatedAt > user.updatedAt;
  }

  const pages = {
    directory: (): Promise<readonly DirectoryPerson[]> =>
      requests.directory().then((list) => {
        mutations.mergeUsers(list.users);

        return list.people;
      }),

    profile: async (userId: number): Promise<PersonProfile> => {
      for (;;) {
        const mark = dndMark(userId);
        const profile = await requests.profile(userId);

        mutations.mergeUsers([profile.user]);

        if (!overtaken(profile.user)) {
          landReadAllowance(userId, mark, profile.dndAllowed);

          return profile;
        }
      }
    },

    /** Bans them (`true`) or removes the ban (`false`); answers their page as it now stands. */
    setBanned: async (userId: number, banned: boolean): Promise<PersonProfile> => {
      const mark = dndMark(userId);
      const profile = await requests.setBanned(userId, banned);

      mutations.mergeUsers([profile.user]);

      if (overtaken(profile.user)) {
        return pages.profile(userId);
      }

      landReadAllowance(userId, mark, profile.dndAllowed);

      return profile;
    },

    /**
     * Lets them through your Do Not Disturb (`true`) or not. The answer lands in the store if no
     * later change to their allowance started meanwhile, wherever the page that asked has gone.
     */
    setDndAllowance: (userId: number, allowed: boolean): Promise<Settings> => {
      dndWritesStarted += 1;

      const number = dndWritesStarted;
      const before = dndWrites.get(userId);

      dndWrites.set(userId, { inFlight: (before?.inFlight ?? 0) + 1, latest: number });

      return requests
        .setDndAllowance(userId, allowed)
        .then((settings) => {
          if (dndWrites.get(userId)?.latest === number) mutations.setDndAllowance(userId, allowed);

          return settings;
        })
        .finally(() => {
          const writes = dndWrites.get(userId);

          if (writes !== undefined) {
            dndWrites.set(userId, { ...writes, inFlight: writes.inFlight - 1 });
          }
        });
    },
  };

  return pages;
}
