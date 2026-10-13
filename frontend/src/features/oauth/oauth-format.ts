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

/** How approving changes an existing agent's access, in names. */
export interface AccessDiff {
  readonly joins: readonly string[];
  readonly leaves: readonly string[];
  readonly gains: readonly string[];
  readonly loses: readonly string[];
}

/** A room's name, or what it is when the person can't see it any more. */
function roomName(consent: OAuthConsent, id: number): string {
  return consent.rooms.find((room) => room.id === id)?.name ?? "A room you're no longer in";
}

/** What approving `form` changes about `agent`'s rooms and abilities. */
export function accessDiff(
  consent: OAuthConsent,
  agent: PersonalAgent,
  form: ConsentForm,
): AccessDiff {
  const before = effective(agent.roomIds, agent.capabilities);
  const after = effective(form.roomIds, form.capabilities);

  return {
    joins: form.roomIds
      .filter((id) => !agent.roomIds.includes(id))
      .map((id) => roomName(consent, id)),
    leaves: agent.roomIds
      .filter((id) => !form.roomIds.includes(id))
      .map((id) => roomName(consent, id)),
    gains: consent.capabilities
      .filter((capability) => after.includes(capability) && !before.includes(capability))
      .map(abilityLabel),
    loses: consent.capabilities
      .filter((capability) => before.includes(capability) && !after.includes(capability))
      .map(abilityLabel),
  };
}

/** Whether a diff changes nothing. */
export function unchanged(diff: AccessDiff): boolean {
  return diff.joins.length + diff.leaves.length + diff.gains.length + diff.loses.length === 0;
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
