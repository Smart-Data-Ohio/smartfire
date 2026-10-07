import { Link, notFound, useParams } from "@tanstack/react-router";
import type { BotIcon } from "../../gen/BotIcon.ts";
import type { BotKey } from "../../gen/BotKey.ts";
import { Avatar } from "../../ui/avatar.tsx";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { copy } from "./admin-parts.tsx";

/** The bot id in the URL (`/app/admin/bots/$botId/...`). */
export function useBotId(): number {
  const { botId } = useParams({ strict: false });
  const id = Number(botId);

  if (!Number.isSafeInteger(id) || id <= 0) {
    throw notFound();
  }

  return id;
}

/** A bot's picture: its icon when it has one and no upload, else its avatar. */
export function BotPicture({
  id,
  name,
  avatarUrl,
  icon,
  size = 32,
}: {
  readonly id: number;
  readonly name: string;
  readonly avatarUrl: string;
  readonly icon: BotIcon | null;
  readonly size?: number;
}) {
  if (icon?.kind === "emoji") {
    return (
      <span
        className="admin-bot-emoji"
        role="img"
        aria-label={icon.title}
        style={{ fontSize: size * 0.75, width: size, height: size }}
      >
        {icon.character}
      </span>
    );
  }

  if (icon?.kind === "image") {
    return (
      <img
        className="admin-icon-image"
        src={icon.url}
        alt={icon.title}
        width={size}
        height={size}
      />
    );
  }

  return <Avatar name={name} userId={id} src={avatarUrl} size={size} />;
}

/** A line of code with a Copy button, for a command or a secret. */
export function CopyLine({ text, what }: { readonly text: string; readonly what: string }) {
  return (
    <div className="admin-copy-line">
      <code className="admin-code">{text}</code>
      <Button variant="ghost" size="sm" icon="copy" onClick={() => copy(text, what)}>
        <span className="visually-hidden">Copy {what.toLowerCase()}</span>
      </Button>
    </div>
  );
}

/** Where a bot's own pages link back to: the bot, and the list for administrators. */
export function BotBack({ botId, label }: { readonly botId: number; readonly label: string }) {
  return (
    <Link
      to="/admin/bots/$botId"
      params={{ botId: `${botId}` }}
      className="settings-link admin-back"
    >
      <Icon name="chevron-left" size={14} />
      {label}
    </Link>
  );
}

/** A bot's key, shown once, as the classic key page shows it. */
export function KeyDialog({
  shown,
  onClose,
}: {
  readonly shown: BotKey | null;
  readonly onClose: () => void;
}) {
  return (
    <Dialog
      open={shown !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      role="alertdialog"
      title={shown === null ? "" : `${shown.name}'s key`}
      description="It will not be shown again. Generate a new key if it is lost."
      footer={
        <Button variant="primary" onClick={onClose} data-autofocus>
          Done
        </Button>
      }
    >
      {shown === null ? null : (
        <div className="admin-secret">
          <CopyLine text={shown.key} what="Bot key" />
          <p className="settings-hint text-faint">Post with</p>
          <CopyLine text={shown.exampleCommand} what="Command" />
        </div>
      )}
    </Dialog>
  );
}

/** A new credential's secret, shown once, as the classic page shows it. */
export function SecretDialog({
  secret,
  usage,
  onClose,
}: {
  readonly secret: string | null;
  readonly usage: string;
  readonly onClose: () => void;
}) {
  return (
    <Dialog
      open={secret !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      role="alertdialog"
      title="Copy this secret now"
      description="It will not be shown again."
      footer={
        <Button variant="primary" onClick={onClose} data-autofocus>
          Done
        </Button>
      }
    >
      {secret === null ? null : (
        <div className="admin-secret">
          <CopyLine text={secret} what="Credential secret" />
          <p className="settings-hint text-faint">
            Send it as <code className="admin-code">{usage}</code>
          </p>
        </div>
      )}
    </Dialog>
  );
}
