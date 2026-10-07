/** The people pages' words and the selection bar's rules, as the classic directory has them. */
import type { PersonProfile } from "../../gen/PersonProfile.ts";
import type { Presence } from "../../gen/Presence.ts";
import type { User } from "../../gen/User.ts";
import type { PresenceStatus } from "../../ui/avatar.tsx";
import { MAX_OTHERS } from "../directs/picker.ts";

/** What the selection bar offers for the people chosen so far (`multi_select_controller#update`). */
export interface SelectionPlan {
  /** `Message (n)`: everyone chosen. */
  readonly messageLabel: string;
  /** `Start huddle (n)`: only the people, as bots aren't rung. */
  readonly huddleLabel: string;
  /** Over the group DM's size: neither button works. */
  readonly messageDisabled: boolean;
  /** Over the size, or nobody to ring. */
  readonly huddleDisabled: boolean;
  /** Why a button is off or what it leaves out; empty when there's nothing to say. */
  readonly note: string;
}

/**
 * The bar for `selected` ids, `bot` telling which are bots. A group DM holds the viewer and
 * `MAX_OTHERS` others; bots join the DM but never the huddle.
 */
export function selectionPlan(
  selected: readonly number[],
  bot: (userId: number) => boolean,
): SelectionPlan {
  const bots = selected.filter(bot).length;
  const humans = selected.length - bots;
  const overCap = selected.length > MAX_OTHERS;
  const reasons: string[] = [];

  if (overCap) {
    reasons.push(`Group DMs hold at most ${MAX_OTHERS + 1} people including you.`);
  }

  if (bots > 0) {
    reasons.push(
      humans === 0
        ? "Agents can't join huddles."
        : `${bots} ${bots === 1 ? "agent stays" : "agents stay"} in the DM but won't be rung.`,
    );
  }

  return {
    messageLabel: `Message (${selected.length})`,
    huddleLabel: `Start huddle (${humans})`,
    messageDisabled: overCap,
    huddleDisabled: overCap || humans === 0,
    note: reasons.join(" "),
  };
}

/**
 * `ids` with `userId` added or taken out. With `range`, everything between the last id touched
 * (`anchor`) and `userId` in `order` takes `userId`'s new state, as a Shift-click does.
 */
export function toggleSelection(
  ids: readonly number[],
  userId: number,
  order: readonly number[],
  anchor: number | null,
  range: boolean,
): readonly number[] {
  const on = !ids.includes(userId);
  const from = anchor === null ? -1 : order.indexOf(anchor);
  const to = order.indexOf(userId);

  const span =
    range && from !== -1 && to !== -1 && from !== to
      ? order.slice(Math.min(from, to), Math.max(from, to) + 1)
      : [userId];

  if (on) {
    return [...ids, ...span.filter((id) => !ids.includes(id))];
  }

  return ids.filter((id) => !span.includes(id));
}

/** The badge after a directory row's name: an agent, another bot, or none for a person. */
export function directoryBadge(user: User, agent: boolean): "Agent" | "Bot" | null {
  if (agent) {
    return "Agent";
  }

  return user.role === "bot" ? "Bot" : null;
}

/** The classic ban button's question (`users/_ban_button.html`). */
export const BAN_CONFIRMATION =
  "Are you sure you want to ban this user? This will log them out, delete their messages, and block their IP addresses.";

/** The classic remove-ban button's question. */
export const UNBAN_CONFIRMATION = "Are you sure you want to remove the ban on this user?";

/** The transfer link's warning on someone else's page (`users/profiles/_transfer.html`). */
export const TRANSFER_HINT = "Share to get them back into their account";

/** The transfer link's warning on your own page. */
export const OWN_TRANSFER_HINT = "Use this link to login automatically on another device";

/**
 * Where a bot's page lives: its agent profile when the SPA has one (`hasAgentPage`), else the
 * classic page, kept classic with `?classic=1` (a ported page would send it straight back here).
 */
export function botPage(user: User, hasAgentPage: boolean): string {
  if (user.agent !== null && hasAgentPage) {
    return `/app/agents/${user.agent.agentId}`;
  }

  return `/users/${user.id}?classic=1`;
}

/** A person page's presence word, in the words the rest of the app uses (idle reads "Away"). */
export const PRESENCE_LABEL = {
  online: "Online",
  idle: "Away",
  dnd: "Do not disturb",
  offline: "Offline",
} as const satisfies Record<Presence, string>;

const FROM_STATUS = {
  online: "online",
  away: "idle",
  dnd: "dnd",
  offline: "offline",
} as const satisfies Record<PresenceStatus, Presence>;

/** The live presence from the store when it's known, else what the page was loaded with. */
export function presenceOf(live: PresenceStatus | undefined, loaded: Presence): Presence {
  return live === undefined ? loaded : FROM_STATUS[live];
}

/**
 * A person's page after a ban or its removal lands: the status and what follows from it (their
 * presence, the sign-in link), from the server's reply. Their DND exception stays as the page has
 * it, since a DND change can land after the ban's reply was made; the reply's only fills it in
 * when the page had none (a page loaded while they were banned).
 */
export function landBan(current: PersonProfile, reply: PersonProfile): PersonProfile {
  return {
    ...current,
    user: { ...current.user, status: reply.user.status },
    status: reply.status,
    transferUrl: reply.transferUrl,
    transferQrSvg: reply.transferQrSvg,
    dndAllowed: current.dndAllowed ?? reply.dndAllowed,
  };
}
