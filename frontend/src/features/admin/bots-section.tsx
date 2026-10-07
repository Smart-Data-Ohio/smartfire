import { Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import type { BotList } from "../../gen/BotList.ts";
import type { BotSummary } from "../../gen/BotSummary.ts";
import { bots } from "../../sync/admin.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { SettingsGroup, SettingsPage } from "../settings/settings-parts.tsx";
import { AdministratorsOnly, useAdmin } from "./admin-parts.tsx";
import { BotPicture, CopyLine } from "./bot-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: BotList };

/** One bot: its picture, name and owner, the way to its page, and the commands that post as it. */
function BotRow({ bot }: { readonly bot: BotSummary }) {
  return (
    <li className="settings-list-row admin-bot-row">
      <BotPicture id={bot.id} name={bot.name} avatarUrl={bot.avatarUrl} icon={bot.icon} />
      <span className="settings-list-main">
        <strong>{bot.name}</strong>
        <span className="text-faint">{bot.ownership}</span>
      </span>
      <Link
        to="/admin/bots/$botId"
        params={{ botId: `${bot.id}` }}
        className="button"
        data-variant="secondary"
        data-size="sm"
      >
        Edit
        <span className="visually-hidden"> {bot.name}</span>
      </Link>
      {bot.rooms.length === 0 ? null : (
        <details className="admin-bot-rooms">
          <summary>
            Post to {bot.rooms.length === 1 ? "its room" : `its ${bot.rooms.length} rooms`}
          </summary>
          <p className="settings-hint text-faint">
            Replace <code className="admin-code">BOT_KEY</code> with the bot's key, shown once when
            the bot was created or its key was last generated.
          </p>
          {bot.rooms.map((room) => (
            <div key={room.id} className="admin-secret">
              <strong>{room.name}</strong>
              <CopyLine text={room.messageCommand} what="Command" />
              <CopyLine text={room.attachmentCommand} what="Command" />
            </div>
          ))}
        </details>
      )}
    </li>
  );
}

/**
 * Chat bots: the active bots, as the classic list shows them, with the commands that post to each
 * room as the bot. A bot's page edits it; New bot makes one.
 */
export function BotsSection() {
  const { workspace } = useAdmin();
  const [load, setLoad] = useState<Load>({ status: "loading" });

  const fetchBots = useCallback(() => {
    bots.list().then(
      (list) => setLoad({ status: "ready", list }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(() => {
    if (workspace.canAdminister) fetchBots();
  }, [fetchBots, workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const reload = () => {
    setLoad({ status: "loading" });
    fetchBots();
  };

  return (
    <SettingsPage
      title="Chat bots"
      description="With Chat bots, other sites and services can post updates directly to Smartfire."
    >
      <div className="settings-actions">
        <Link to="/admin/bots/new" className="button" data-variant="primary">
          <Icon name="plus" size={16} />
          New bot
        </Link>
      </div>
      <SettingsGroup title="Bots">
        {load.status === "loading" ? <PaneListSkeleton rows={3} /> : null}
        {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
        {load.status === "ready" && load.list.bots.length === 0 ? (
          <p className="text-muted">No chat bots yet.</p>
        ) : null}
        {load.status === "ready" && load.list.bots.length > 0 ? (
          <ul className="settings-list">
            {load.list.bots.map((bot) => (
              <BotRow key={bot.id} bot={bot} />
            ))}
          </ul>
        ) : null}
      </SettingsGroup>
    </SettingsPage>
  );
}
