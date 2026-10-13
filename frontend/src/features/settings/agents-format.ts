/** Settings → Agents: the words, choices and small rules behind the cards and the wizard. */
import type { Icon } from "../../gen/Icon.ts";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";
import type { AgentClient, WebConnector } from "./agent-snippets.ts";

export interface ClientChoice {
  readonly value: AgentClient;
  readonly label: string;
  readonly hint: string;
  /** Stored as the agent's `provider`, so its card names it. */
  readonly provider: string | null;
  /** The icon the Identity step starts with. */
  readonly icon: string;
}

const TOKEN_CLIENTS: readonly ClientChoice[] = [
  {
    value: "claude-code",
    label: "Claude Code",
    hint: "Anthropic's coding agent, in your terminal",
    provider: "anthropic",
    icon: "sparkles",
  },
  {
    value: "codex",
    label: "Codex CLI",
    hint: "OpenAI's coding agent, in your terminal",
    provider: "openai",
    icon: "robot",
  },
  {
    value: "cursor",
    label: "Cursor",
    hint: "The editor's agent",
    provider: "cursor",
    icon: "zap",
  },
  {
    value: "other",
    label: "Other MCP or API client",
    hint: "Anything that can send a header",
    provider: null,
    icon: "robot",
  },
];

const WEB_CONNECTORS: readonly (ClientChoice & { readonly value: WebConnector })[] = [
  {
    value: "claude-ai",
    label: "claude.ai",
    hint: "Claude on the web, desktop and phone",
    provider: "anthropic",
    icon: "sparkles",
  },
  {
    value: "chatgpt",
    label: "ChatGPT",
    hint: "ChatGPT on the web, desktop and phone",
    provider: "openai",
    icon: "robot",
  },
  {
    value: "grok",
    label: "Grok",
    hint: "Grok on the web and phone",
    provider: "xai",
    icon: "rocket",
  },
];

/** The clients the first step offers: the web apps only when the server can sign them in. */
export function clientChoices(oauthAvailable: boolean): readonly ClientChoice[] {
  return oauthAvailable ? [...WEB_CONNECTORS, ...TOKEN_CLIENTS] : TOKEN_CLIENTS;
}

const ALL_CLIENTS = [...TOKEN_CLIENTS, ...WEB_CONNECTORS];

/** The client an agent was set up for (its `runtime`), when it's one we know. */
export function clientOf(agent: PersonalAgent): ClientChoice | null {
  return ALL_CLIENTS.find((choice) => choice.value === agent.runtime) ?? null;
}

/** The emoji icons an agent can wear, by `:shortcode:` (gemoji aliases the server knows). */
export const ICON_CHOICES = [
  { name: "robot", character: "🤖", title: "Robot" },
  { name: "sparkles", character: "✨", title: "Sparkles" },
  { name: "zap", character: "⚡", title: "Zap" },
  { name: "brain", character: "🧠", title: "Brain" },
  { name: "rocket", character: "🚀", title: "Rocket" },
  { name: "owl", character: "🦉", title: "Owl" },
  { name: "fox_face", character: "🦊", title: "Fox" },
  { name: "crystal_ball", character: "🔮", title: "Crystal ball" },
] as const;

/** An agent's icon as AgentAvatar draws it; `null` (its drawn bot) for any other name. */
export function iconOf(name: string | null): Icon | null {
  const choice = ICON_CHOICES.find((icon) => icon.name === name);

  return choice === undefined
    ? null
    : {
        name: choice.name,
        title: choice.title,
        kind: "emoji",
        character: choice.character,
        imageUrl: null,
        animated: false,
        stillUrl: null,
      };
}

const ABILITIES = new Map([
  [
    "read_messages",
    { label: "Read messages", hint: "Read conversations and threads in its rooms" },
  ],
  ["post_messages", { label: "Post messages", hint: "Post, stream replies and attach files" }],
  ["react", { label: "React", hint: "Add and remove emoji reactions" }],
  ["manage_threads", { label: "Manage threads", hint: "Start, rename and close threads" }],
]);

/** An ability's name: "Read messages", or the capability itself spelled out. */
export function abilityLabel(capability: string): string {
  return ABILITIES.get(capability)?.label ?? capability.replaceAll("_", " ");
}

export function abilityHint(capability: string): string | undefined {
  return ABILITIES.get(capability)?.hint;
}

/** How long a new token lasts: the server's 90 days, or no expiry. */
export type Expiry = "90d" | "never";

export const EXPIRY_CHOICES: readonly { readonly value: Expiry; readonly label: string }[] = [
  { value: "90d", label: "90 days" },
  { value: "never", label: "Never" },
];

/** `expiresAt` as the API takes it: `null` is 90 days. */
export function expiresAtOf(expiry: Expiry): string | null {
  return expiry === "never" ? "never" : null;
}

/** The wizard's steps, in order. */
export const STEPS = ["client", "identity", "access", "connect"] as const;

export type Step = (typeof STEPS)[number];

export const STEP_LABELS: Readonly<Record<Step, string>> = {
  client: "Client",
  identity: "Identity",
  access: "Access",
  connect: "Connect",
};

/** What the wizard holds before the agent exists. */
export interface WizardDraft {
  readonly client: AgentClient | null;
  readonly name: string;
  readonly iconName: string | null;
  readonly roomIds: readonly number[];
  readonly capabilities: readonly string[];
  readonly expiry: Expiry;
}

/** A fresh draft: every ability ticked, no rooms (the owner's DM works regardless). */
export function emptyDraft(capabilities: readonly string[]): WizardDraft {
  return {
    client: null,
    name: "",
    iconName: null,
    roomIds: [],
    capabilities: [...capabilities],
    expiry: "90d",
  };
}

/** Why `step` can't move on yet, or `null` when it can. */
export function stepProblem(step: Step, draft: WizardDraft): string | null {
  switch (step) {
    case "client":
      return draft.client === null ? "Choose where you'll use the agent." : null;

    case "identity":
      return draft.name.trim() === "" ? "Give the agent a name." : null;

    case "access":
      return draft.roomIds.length > 0 && draft.capabilities.length === 0
        ? "Choose at least one ability for its rooms."
        : null;

    case "connect":
      return null;
  }
}

/** The draft with the client picked: its icon too, unless one was already chosen. */
export function withClient(draft: WizardDraft, choice: ClientChoice): WizardDraft {
  const previous = ALL_CLIENTS.find((c) => c.value === draft.client);
  const keepIcon = draft.iconName !== null && draft.iconName !== previous?.icon;

  return { ...draft, client: choice.value, iconName: keepIcon ? draft.iconName : choice.icon };
}

/** `values` with `value` in or out, in its original order. */
export function toggled<T>(values: readonly T[], value: T, on: boolean): T[] {
  const without = values.filter((v) => v !== value);

  return on ? [...without, value] : without;
}

/** Whether the person may add another agent: on, and under the limit (admins have none). */
export function canCreate(list: PersonalAgentList): boolean {
  return list.enabled && (list.limit === null || list.agents.length < list.limit);
}

/** "2 of 3 agents", or `null` for administrators, who have no limit. */
export function usage(list: PersonalAgentList): string | null {
  return list.limit === null ? null : `${list.agents.length} of ${list.limit} agents`;
}

/** The agent's rooms by name, in the list's order; rooms it can't see any more drop out. */
export function roomNames(list: PersonalAgentList, roomIds: readonly number[]): string[] {
  return list.rooms.filter((room) => roomIds.includes(room.id)).map((room) => room.name);
}

/** Tokens newest first; revoked ones last. */
export function sortedTokens(
  credentials: readonly PersonalAgentCredential[],
): PersonalAgentCredential[] {
  const rank = (credential: PersonalAgentCredential) => (credential.state === "revoked" ? 1 : 0);

  return [...credentials].sort((a, b) => rank(a) - rank(b) || b.id - a.id);
}

const day = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" });

/** "Oct 13, 2026". */
export function shortDay(at: string): string {
  return day.format(Date.parse(at));
}

/** "Created Oct 13, 2026 · expires Jan 11, 2027 · never used". */
export function tokenMeta(
  credential: PersonalAgentCredential,
  ago: (at: string) => string,
): string {
  const parts = [`Created ${shortDay(credential.createdAt)}`];

  if (credential.state !== "revoked") {
    parts.push(
      credential.expiresAt === null
        ? "never expires"
        : `${credential.state === "expired" ? "expired" : "expires"} ${shortDay(credential.expiresAt)}`,
    );
  }

  parts.push(
    credential.lastUsedAt === null ? "never used" : `last used ${ago(credential.lastUsedAt)}`,
  );

  return parts.join(" · ");
}

export const TOKEN_STATE: Readonly<Record<PersonalAgentCredential["state"], string>> = {
  active: "Active",
  expired: "Expired",
  revoked: "Revoked",
};

/** A draft as text for session storage: what was chosen, never a token. */
export function serializeDraft(draft: WizardDraft): string {
  return new URLSearchParams({
    client: draft.client ?? "",
    name: draft.name,
    iconName: draft.iconName ?? "",
    roomIds: draft.roomIds.join(","),
    capabilities: draft.capabilities.join(","),
    expiry: draft.expiry,
  }).toString();
}

/** A comma list, its blanks dropped. */
function items(text: string | null): string[] {
  return (text ?? "").split(",").filter((item) => item !== "");
}

/**
 * A draft kept across a Google confirmation, read back; `null` when the text isn't one (no known
 * client, say).
 */
export function parseDraft(text: string): WizardDraft | null {
  const fields = new URLSearchParams(text);
  const client = ALL_CLIENTS.find((choice) => choice.value === fields.get("client"));
  const expiry = EXPIRY_CHOICES.find((choice) => choice.value === fields.get("expiry"));
  const iconName = fields.get("iconName") ?? "";

  if (client === undefined || expiry === undefined) return null;

  return {
    client: client.value,
    name: fields.get("name") ?? "",
    iconName: iconName === "" ? null : iconName,
    roomIds: items(fields.get("roomIds"))
      .map(Number)
      .filter((id) => Number.isSafeInteger(id) && id > 0),
    capabilities: items(fields.get("capabilities")),
    expiry: expiry.value,
  };
}
