import type { OAuthConsent } from "../../gen/OAuthConsent.ts";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import { abilityLabel } from "../settings/agents-format.ts";

/** Which agent the approval connects as: a new one, or one of the person's own. */
export type AgentPick =
  | { readonly kind: "new" }
  | { readonly kind: "existing"; readonly id: number };

/** What the approval form holds: the agent, a new one's name and icon, and its access. */
export interface ConsentForm {
  readonly pick: AgentPick;
  readonly name: string;
  readonly iconName: string | null;
  readonly roomIds: readonly number[];
  readonly capabilities: readonly string[];
}

/** Whether the person may add another agent here: on, and under the limit (admins have none). */
export function canCreateNew(consent: OAuthConsent): boolean {
  return consent.enabled && (consent.limit === null || consent.agents.length < consent.limit);
}

/**
 * The agent picked first: the one this app last connected as, else a new one when that's allowed,
 * else the first of the person's agents.
 */
export function firstPick(consent: OAuthConsent): AgentPick {
  const selected = consent.agents.find((agent) => agent.id === consent.selectedAgentId);

  if (selected !== undefined) return { kind: "existing", id: selected.id };

  const first = consent.agents[0];

  return canCreateNew(consent) || first === undefined
    ? { kind: "new" }
    : { kind: "existing", id: first.id };
}

/** The agent a pick names, or `null` for a new one (or one that's gone). */
export function pickedAgent(consent: OAuthConsent, pick: AgentPick): PersonalAgent | null {
  return pick.kind === "new"
    ? null
    : (consent.agents.find((agent) => agent.id === pick.id) ?? null);
}

/**
 * The form for a pick: an existing agent's current rooms and abilities (every ability while it has
 * no rooms, as Settings seeds it), or a new agent with no rooms and every ability.
 */
export function formFor(consent: OAuthConsent, pick: AgentPick, name = ""): ConsentForm {
  const agent = pickedAgent(consent, pick);

  if (agent === null) {
    return { pick, name, iconName: null, roomIds: [], capabilities: [...consent.capabilities] };
  }

  return {
    pick,
    name: agent.name,
    iconName: agent.iconName,
    roomIds: agent.roomIds,
    capabilities: agent.roomIds.length === 0 ? [...consent.capabilities] : agent.capabilities,
  };
}

/** Abilities only count while the agent has a room to use them in. */
function effective(roomIds: readonly number[], capabilities: readonly string[]): readonly string[] {
  return roomIds.length === 0 ? [] : capabilities;
}

/** How approving changes an existing agent's access in one room, in names. */
export interface RoomChange {
  readonly roomId: number;
  readonly room: string;
  readonly change: "joins" | "leaves" | "abilities";
  /** What it can newly do there: everything it'll have when it joins. */
  readonly gains: readonly string[];
  /** What it can no longer do there: everything it had when it leaves. */
  readonly loses: readonly string[];
}

/** How approving changes an existing agent's access, room by room. */
export type AccessDiff = readonly RoomChange[];

/** `capabilities` in the server's order (any it doesn't offer any more last), named for a sentence. */
function labels(consent: OAuthConsent, capabilities: readonly string[]): readonly string[] {
  const known = consent.capabilities.filter((capability) => capabilities.includes(capability));
  const other = capabilities.filter((capability) => !consent.capabilities.includes(capability));

  return [...known, ...other].map((capability) => abilityLabel(capability).toLowerCase());
}

/**
 * What approving `form` changes about `agent`'s access in each room. Approval gives every chosen
 * room every chosen ability and takes the agent out of every other room, so a room it only reads
 * in today gains posting when posting is chosen for another room.
 */
export function accessDiff(
  consent: OAuthConsent,
  agent: PersonalAgent,
  form: ConsentForm,
): AccessDiff {
  const now = new Map(
    consent.roomAccess
      .filter((access) => access.agentId === agent.id)
      .map((access) => [access.roomId, access]),
  );

  const after = effective(form.roomIds, form.capabilities);

  // The rooms in the order the page lists them, then any the person isn't in any more.
  const roomIds = [
    ...consent.rooms
      .map((room) => room.id)
      .filter((id) => now.has(id) || form.roomIds.includes(id)),
    ...[...now.keys()].filter((id) => !consent.rooms.some((room) => room.id === id)),
  ];

  return roomIds.flatMap((id): RoomChange[] => {
    const current = now.get(id);
    const chosen = form.roomIds.includes(id);

    const room =
      consent.rooms.find((offered) => offered.id === id)?.name ??
      (current?.roomName || "A room you're no longer in");

    if (current === undefined) {
      return chosen
        ? [{ roomId: id, room, change: "joins", gains: labels(consent, after), loses: [] }]
        : [];
    }

    if (!chosen) {
      return [
        {
          roomId: id,
          room,
          change: "leaves",
          gains: [],
          loses: labels(consent, current.capabilities),
        },
      ];
    }

    const gains = after.filter((capability) => !current.capabilities.includes(capability));
    const loses = current.capabilities.filter((capability) => !after.includes(capability));

    return gains.length + loses.length === 0
      ? []
      : [
          {
            roomId: id,
            room,
            change: "abilities",
            gains: labels(consent, gains),
            loses: labels(consent, loses),
          },
        ];
  });
}

/** Whether a diff changes nothing. */
export function unchanged(diff: AccessDiff): boolean {
  return diff.length === 0;
}

/** One room's change as a sentence: "can now Post messages; can no longer React". */
export function roomChangeText(change: RoomChange): string {
  if (change.change === "joins") {
    return change.gains.length === 0 ? "with no abilities" : `can ${change.gains.join(", ")}`;
  }

  if (change.change === "leaves") {
    return change.loses.length === 0
      ? "it couldn't do anything there"
      : `can no longer ${change.loses.join(", ")}`;
  }

  return [
    change.gains.length === 0 ? null : `can now ${change.gains.join(", ")}`,
    change.loses.length === 0 ? null : `can no longer ${change.loses.join(", ")}`,
  ]
    .filter((part) => part !== null)
    .join("; ");
}

/** Why the form can't be approved yet, or `null` when it can. */
export function consentProblem(consent: OAuthConsent, form: ConsentForm): string | null {
  if (!consent.enabled) return "Personal agents are turned off in this workspace.";

  if (form.pick.kind === "new" && form.name.trim() === "") return "Give the agent a name.";

  if (form.pick.kind === "existing" && pickedAgent(consent, form.pick) === null) {
    return "Choose an agent.";
  }

  return form.roomIds.length > 0 && form.capabilities.length === 0
    ? "Choose at least one ability for its rooms."
    : null;
}

/** The warning under the app's name, by what the server knows of it. */
export function badgeWarning(consent: OAuthConsent): string {
  return consent.badge === "appOnThisComputer"
    ? `This is an app on your own computer (it answers at ${consent.redirectHostname}). Approve it only if you just started connecting it.`
    : `Smartfire hasn't verified this app; its name is what it calls itself. Approve it only if you started connecting it from ${consent.redirectHostname}.`;
}

/** What the error page says for an authorization error code. */
export interface OAuthErrorText {
  readonly title: string;
  readonly message: string;
}

const ERRORS = new Map<string, OAuthErrorText>([
  [
    "invalid_client",
    {
      title: "Couldn't identify the app",
      message:
        "The app asking to connect didn't describe itself in a way Smartfire can check. Try connecting again from the app; if it keeps happening, its makers need to fix it.",
    },
  ],
  [
    "unsupported_client_authentication",
    {
      title: "This app can't connect this way",
      message:
        "The app wants to sign in with a secret of its own, which Smartfire doesn't accept. Connect it with a token from Settings → Agents instead.",
    },
  ],
  [
    "invalid_redirect_uri",
    {
      title: "The app's return address doesn't match",
      message:
        "The app asked to send you back to an address it never registered, so Smartfire stopped. Try connecting again from the app itself.",
    },
  ],
  [
    "access_denied",
    {
      title: "The connection was refused",
      message: "Nothing was connected. You can start again from the app whenever you like.",
    },
  ],
]);

/** The page's title and explanation for `error`, with a general one for codes it doesn't know. */
export function oauthErrorText(error: string | null): OAuthErrorText {
  return (
    (error === null ? undefined : ERRORS.get(error)) ?? {
      title: "Couldn't connect the app",
      message:
        "Something about the request wasn't right, so nothing was connected. Try connecting again from the app.",
    }
  );
}
