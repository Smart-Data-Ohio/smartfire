import { useNavigate } from "@tanstack/react-router";
import { type FormEvent, useEffect, useRef, useState } from "react";
import type { BotKey } from "../../gen/BotKey.ts";
import { bots } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { leftToConfirm } from "../auth/confirmation.ts";
import {
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  useBusy,
} from "../settings/settings-parts.tsx";
import {
  AdministratorsOnly,
  adminFailure,
  keepDraft,
  takeDraft,
  uploaded,
  useAdmin,
} from "./admin-parts.tsx";
import { optional } from "./bot-format.ts";
import { KeyDialog } from "./bot-parts.tsx";

type Fields = Readonly<Record<string, readonly string[]>>;

/** Where the form waits while the classic page confirms the password (not the picture). */
const DRAFT_KEY = "smartfire.draft.admin-new-bot";

interface Draft {
  readonly name: string;
  readonly iconName: string;
  readonly webhookUrl: string;
}

const EMPTY: Draft = { name: "", iconName: "", webhookUrl: "" };

/** The kept draft, once; the empty form when there's none. */
function restored(): Draft {
  const kept = takeDraft(DRAFT_KEY);

  if (kept === null) {
    return EMPTY;
  }

  const fields = new URLSearchParams(kept);

  toast({ title: "Your new bot is back", description: "Create it to keep it." });

  return {
    name: fields.get("name") ?? "",
    iconName: fields.get("iconName") ?? "",
    webhookUrl: fields.get("webhookUrl") ?? "",
  };
}

/**
 * New bot: a name, an optional icon and webhook URL, and a picture. It becomes a workspace agent
 * the administrator owns; its key shows once, then its page opens.
 */
export function BotNewSection() {
  const { workspace } = useAdmin();
  const navigate = useNavigate();
  const [draft, setDraft] = useState<Draft>(EMPTY);
  const [file, setFile] = useState<File | null>(null);
  const [fields, setFields] = useState<Fields>({});
  const [key, setKey] = useState<BotKey | null>(null);
  const input = useRef<HTMLInputElement | null>(null);
  const { busy, track } = useBusy();
  // Whether the kept draft was taken: once, even when Strict Mode runs the effect twice.
  const taken = useRef(false);

  useEffect(() => {
    if (taken.current || !workspace.canAdminister) {
      return;
    }

    taken.current = true;

    const kept = restored();

    if (kept !== EMPTY) setDraft(kept);
  }, [workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const edit = (change: Partial<Draft>) => setDraft((current) => ({ ...current, ...change }));

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setFields({});

    const save = async () => {
      const avatar = file === null ? null : await uploaded(file);

      return bots.create({
        name: draft.name,
        iconName: optional(draft.iconName),
        webhookUrl: optional(draft.webhookUrl),
        avatar,
      });
    };

    void track(
      "create",
      save().then(setKey, (error: Error) => {
        const named = fieldsOf(error);

        if (Object.keys(named).length > 0) {
          setFields(named);

          return;
        }

        if (leftToConfirm(error))
          keepDraft(DRAFT_KEY, new URLSearchParams({ ...draft }).toString());

        adminFailure("Couldn't create the bot", error);
      }),
    );
  };

  const first = (name: string) => fields[name]?.[0];

  return (
    <SettingsPage title="New bot" description="Chat bots post to rooms with their key.">
      <SettingsGroup title="The bot">
        <form className="settings-form" onSubmit={submit}>
          <FieldError message={first("base")} />
          <TextField
            label="Name"
            value={draft.name}
            required
            autoComplete="off"
            placeholder="Name the bot"
            error={first("name")}
            onChange={(event) => edit({ name: event.target.value })}
          />
          <TextField
            label="Icon"
            hint="A :shortcode: from the icon set, shown when there's no picture"
            value={draft.iconName}
            autoComplete="off"
            placeholder="robot"
            error={first("iconName")}
            onChange={(event) => edit({ iconName: event.target.value })}
          />
          <TextField
            label="Webhook URL"
            hint="Messages that mention the bot are posted here"
            type="url"
            value={draft.webhookUrl}
            autoComplete="off"
            placeholder="https://example.com/webhook"
            error={first("webhookUrl")}
            onChange={(event) => edit({ webhookUrl: event.target.value })}
          />
          <div className="settings-field">
            <label className="settings-label" htmlFor="admin-bot-avatar">
              Picture
            </label>
            <input
              ref={input}
              id="admin-bot-avatar"
              className="input"
              type="file"
              accept="image/*"
              onChange={(event) => setFile(event.target.files?.[0] ?? null)}
            />
            <FieldError message={first("avatar")} />
          </div>
          <div className="settings-actions">
            <Button
              type="submit"
              variant="primary"
              icon="bot"
              loading={busy("create")}
              disabled={busy("create")}
            >
              Create bot
            </Button>
          </div>
        </form>
      </SettingsGroup>
      <KeyDialog
        shown={key}
        onClose={() => {
          const created = key;

          setKey(null);

          if (created !== null) {
            void navigate({ to: "/admin/bots/$botId", params: { botId: `${created.id}` } });
          }
        }}
      />
    </SettingsPage>
  );
}
