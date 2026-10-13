import { useId, useRef } from "react";
import type { GrantRoom } from "../../gen/GrantRoom.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { copy } from "../admin/admin-parts.tsx";
import { type AgentClient, agentSnippet } from "./agent-snippets.ts";
import {
  abilityHint,
  abilityLabel,
  EXPIRY_CHOICES,
  type Expiry,
  ICON_CHOICES,
  toggled,
} from "./agents-format.ts";
import { SettingsRadios } from "./settings-parts.tsx";

/** The agent's name and its icon. */
export function IdentityFields({
  name,
  iconName,
  error,
  onName,
  onIcon,
}: {
  readonly name: string;
  readonly iconName: string | null;
  readonly error?: string | undefined;
  readonly onName: (name: string) => void;
  readonly onIcon: (iconName: string) => void;
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
      <fieldset className="agents-icons">
        <legend className="settings-label">Icon</legend>
        <div className="agents-icon-row">
          {ICON_CHOICES.map((icon) => (
            <label key={icon.name} className="agents-icon" title={icon.title}>
              <input
                type="radio"
                name={group}
                value={icon.name}
                checked={iconName === icon.name}
                aria-label={icon.title}
                onChange={() => onIcon(icon.name)}
              />
              <span aria-hidden="true">{icon.character}</span>
            </label>
          ))}
        </div>
      </fieldset>
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
  /** The non-direct rooms the person is in. */
  readonly rooms: readonly GrantRoom[];
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
                  label={room.name}
                  onCheckedChange={(on) => onRooms(toggled(roomIds, room.id, on))}
                />
              </li>
            ))}
          </ul>
        )}
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
