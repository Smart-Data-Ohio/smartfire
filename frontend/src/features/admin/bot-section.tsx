import { Link, useNavigate } from "@tanstack/react-router";
import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import type { Bot } from "../../gen/Bot.ts";
import type { BotChange } from "../../gen/BotChange.ts";
import type { BotKey } from "../../gen/BotKey.ts";
import { bots } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import {
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  useBusy,
} from "../settings/settings-parts.tsx";
import {
  adminFailure,
  Confirm,
  keepDraft,
  needsSudo,
  takeDraft,
  uploaded,
  useAdmin,
} from "./admin-parts.tsx";
import { type BotForm, botChange, botForm, CONFIRM, SUSPENDED } from "./bot-format.ts";
import { BotPicture, CopyLine, KeyDialog, useBotId } from "./bot-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly bot: Bot };

type Fields = Readonly<Record<string, readonly string[]>>;

type Ask = Parameters<typeof Confirm>[0]["ask"];

/** Where an unsaved edit of bot `id` waits while the classic page confirms the password. */
const draftKey = (id: number) => `smartfire.draft.admin-bot-${id}`;

/** The kept edit of bot `id`, once, over `form`; `null` when there's none. */
function restored(id: number, form: BotForm): BotForm | null {
  const kept = takeDraft(draftKey(id));

  if (kept === null) {
    return null;
  }

  const fields = new URLSearchParams(kept);
  const field = (key: keyof BotForm) => fields.get(key) ?? form[key];

  return {
    name: field("name"),
    iconName: field("iconName"),
    webhookUrl: field("webhookUrl"),
    provider: field("provider"),
    runtime: field("runtime"),
    description: field("description"),
    dailyMessageCap: field("dailyMessageCap"),
    dailyBoardPostCap: field("dailyBoardPostCap"),
    dailyExternalActionCap: field("dailyExternalActionCap"),
  };
}

/** A budget's input: a whole number, blank for unlimited. */
function CapField({
  label,
  value,
  error,
  onChange,
}: {
  readonly label: string;
  readonly value: string;
  readonly error: string | undefined;
  readonly onChange: (value: string) => void;
}) {
  return (
    <TextField
      label={label}
      value={value}
      inputMode="numeric"
      autoComplete="off"
      placeholder="Unlimited"
      error={error}
      onChange={(event) => onChange(event.target.value)}
    />
  );
}

/** The GitHub account the agent's approved write actions post as. */
function GithubGroup({
  bot,
  busy,
  onConnect,
  onDisconnect,
}: {
  readonly bot: Bot;
  readonly busy: boolean;
  /** Answers whether GitHub took the token. */
  readonly onConnect: (token: string, failed: (fields: Fields) => void) => Promise<boolean>;
  readonly onDisconnect: () => void;
}) {
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const { github } = bot;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setError(undefined);
    void onConnect(token, (fields) => setError(fields.accessToken?.[0])).then((linked) => {
      if (linked) setToken("");
    });
  };

  return (
    <SettingsGroup title="GitHub account">
      {github === null ? null : (
        <div className="admin-secret">
          <p>
            Connected as <strong>{github.login}</strong>. Approved write actions post from PR
            threads as {github.login} on GitHub.
          </p>
          {github.usable ? null : (
            <p className="settings-callout" role="status">
              GitHub rejected the connection
              {github.disconnectedReason === null ? "." : `: ${github.disconnectedReason}`}
            </p>
          )}
          {bot.canAdminister ? (
            <div className="settings-actions">
              <Button variant="secondary" disabled={busy} onClick={onDisconnect}>
                Disconnect GitHub
              </Button>
            </div>
          ) : null}
        </div>
      )}
      {github === null && !bot.canAdminister ? (
        <p className="text-muted">
          No GitHub account is connected. Only an administrator can connect one.
        </p>
      ) : null}
      {bot.canAdminister && (github === null || !github.usable) ? (
        <form className="settings-form" onSubmit={submit}>
          <TextField
            label="Personal access token"
            hint="A fine-grained token for a machine user dedicated to this agent, with Pull requests and Issues (read and write) and Metadata (read). It's checked with GitHub before it's stored, and never shown again."
            type="password"
            value={token}
            autoComplete="off"
            error={error}
            onChange={(event) => setToken(event.target.value)}
          />
          <div className="settings-actions">
            <Button type="submit" variant="primary" loading={busy} disabled={busy}>
              Connect GitHub
            </Button>
          </div>
        </form>
      ) : null}
    </SettingsGroup>
  );
}

/**
 * One bot, as its classic edit page shows it: its name, icon, picture and webhook, its agent's
 * description and daily budgets, the signing secret, the key, the GitHub account, its credentials
 * and grants, the kill switch, and removal. Its agent's owner sees it too, without the
 * administrators' controls.
 */
export function BotSection() {
  const id = useBotId();
  const { workspace } = useAdmin();
  const navigate = useNavigate();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [form, setForm] = useState<BotForm | null>(null);
  const [file, setFile] = useState<File | null>(null);
  const [fields, setFields] = useState<Fields>({});
  const [key, setKey] = useState<BotKey | null>(null);
  const [ask, setAsk] = useState<Ask>(null);
  const input = useRef<HTMLInputElement | null>(null);
  const { busy, track } = useBusy();
  // Whether the kept edit was taken: once, even when a load lands twice.
  const taken = useRef(false);

  const show = useCallback((bot: Bot) => {
    setLoad({ status: "ready", bot });
    setForm(botForm(bot));
  }, []);

  const fetchBot = useCallback(() => {
    bots.get(id).then(
      (bot) => {
        show(bot);

        if (taken.current) {
          return;
        }

        taken.current = true;

        const kept = restored(id, botForm(bot));

        if (kept !== null) {
          setForm(kept);
          toast({ title: "Your unsaved changes are back", description: "Save them to keep them." });
        }
      },
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, [id, show]);

  useEffect(fetchBot, [fetchBot]);

  if (load.status === "error") {
    const retry = () => {
      setLoad({ status: "loading" });
      fetchBot();
    };

    return <PaneError message={load.message} onRetry={retry} />;
  }

  if (load.status === "loading" || form === null) {
    return <PaneListSkeleton rows={4} />;
  }

  const { bot } = load;

  const landed = ({ bot: next, notice }: BotChange) => {
    show(next);

    if (notice !== null) toast({ title: notice, tone: "success" });
  };

  const edit = (change: Partial<BotForm>) =>
    setForm((current) => (current === null ? current : { ...current, ...change }));

  const save = (event: FormEvent) => {
    event.preventDefault();
    setFields({});

    const run = async () => {
      const avatar = file === null ? null : await uploaded(file);

      return bots.update(id, { ...botChange(form, bot), avatar });
    };

    void track(
      "save",
      run().then(
        (change) => {
          landed(change);
          setFile(null);

          if (input.current !== null) input.current.value = "";

          toast({ title: "Bot saved", tone: "success" });
        },
        (error: Error) => {
          const named = fieldsOf(error);

          if (Object.keys(named).length > 0) {
            setFields(named);

            return;
          }

          if (needsSudo(error))
            keepDraft(draftKey(id), new URLSearchParams({ ...form }).toString());

          adminFailure("Couldn't save the bot", error);
        },
      ),
    );
  };

  const write = (what: string, work: () => Promise<BotChange>) => {
    void track(
      what,
      work().then(landed, (error: Error) => adminFailure("Couldn't change the bot", error)),
    );
  };

  const connect = (token: string, failed: (named: Fields) => void) =>
    track(
      "github",
      bots.connectGithub(id, token).then(
        (change) => {
          landed(change);

          return true;
        },
        (error: Error) => {
          const named = fieldsOf(error);

          if (Object.keys(named).length > 0) {
            failed(named);
          } else {
            adminFailure("Couldn't connect GitHub", error);
          }

          return false;
        },
      ),
    );

  const first = (name: string) => fields[name]?.[0];
  const { agent } = bot;
  const signs = agent !== null || bot.webhookUrl !== null;

  return (
    <SettingsPage title={bot.name} description="Chat bot setup.">
      {workspace.canAdminister ? (
        <Link to="/admin/bots" className="settings-link admin-back">
          <Icon name="chevron-left" size={14} />
          All chat bots
        </Link>
      ) : null}
      <SettingsGroup title="The bot">
        <form className="settings-form" onSubmit={save}>
          <FieldError message={first("base")} />
          <div className="settings-avatar">
            <BotPicture
              id={bot.id}
              name={bot.name}
              avatarUrl={bot.avatarUrl}
              icon={bot.avatarAttached ? null : bot.icon}
              size={64}
            />
            <div className="settings-field">
              <label className="settings-label" htmlFor="admin-bot-picture">
                Picture
              </label>
              <input
                ref={input}
                id="admin-bot-picture"
                className="input"
                type="file"
                accept="image/*"
                onChange={(event) => setFile(event.target.files?.[0] ?? null)}
              />
              <FieldError message={first("avatar")} />
            </div>
          </div>
          <TextField
            label="Name"
            value={form.name}
            required
            autoComplete="off"
            placeholder="Name the bot"
            error={first("name")}
            onChange={(event) => edit({ name: event.target.value })}
          />
          <TextField
            label="Icon"
            hint="A :shortcode: from the icon set, shown when there's no picture"
            value={form.iconName}
            autoComplete="off"
            placeholder="robot"
            error={first("iconName")}
            onChange={(event) => edit({ iconName: event.target.value })}
          />
          <TextField
            label="Webhook URL"
            hint={
              bot.canAdminister
                ? "Messages that mention the bot are posted here"
                : "Only an administrator can change the webhook URL"
            }
            type="url"
            value={form.webhookUrl}
            disabled={!bot.canAdminister}
            autoComplete="off"
            placeholder="https://example.com/webhook"
            error={first("webhookUrl")}
            onChange={(event) => edit({ webhookUrl: event.target.value })}
          />
          {agent === null ? null : (
            <>
              <TextField
                label="Provider"
                value={form.provider}
                autoComplete="off"
                placeholder="Provider (e.g. OpenAI)"
                error={first("provider")}
                onChange={(event) => edit({ provider: event.target.value })}
              />
              <TextField
                label="Runtime"
                value={form.runtime}
                autoComplete="off"
                placeholder="Runtime (e.g. Codex CLI 0.9)"
                error={first("runtime")}
                onChange={(event) => edit({ runtime: event.target.value })}
              />
              <div className="settings-field">
                <label className="settings-label" htmlFor="admin-bot-description">
                  Description
                </label>
                <textarea
                  id="admin-bot-description"
                  className="input settings-textarea"
                  rows={3}
                  maxLength={500}
                  value={form.description}
                  placeholder="What this agent does (max 500 characters)"
                  onChange={(event) => edit({ description: event.target.value })}
                />
                <FieldError message={first("description")} />
              </div>
              <div className="settings-inline admin-icon-names">
                <CapField
                  label="Messages a day"
                  value={form.dailyMessageCap}
                  error={first("dailyMessageCap")}
                  onChange={(value) => edit({ dailyMessageCap: value })}
                />
                <CapField
                  label="Board posts a day"
                  value={form.dailyBoardPostCap}
                  error={first("dailyBoardPostCap")}
                  onChange={(value) => edit({ dailyBoardPostCap: value })}
                />
                <CapField
                  label="External actions a day"
                  value={form.dailyExternalActionCap}
                  error={first("dailyExternalActionCap")}
                  onChange={(value) => edit({ dailyExternalActionCap: value })}
                />
              </div>
              <p className="settings-hint text-faint">Today: {agent.usage}</p>
            </>
          )}
          <div className="settings-actions">
            <Button type="submit" variant="primary" loading={busy("save")} disabled={busy("save")}>
              Save
            </Button>
          </div>
        </form>
      </SettingsGroup>
      {agent === null ? null : (
        <SettingsGroup title="Agent">
          <div className="settings-actions">
            <a className="settings-classic-link" href={agent.ledgerUrl}>
              Activity ledger
              <Icon name="external-link" size={14} />
            </a>
            <a className="settings-classic-link" href={agent.approvalsUrl}>
              Approval requests
              <Icon name="external-link" size={14} />
            </a>
          </div>
          <div className="settings-actions">
            <Link
              to="/admin/bots/$botId/credentials"
              params={{ botId: `${bot.id}` }}
              className="button"
              data-variant="secondary"
              data-size="sm"
            >
              Credentials
            </Link>
            <Link
              to="/admin/bots/$botId/grants"
              params={{ botId: `${bot.id}` }}
              className="button"
              data-variant="secondary"
              data-size="sm"
            >
              Grants
            </Link>
          </div>
          {agent.suspended ? (
            <p className="settings-callout" role="status">
              This agent is suspended.
            </p>
          ) : (
            <div className="settings-actions">
              <Button
                variant="danger"
                size="sm"
                disabled={busy("suspend")}
                onClick={() =>
                  setAsk({
                    title: "Suspend this agent?",
                    message: CONFIRM.suspend,
                    label: "Suspend",
                    danger: true,
                    run: () =>
                      write("suspend", () =>
                        bots.suspend(id).then((change) => ({
                          ...change,
                          notice: change.notice ?? SUSPENDED,
                        })),
                      ),
                  })
                }
              >
                Kill switch
              </Button>
            </div>
          )}
        </SettingsGroup>
      )}
      {signs ? (
        <SettingsGroup title="Webhook signing secret">
          {bot.signingSecret === null ? (
            <p className="text-muted">
              No signing secret yet. Deliveries go out unsigned until one is generated.
            </p>
          ) : (
            <>
              <p>
                Deliveries carry an <code className="admin-code">X-Smartfire-Signature</code>{" "}
                header. Copy this secret into the receiving service to verify them:
              </p>
              <CopyLine text={bot.signingSecret} what="Signing secret" />
            </>
          )}
          <div className="settings-actions">
            <Button
              variant={bot.signingSecret === null ? "secondary" : "danger"}
              size="sm"
              disabled={busy("secret")}
              onClick={() => {
                const run = () => write("secret", () => bots.resetSigningSecret(id));

                if (bot.signingSecret === null) {
                  run();

                  return;
                }

                setAsk({
                  title: "Reset the signing secret?",
                  message: CONFIRM.secret,
                  label: "Reset",
                  danger: true,
                  run,
                });
              }}
            >
              {bot.signingSecret === null ? "Generate signing secret" : "Reset signing secret"}
            </Button>
          </div>
        </SettingsGroup>
      ) : null}
      {agent === null ? null : (
        <GithubGroup
          bot={bot}
          busy={busy("github")}
          onConnect={connect}
          onDisconnect={() =>
            setAsk({
              title: "Disconnect GitHub?",
              message: CONFIRM.disconnect,
              label: "Disconnect",
              danger: true,
              run: () => write("github", () => bots.disconnectGithub(id)),
            })
          }
        />
      )}
      {bot.canAdminister ? (
        <SettingsGroup title="Key and removal">
          <div className="settings-actions">
            <Button
              variant="danger"
              size="sm"
              disabled={busy("key")}
              onClick={() =>
                setAsk({
                  title: "Generate a new key?",
                  message: CONFIRM.key,
                  label: "Generate",
                  danger: true,
                  run: () =>
                    void track(
                      "key",
                      bots
                        .resetKey(id)
                        .then(setKey, (error: Error) =>
                          adminFailure("Couldn't generate a new key", error),
                        ),
                    ),
                })
              }
            >
              Generate a new key
            </Button>
            <Button
              variant="danger"
              size="sm"
              icon="trash"
              disabled={busy("remove")}
              onClick={() =>
                setAsk({
                  title: `Remove ${bot.name}?`,
                  message: CONFIRM.remove,
                  label: "Remove",
                  danger: true,
                  run: () =>
                    void track(
                      "remove",
                      bots.remove(id).then(
                        () => {
                          toast({ title: `${bot.name} was removed`, tone: "success" });
                          void navigate({ to: "/admin/bots" });
                        },
                        (error: Error) => adminFailure(`Couldn't remove ${bot.name}`, error),
                      ),
                    ),
                })
              }
            >
              Remove this bot
            </Button>
          </div>
        </SettingsGroup>
      ) : null}
      <a className="settings-classic-link" href={`/account/bots/${bot.id}/edit?classic=1`}>
        Fizzy and the rest on the classic page
        <Icon name="external-link" size={14} />
      </a>
      <Confirm ask={ask} onCancel={() => setAsk(null)} />
      <KeyDialog shown={key} onClose={() => setKey(null)} />
    </SettingsPage>
  );
}
