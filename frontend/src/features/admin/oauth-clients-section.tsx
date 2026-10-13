import { useCallback, useEffect, useState } from "react";
import type { AdminOAuthClient } from "../../gen/AdminOAuthClient.ts";
import type { AdminOAuthClientList } from "../../gen/AdminOAuthClientList.ts";
import { oauthClients } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { SettingsGroup } from "../settings/settings-parts.tsx";
import { timeAgo } from "../threads/thread-format.ts";
import { Confirm } from "./admin-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: AdminOAuthClientList };

type Ask = Parameters<typeof Confirm>[0]["ask"];

/** "2 connected · last connected 3 days ago", or "Blocked 1 hour ago · …". */
export function clientMeta(client: AdminOAuthClient, now: number): string {
  const parts = [
    client.liveConnections === 1 ? "1 connected" : `${client.liveConnections} connected`,
  ];

  if (client.lastConnectedAt !== null) {
    parts.push(`last connected ${timeAgo(client.lastConnectedAt, now)}`);
  }

  if (client.blockedAt !== null) parts.unshift(`Blocked ${timeAgo(client.blockedAt, now)}`);

  return parts.join(" · ");
}

/** One client: its name, where it lives, how many connect through it, and Block or Unblock. */
function ClientRow({
  client,
  busy,
  onToggle,
}: {
  readonly client: AdminOAuthClient;
  readonly busy: boolean;
  readonly onToggle: () => void;
}) {
  const blocked = client.blockedAt !== null;

  return (
    <li className="settings-list-row" data-blocked={blocked || undefined}>
      <span className="settings-list-main">
        <strong>
          {client.clientName}
          {blocked ? <span className="settings-badge">Blocked</span> : null}
        </strong>
        <span className="text-faint settings-endpoint">{client.clientHost}</span>
        <span className="text-faint">{clientMeta(client, Date.now())}</span>
      </span>
      <Button
        variant={blocked ? "secondary" : "danger"}
        size="sm"
        loading={busy}
        onClick={onToggle}
        aria-label={`${blocked ? "Unblock" : "Block"} ${client.clientName}`}
      >
        {blocked ? "Unblock" : "Block"}
      </Button>
    </li>
  );
}

/**
 * Chat bots → Connected apps: every app that connected an agent by signing in (OAuth), and
 * blocking one, which stops its connections at once and keeps it from connecting again until
 * it's unblocked. Hidden while the server has no OAuth and nothing ever connected.
 */
export function ConnectedAppsGroup() {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [ask, setAsk] = useState<Ask>(null);
  const [busy, setBusy] = useState<ReadonlySet<number>>(new Set());

  const fetchClients = useCallback(() => {
    oauthClients.list().then(
      (list) => setLoad({ status: "ready", list }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchClients, [fetchClients]);

  if (load.status === "ready" && !load.list.oauthAvailable && load.list.clients.length === 0) {
    return null;
  }

  const mark = (id: number, on: boolean) =>
    setBusy((current) => {
      const next = new Set(current);

      if (on) next.add(id);
      else next.delete(id);

      return next;
    });

  const write = (client: AdminOAuthClient, blocked: boolean) => {
    mark(client.id, true);
    oauthClients
      .setBlocked(client.id, blocked)
      .then(
        (answer) => {
          setLoad((current) =>
            current.status === "ready"
              ? {
                  status: "ready",
                  list: {
                    ...current.list,
                    clients: current.list.clients.map((c) => (c.id === answer.id ? answer : c)),
                  },
                }
              : current,
          );
          toast({
            title: blocked
              ? `${answer.clientName} was blocked`
              : `${answer.clientName} was unblocked`,
            tone: "success",
          });
        },
        (error: Error) =>
          toast({
            title: `Couldn't ${blocked ? "block" : "unblock"} ${client.clientName}`,
            description: error.message,
            tone: "danger",
          }),
      )
      .finally(() => mark(client.id, false));
  };

  const toggle = (client: AdminOAuthClient) => {
    if (client.blockedAt !== null) {
      write(client, false);

      return;
    }

    setAsk({
      title: `Block ${client.clientName}?`,
      message: `Everyone's agents connected through ${client.clientName} stop working at once, and nobody can connect it again until you unblock it.`,
      label: "Block",
      danger: true,
      run: () => write(client, true),
    });
  };

  const reload = () => {
    setLoad({ status: "loading" });
    fetchClients();
  };

  return (
    <SettingsGroup
      title="Connected apps"
      description="Apps people connected their personal agents through by signing in, such as claude.ai or ChatGPT. Block one to cut off every connection through it."
    >
      {load.status === "loading" ? <PaneListSkeleton rows={2} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" && load.list.clients.length === 0 ? (
        <p className="text-muted">No apps have connected yet.</p>
      ) : null}
      {load.status === "ready" && load.list.clients.length > 0 ? (
        <ul className="settings-list" aria-label="Connected apps">
          {load.list.clients.map((client) => (
            <ClientRow
              key={client.id}
              client={client}
              busy={busy.has(client.id)}
              onToggle={() => toggle(client)}
            />
          ))}
        </ul>
      ) : null}
      <Confirm ask={ask} onCancel={() => setAsk(null)} />
    </SettingsGroup>
  );
}
