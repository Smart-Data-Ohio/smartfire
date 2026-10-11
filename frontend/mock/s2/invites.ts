/**
 * Private workspace invites (slices 49 and 50): `/admin/invites`, as `api::invites` serves them,
 * kept per world so `reset()` starts them over. An invite made here works on the invite page (the
 * join module asks `refusal` and `use`), so a test can make, copy, open and revoke one end to end.
 *
 * The world starts with one invite in each state, none of whose tokens is known: an active one of
 * Riel's (3 of 10 uses), Priya's used up one, an expired one and a revoked one.
 */
import type { InviteRefusal } from "../../src/gen/InviteRefusal.ts";
import type { WorkspaceInvite } from "../../src/gen/WorkspaceInvite.ts";
import type { WorkspaceInviteCreated } from "../../src/gen/WorkspaceInviteCreated.ts";
import type { WorkspaceInviteExpiry } from "../../src/gen/WorkspaceInviteExpiry.ts";
import type { WorkspaceInviteState } from "../../src/gen/WorkspaceInviteState.ts";
import { notFound, ok, validation } from "../http.ts";
import { field, intField, stringField } from "../json.ts";
import { rowTimestamp, timestamp, USER_IDS, VIEWER_ID, type World } from "../seed.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

const DAY = 24 * HOUR;

/** `InviteExpiry::seconds`, in ms; `null` never expires. */
const EXPIRY_MS: Readonly<Record<WorkspaceInviteExpiry, number | null>> = {
  "30m": 30 * MINUTE,
  "1h": HOUR,
  "6h": 6 * HOUR,
  "12h": 12 * HOUR,
  "1d": DAY,
  "7d": 7 * DAY,
  never: null,
};

/** `WorkspaceInvite::create`'s allowed limits. */
const LIMITS = new Set([1, 5, 10, 25, 50, 100]);

/** An invite as the server keeps it: no state (worked out when asked) and the token beside it. */
interface Held {
  readonly id: number;
  readonly token: string;
  readonly creatorId: number;
  readonly createdAt: number;
  readonly expiresAt: number | null;
  readonly maxUses: number | null;
  uses: number;
  revokedAt: number | null;
  updatedAt: number;
}

interface State {
  invites: Held[];
  nextId: number;
}

/** The invites module. */
export interface InvitesModule {
  readonly routes: readonly Route[];
  /**
   * Why the invite with `token` can't enroll anyone, `null` when it can, or `undefined` when no
   * invite made here has that token.
   */
  refusal(token: string): InviteRefusal | null | undefined;
  /** Takes one use of the invite with `token`, as an enrollment does. */
  use(token: string): void;
}

/** `WorkspaceInvite::state`: revoked, then expired, then used up. */
function stateOf(invite: Held, now: number): WorkspaceInviteState {
  if (invite.revokedAt !== null) return "revoked";

  if (invite.expiresAt !== null && invite.expiresAt <= now) return "expired";

  if (invite.maxUses !== null && invite.uses >= invite.maxUses) return "exhausted";

  return "active";
}

/** Creates the invites module. */
export function createInvites(ctx: S2Context): InvitesModule {
  let state: State | null = null;
  let stateWorld: World | null = null;

  const seeded = (now: number): Held[] => {
    const held = (
      id: number,
      creatorId: number,
      createdAgo: number,
      lasts: number | null,
      maxUses: number | null,
      uses: number,
      revokedAgo: number | null = null,
    ): Held => ({
      id,
      token: `mock-seeded-invite-${id}`,
      creatorId,
      createdAt: now - createdAgo,
      expiresAt: lasts === null ? null : now - createdAgo + lasts,
      maxUses,
      uses,
      revokedAt: revokedAgo === null ? null : now - revokedAgo,
      updatedAt: now - (revokedAgo ?? createdAgo),
    });

    // Newest first, as the list answers.
    return [
      held(4, VIEWER_ID, 2 * HOUR, 7 * DAY, 10, 3),
      held(3, USER_IDS.priya, 3 * DAY, null, 5, 5),
      held(2, VIEWER_ID, 9 * DAY, 7 * DAY, null, 2),
      held(1, USER_IDS.priya, 12 * DAY, null, 25, 1, 11 * DAY),
    ];
  };

  const current = (): State => {
    const world = ctx.world();

    if (state === null || stateWorld !== world) {
      state = { invites: seeded(ctx.now()), nextId: 5 };
      stateWorld = world;
    }

    return state;
  };

  const dto = (invite: Held): WorkspaceInvite => {
    const creator = ctx.world().users.get(invite.creatorId);

    return {
      id: invite.id,
      creator: { id: invite.creatorId, name: creator?.name ?? "Someone" },
      expiresAt: invite.expiresAt === null ? null : timestamp(invite.expiresAt),
      maxUses: invite.maxUses,
      uses: invite.uses,
      revokedAt: invite.revokedAt === null ? null : timestamp(invite.revokedAt),
      createdAt: timestamp(invite.createdAt),
      updatedAt: rowTimestamp(invite.updatedAt),
      state: stateOf(invite, ctx.now()),
    };
  };

  const byToken = (token: string) => current().invites.find((invite) => invite.token === token);

  return {
    refusal: (token) => {
      const invite = byToken(token);

      if (invite === undefined) return undefined;

      const now = stateOf(invite, ctx.now());

      return now === "active" ? null : now;
    },
    use: (token) => {
      const invite = byToken(token);

      if (invite !== undefined) {
        invite.uses += 1;
        invite.updatedAt = ctx.now();
      }
    },
    routes: [
      route("GET", /^\/admin\/invites$/, () => ok({ invites: current().invites.map(dto) })),
      route("POST", /^\/admin\/invites$/, ({ body }) => {
        const expiry = stringField(body, "expiry");
        const maxUses = field(body, "maxUses") === null ? null : intField(body, "maxUses");

        const lasts = Object.entries(EXPIRY_MS).find(([choice]) => choice === expiry)?.[1];

        if (lasts === undefined) {
          throw validation("expiry", "is not a known expiry");
        }

        if (maxUses !== null && !LIMITS.has(maxUses)) {
          throw validation("max_uses", "must be 1, 5, 10, 25, 50, 100 or unlimited");
        }

        const held = current();
        const now = ctx.now();
        const token = ctx.hex(64);

        const invite: Held = {
          id: held.nextId,
          token,
          creatorId: VIEWER_ID,
          createdAt: now,
          expiresAt: lasts === null ? null : now + lasts,
          maxUses,
          uses: 0,
          revokedAt: null,
          updatedAt: now,
        };

        held.nextId += 1;
        held.invites.unshift(invite);

        const created: WorkspaceInviteCreated = {
          invite: dto(invite),
          token,
          url: `http://127.0.0.1/invite/${token}`,
        };

        return ok(created, 201);
      }),
      route("DELETE", /^\/admin\/invites\/(\d+)$/, (request) => {
        const id = firstId(request);
        const invite = current().invites.find((each) => each.id === id);

        if (invite === undefined) throw notFound();

        // Revoking twice keeps the first time, as `WorkspaceInvite::revoke` does.
        if (invite.revokedAt === null) {
          invite.revokedAt = ctx.now();
          invite.updatedAt = ctx.now();
        }

        return ok(dto(invite));
      }),
    ],
  };
}
