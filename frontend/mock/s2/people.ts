/**
 * The people directory and a person's page (S7), `users#index` and `users#show` as JSON. The
 * directory and the page read the world's users, presence, stars and the viewer's DND exceptions
 * (the settings mock's set); banning flips a user's status. The viewer is
 * an administrator unless the admin mock made them a member.
 */
import type { DirectoryPerson } from "../../src/gen/DirectoryPerson.ts";
import type { PeopleDirectory } from "../../src/gen/PeopleDirectory.ts";
import type { PersonProfile } from "../../src/gen/PersonProfile.ts";
import type { User } from "../../src/gen/User.ts";
import { forbidden, notFound, ok } from "../http.ts";
import { VIEWER_ID } from "../seed.ts";
import { MOCK_TRANSFER_QR_SVG } from "./account.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";

/** The people module. */
export interface PeopleModule {
  readonly routes: readonly Route[];
}

/** The sign-in link the mock shows on someone else's page. */
export function mockPersonTransferUrl(userId: number): string {
  return `http://localhost/session/transfers/mock-person-${userId}`;
}

/** The mock's email for a person, as the admin mock spells it. */
function emailOf(user: User): string {
  return `${user.name.split(" ")[0]?.toLowerCase() ?? `user${user.id}`}@smartdata.example`;
}

/** Creates the people module; `requireSudo` is the admin mock's password check. */
export function createPeople(ctx: S2Context, requireSudo: () => void): PeopleModule {
  const viewerIsAdmin = () => ctx.world().users.get(VIEWER_ID)?.role === "administrator";

  const byName = (a: User, b: User) => {
    const [left, right] = [a.name.toLowerCase(), b.name.toLowerCase()];

    return left < right ? -1 : left > right ? 1 : a.id - b.id;
  };

  const directory = (): PeopleDirectory => {
    const world = ctx.world();

    const users = [...world.users.values()]
      .filter((user) => user.status === "active" && user.id !== VIEWER_ID)
      .sort(
        (a, b) => Number(world.stars.has(b.id)) - Number(world.stars.has(a.id)) || byName(a, b),
      );

    const people = users.map(
      (user): DirectoryPerson => ({
        userId: user.id,
        // A live lease counts as online in the classic directory, whatever the presence says.
        online:
          user.role === "bot"
            ? user.agent !== null && !user.agent.suspended
            : (world.presence.get(user.id)?.presence ?? "offline") !== "offline",
        starred: world.stars.has(user.id),
        agent: user.agent !== null,
      }),
    );

    return { people, users };
  };

  const profile = (userId: number): PersonProfile => {
    const world = ctx.world();
    const user = world.users.get(userId);

    if (user === undefined) throw notFound("User not found");

    const admin = viewerIsAdmin();
    const person = user.role !== "bot" && user.status !== "deactivated";
    const active = person && user.status === "active";
    const other = userId !== VIEWER_ID;
    const presence = world.presence.get(userId);
    const transferUrl = admin && active ? mockPersonTransferUrl(userId) : null;

    return {
      user,
      status: person
        ? {
            presence: user.status === "active" ? (presence?.presence ?? "offline") : "offline",
            statusText: presence?.statusText ?? null,
          }
        : null,
      dndAllowed: active && other ? world.dndAllowed.has(userId) : null,
      emailAddress: admin && person ? emailOf(user) : null,
      transferUrl,
      transferQrSvg: transferUrl === null ? null : MOCK_TRANSFER_QR_SVG,
      canBan: admin && person && other,
    };
  };

  /** `users/bans#create` / `#destroy`: administrator, then the user, then the password. */
  const setBanned = (userId: number, banned: boolean) => {
    if (!viewerIsAdmin()) throw forbidden("Not allowed");

    const world = ctx.world();
    const user = world.users.get(userId);

    if (user === undefined) throw notFound("User not found");

    requireSudo();
    world.users.set(userId, { ...user, status: banned ? "banned" : "active" });

    return ok(profile(userId));
  };

  return {
    routes: [
      route("GET", /^\/people$/, () => ok(directory())),
      route("GET", /^\/people\/(\d+)$/, (request) => ok(profile(firstId(request)))),
      route("POST", /^\/people\/(\d+)\/ban$/, (request) => setBanned(firstId(request), true)),
      route("DELETE", /^\/people\/(\d+)\/ban$/, (request) => setBanned(firstId(request), false)),
    ],
  };
}
