import { type ReactNode, useId, useRef } from "react";
import type { PersonalAgentRoom } from "../../gen/PersonalAgentRoom.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { copy } from "../admin/admin-parts.tsx";
import type { AvatarChoice } from "../bot-avatar/avatar-choice.ts";
import { BotAvatarPicker } from "../bot-avatar/bot-avatar-picker.tsx";
import {
  type AgentClient,
  agentSnippet,
  canSignIn,
  type Snippet,
  signInSnippet,
} from "./agent-snippets.ts";
import {
  abilityHint,
  abilityLabel,
  EXPIRY_CHOICES,
  type Expiry,
  toggled,
} from "./agents-format.ts";
import { SettingsRadios } from "./settings-parts.tsx";

/** The agent's name and its picture: a picked bot avatar, or an uploaded image. */
export function IdentityFields({
  name,
  avatar,
  current,
  error,
  onName,
  onAvatar,
}: {
  readonly name: string;
  readonly avatar: AvatarChoice;
  /** An existing agent's icon or picture, drawn, offered as "Keep current". */
  readonly current?: ReactNode;
  readonly error?: string | undefined;
  readonly onName: (name: string) => void;
  readonly onAvatar: (avatar: AvatarChoice) => void;
}) {
  const group = useId();

  return (
    <div className="settings-form">
      <TextField
        label="Name"
        value={name}
        required
        maxLength={80}
        autoComplete="off"
        placeholder="My Claude"
        hint="People see this name when it posts."
        error={error}
        onChange={(event) => onName(event.target.value)}
      />
      <BotAvatarPicker
        id={`agent-avatar${group.replaceAll(":", "")}`}
        name={name}
        value={avatar}
        onChange={onAvatar}
        current={current}
      />
    </div>
  );
}

/** The rooms an agent joins and what it may do there. */
export function AccessFields({
  rooms,
  capabilities,
  roomIds,
  granted,
  onRooms,
  onCapabilities,
}: {
  /** The non-direct rooms the person is in; private ones are marked. */
  readonly rooms: readonly PersonalAgentRoom[];
  /** The abilities the workspace lets a personal agent have. */
  readonly capabilities: readonly string[];
  readonly roomIds: readonly number[];
  readonly granted: readonly string[];
  readonly onRooms: (roomIds: number[]) => void;
  readonly onCapabilities: (capabilities: string[]) => void;
}) {
  return (
    <div className="settings-form">
      <fieldset className="agents-choices">
        <legend className="settings-label">Rooms</legend>
        <p className="settings-hint text-faint">
          It joins each room you pick and shows in its member list; room managers can remove it.
          With no rooms, it can only talk with you in your direct messages.
        </p>
        {rooms.length === 0 ? (
          <p className="text-muted">You're not in any rooms it could join.</p>
        ) : (
          <ul className="agents-checks">
            {rooms.map((room) => (
              <li key={room.id}>
                <Checkbox
                  checked={roomIds.includes(room.id)}
                  label={
                    room.private ? (
                      <span>
                        {room.name} <span className="settings-badge">Private</span>
                      </span>
                    ) : (
                      room.name
                    )
                  }
                  onCheckedChange={(on) => onRooms(toggled(roomIds, room.id, on))}
                />
              </li>
            ))}
          </ul>
        )}
        {rooms.some((room) => room.private) ? (
          <p className="settings-hint text-faint" role="note">
            In a private room, everyone in it will see your agent in the member list.
          </p>
        ) : null}
      </fieldset>
      <fieldset className="agents-choices">
        <legend className="settings-label">Abilities in those rooms</legend>
        <ul className="agents-checks">
          {capabilities.map((capability) => (
            <li key={capability}>
              <Checkbox
                checked={granted.includes(capability)}
                label={
                  <span className="agents-check-label">
                    <span>{abilityLabel(capability)}</span>
                    {abilityHint(capability) === undefined ? null : (
                      <span className="text-faint">{abilityHint(capability)}</span>
                    )}
                  </span>
                }
                onCheckedChange={(on) => onCapabilities(toggled(granted, capability, on))}
              />
            </li>
          ))}
        </ul>
        <p className="settings-hint text-faint">
          Personal agents can only chat. They can't take actions in other apps or message anyone but
          you directly.
        </p>
      </fieldset>
    </div>
  );
}

/** How long a new token lasts. */
export function ExpiryField({
  value,
  onChange,
}: {
  readonly value: Expiry;
  readonly onChange: (value: Expiry) => void;
}) {
  return (
    <SettingsRadios
      label="Token expires"
      value={value}
      choices={EXPIRY_CHOICES}
      onChange={onChange}
    />
  );
}

/** Text to copy: a `<pre>` (it scrolls sideways on its own) and a Copy button. */
export function CodeBlock({
  label,
  code,
  what,
  wrap = false,
}: {
  readonly label: string;
  readonly code: string;
  /** The toast's subject: "Token copied". */
  readonly what: string;
  /** Break a single long word (a token) across lines instead of scrolling it. */
  readonly wrap?: boolean;
}) {
  const shown = useRef<HTMLPreElement>(null);
  const id = useId();

  return (
    <div className="agents-code">
      <div className="agents-code-header">
        <span id={id} className="settings-label">
          {label}
        </span>
        <Button
          variant="ghost"
          size="sm"
          icon="copy"
          aria-describedby={id}
          onClick={() => void copy(code, what, shown.current)}
        >
          Copy
        </Button>
      </div>
      <pre ref={shown} className="agents-pre" data-wrap={wrap ? "" : undefined}>
        <code>{code}</code>
      </pre>
    </div>
  );
}

/** Said where the MCP URL would be, when the server has no public address configured. */
export function NoPublicUrl() {
  return (
    <p className="settings-callout text-muted" role="note">
      This server's public address isn't configured yet, so there's no URL to connect to. An
      administrator needs to set it (APP_URL). Keep the token: it will work once the address is set.
    </p>
  );
}

/** A secret, shown once, and how `client` uses it. */
export function SecretBlock({
  secret,
  client,
  agentName,
  mcpUrl,
}: {
  readonly secret: string;
  readonly client: AgentClient | null;
  readonly agentName: string;
  /** `null` when the server has no public address configured. */
  readonly mcpUrl: string | null;
}) {
  const snippet =
    mcpUrl === null || client === null ? null : agentSnippet(client, { agentName, mcpUrl, secret });

  return (
    <div className="agents-secret">
      <p className="agents-secret-warning" role="note">
        Copy the token now. It won't be shown again.
      </p>
      <CodeBlock label="Token" code={secret} what="Token" wrap />
      {mcpUrl === null ? <NoPublicUrl /> : null}
      {snippet === null ? null : (
        <>
          {snippet.steps.map((step) => (
            <CodeBlock key={step.label} label={step.label} code={step.code} what="Setup" />
          ))}
          <p className="settings-hint text-faint">{snippet.note}</p>
        </>
      )}
    </div>
  );
}

/** Said where the connector URL would be: signing in needs the server's public address. */
function NoSignInUrl() {
  return (
    <p className="settings-callout text-muted" role="note">
      This server's public address isn't configured yet, so apps can't sign in to it. An
      administrator needs to set it (APP_URL).
    </p>
  );
}

/** A snippet's things to copy, then its note. */
function SnippetSteps({ snippet }: { readonly snippet: Snippet }) {
  return (
    <>
      {snippet.steps.map((step) => (
        <CodeBlock key={step.label} label={step.label} code={step.code} what="Setup" />
      ))}
      <p className="settings-hint text-faint">{snippet.note}</p>
    </>
  );
}

/**
 * How a client that signs in connects: the connector URL and where it goes (a web app), or the
 * command that adds the server and signs in (Claude Code, Codex). No token is shown or kept.
 */
export function SignInSetup({
  client,
  mcpUrl,
}: {
  readonly client: AgentClient;
  /** `null` when the server has no public address configured. */
  readonly mcpUrl: string | null;
}) {
  if (mcpUrl === null) return <NoSignInUrl />;

  const snippet = canSignIn(client)
    ? signInSnippet(client, mcpUrl)
    : agentSnippet(client, { agentName: "", mcpUrl });

  return (
    <div className="agents-secret">
      <SnippetSteps snippet={snippet} />
      <p className="settings-hint text-faint">
        There's no token to keep. On Smartfire's approval page you pick a new or existing agent, its
        rooms and abilities, and confirm your password. A connection lasts up to 180 days, then you
        approve it again.
      </p>
    </div>
  );
}

/**
 * Reconnecting a connection: the app has to sign in again, from its own settings, so this says
 * how, with the connector URL. A product known by its exact client id gets its own steps.
 */
export function ReconnectSetup({
  client,
  clientName,
  agentName,
  mcpUrl,
}: {
  /** The product it is, when its client id says so; else it's named only by `clientName`. */
  readonly client: AgentClient | null;
  readonly clientName: string;
  readonly agentName: string;
  readonly mcpUrl: string | null;
}) {
  if (mcpUrl === null) return <NoSignInUrl />;

  const snippet: Snippet =
    client === null
      ? {
          steps: [{ label: "Connector URL", code: mcpUrl }],
          note: `Sign in again from the app you connected (${clientName}): reconnect Smartfire where it lists its connectors or MCP servers, or add it again with this URL.`,
        }
      : canSignIn(client)
        ? signInSnippet(client, mcpUrl)
        : agentSnippet(client, { agentName, mcpUrl });

  return (
    <div className="agents-secret">
      <SnippetSteps snippet={snippet} />
      <p className="settings-hint text-faint">
        When Smartfire's approval page opens, choose {agentName} (it's picked already if it's the
        last agent this app connected as), check its rooms and abilities, and approve.
      </p>
    </div>
  );
}
