import { Link } from "@tanstack/react-router";
import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import type { WorkspaceInvite } from "../../gen/WorkspaceInvite.ts";
import type { WorkspaceInviteCreated } from "../../gen/WorkspaceInviteCreated.ts";
import type { WorkspaceInviteExpiry } from "../../gen/WorkspaceInviteExpiry.ts";
import type { WorkspaceInviteState } from "../../gen/WorkspaceInviteState.ts";
import { admin } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { Tabs } from "../../ui/tabs.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import {
  SettingsGroup,
  SettingsPage,
  SettingsSelect,
  useBusy,
} from "../settings/settings-parts.tsx";
import { useNow } from "../threads/use-now.ts";
import { auditTime } from "./admin-format.ts";
import {
  AdministratorsOnly,
  adminFailure,
  CardCell,
  CardRow,
  CardTable,
  Confirm,
  copy,
  useAdmin,
  useRowFocus,
} from "./admin-parts.tsx";
import {
  DEFAULT_EXPIRY,
  EXPIRY_CHOICES,
  emptyLine,
  expiresLabel,
  FILTERS,
  filterInvites,
  type InviteFilter,
  inviteState,
  linkTerms,
  maxUsesOf,
  revokeConfirmation,
  STATE_LABEL,
  shortTime,
  USE_CHOICES,
  type UseLimit,
  usesLabel,
  withMade,
} from "./invite-format.ts";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly invites: readonly WorkspaceInvite[] };

/** Each state's mark beside its name. */
const STATE_ICON: Readonly<Record<WorkspaceInviteState, IconName>> = {
  active: "circle-check",
  expired: "hourglass",
  exhausted: "users",
  revoked: "ban",
};

/** A new invite, with the expiry it was made with, for the link's dialog to say. */
type Created = WorkspaceInviteCreated & { readonly expiry: WorkspaceInviteExpiry };

const COLUMNS = ["Created by", "Created", "Expires", "Uses", "Status", "Actions"] as const;

/** The new-invite form: Discord's "Expire after" and "Max number of uses". */
function NewInvite({ onCreated }: { readonly onCreated: (created: Created) => void }) {
  const [expiry, setExpiry] = useState<WorkspaceInviteExpiry>(DEFAULT_EXPIRY);
  const [limit, setLimit] = useState<UseLimit>("");
  const { busy, track } = useBusy();

  const submit = (event: FormEvent) => {
    event.preventDefault();
    void track(
      "create",
      admin.createInvite({ expiry, maxUses: maxUsesOf(limit) }).then(
        (reply) => onCreated({ ...reply, expiry }),
        (error: Error) => adminFailure("Couldn't create the invite", error),
      ),
    );
  };

  return (
    <form className="settings-form admin-invite-form" onSubmit={submit}>
      <div className="admin-invite-choices">
        <SettingsSelect
          label="Expire after"
          value={expiry}
          choices={EXPIRY_CHOICES}
          onChange={setExpiry}
          disabled={busy("create")}
        />
        <SettingsSelect
          label="Max number of uses"
          value={limit}
          choices={USE_CHOICES}
          onChange={setLimit}
          disabled={busy("create")}
        />
      </div>
      <p className="settings-hint text-faint" aria-live="polite">
        {linkTerms(expiry, maxUsesOf(limit))}
      </p>
      <div className="settings-actions">
        <Button
          type="submit"
          variant="primary"
          icon="link"
          loading={busy("create")}
          loadingLabel="Creating…"
        >
          Create invite link
        </Button>
      </div>
    </form>
  );
}

/** The link and its Copy button, which says when it has copied (a new link starts over). */
function InviteLink({ created }: { readonly created: Created }) {
  const shown = useRef<HTMLElement>(null);
  const [copied, setCopied] = useState(false);

  return (
    <div className="admin-invite-link">
      <div className="admin-invite-link-box" data-copied={copied || undefined}>
        <code ref={shown} className="admin-invite-link-url">
          {created.url}
        </code>
        <Button
          variant="primary"
          icon={copied ? "check" : "copy"}
          data-autofocus
          onClick={() =>
            void copy(created.url, "Invite link", shown.current).then((done) => {
              if (done) setCopied(true);
            })
          }
        >
          {copied ? "Copied" : "Copy link"}
        </Button>
      </div>
      <p className="settings-hint text-faint">
        {linkTerms(created.expiry, created.invite.maxUses)} Send it only to the people you mean to
        invite.
      </p>
    </div>
  );
}

/** A new invite's link, shown this once, with the way to copy it. */
function InviteLinkDialog({
  created,
  onClose,
}: {
  readonly created: Created | null;
  readonly onClose: () => void;
}) {
  return (
    <Dialog
      open={created !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      role="alertdialog"
      title="Your invite link"
      description="Copy it now: it won't be shown again. You can always create another."
      footer={
        <Button variant="secondary" onClick={onClose}>
          Done
        </Button>
      }
    >
      {created === null ? null : <InviteLink key={created.url} created={created} />}
    </Dialog>
  );
}

/** Where an invite stands, said in words with a mark: never by colour alone. */
function StateBadge({
  invite,
  state,
  now,
}: {
  readonly invite: WorkspaceInvite;
  readonly state: WorkspaceInviteState;
  readonly now: number;
}) {
  return (
    <span className="admin-invite-state-cell">
      <span className="admin-invite-state" data-state={state}>
        <Icon name={STATE_ICON[state]} size={12} />
        {STATE_LABEL[state]}
      </span>
      {state === "revoked" && invite.revokedAt !== null ? (
        <time
          className="text-faint"
          dateTime={invite.revokedAt}
          title={auditTime(invite.revokedAt)}
        >
          {shortTime(invite.revokedAt, now)}
        </time>
      ) : null}
    </span>
  );
}

/** One invite as a table row (a card on phones). Only a live invite can be revoked. */
function InviteRow({
  invite,
  now,
  busy,
  onRevoke,
}: {
  readonly invite: WorkspaceInvite;
  readonly now: number;
  readonly busy: boolean;
  readonly onRevoke: (invite: WorkspaceInvite) => void;
}) {
  const state = inviteState(invite, now);
  const label = `invite by ${invite.creator.name}, created ${auditTime(invite.createdAt)}`;

  return (
    <CardRow row={invite.id} state={state}>
      <CardCell column="Created by" title>
        {invite.creator.name}
      </CardCell>
      <CardCell column="Created">
        <time dateTime={invite.createdAt} title={auditTime(invite.createdAt)}>
          {shortTime(invite.createdAt, now)}
        </time>
      </CardCell>
      <CardCell column="Expires">
        {invite.expiresAt === null ? (
          "Never"
        ) : (
          <time dateTime={invite.expiresAt} title={auditTime(invite.expiresAt)}>
            {expiresLabel(invite, now, (at) => shortTime(at, now))}
          </time>
        )}
      </CardCell>
      <CardCell column="Uses">{usesLabel(invite)}</CardCell>
      <CardCell column="Status">
        <StateBadge invite={invite} state={state} now={now} />
      </CardCell>
      <CardCell column="Actions" className="admin-invite-actions">
        {state === "active" ? (
          <Button
            variant="ghost"
            size="sm"
            className="admin-invite-revoke"
            data-row-control="revoke"
            disabled={busy}
            loading={busy}
            onClick={() => onRevoke(invite)}
          >
            Revoke
            <span className="visually-hidden"> {label}</span>
          </Button>
        ) : null}
      </CardCell>
    </CardRow>
  );
}

/**
 * Where focus goes when the revoke confirmation for invite `id` closes: its Revoke button while it
 * can take focus (a cancel), else its row (the revoke is running or done, and the button is
 * disabled or gone). `null` when the row left the list too, for the row focus to land nearby.
 */
function revokeFocus(root: HTMLElement | null, id: number | null): HTMLElement | null {
  const row = id === null ? null : (root?.querySelector<HTMLElement>(`[data-row="${id}"]`) ?? null);
  const button = row?.querySelector<HTMLElement>("[data-row-control='revoke']:not(:disabled)");

  return button ?? row;
}

/**
 * `/app/admin/invites`: private invite links, Discord's server invites. An administrator makes a
 * link that expires and may be used only so many times, sees it once to copy, and can revoke it;
 * the list shows every invite with who made it, its uses and where it stands. The shared join
 * link and the vanity invite stay on the workspace page, which this page points to.
 */
export function InvitesSection() {
  const { workspace } = useAdmin();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [filter, setFilter] = useState<InviteFilter>("all");
  const [created, setCreated] = useState<Created | null>(null);
  const [asking, setAsking] = useState<WorkspaceInvite | null>(null);
  const { busy, track } = useBusy();
  const container = useRowFocus();
  const now = useNow();

  /** Invites made on this page, newest first: kept when the list answers from before them. */
  const made = useRef<readonly WorkspaceInvite[]>([]);
  /** The invite whose revoke was last asked: focus goes back to its row. */
  const revoking = useRef<number | null>(null);

  const fetchInvites = useCallback(() => {
    admin.invites().then(
      ({ invites }) => setLoad({ status: "ready", invites: withMade(made.current, invites) }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(() => {
    if (workspace.canAdminister) fetchInvites();
  }, [fetchInvites, workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const reload = () => {
    setLoad({ status: "loading" });
    fetchInvites();
  };

  const replace = (next: WorkspaceInvite) =>
    setLoad((current) =>
      current.status === "ready"
        ? {
            status: "ready",
            invites: current.invites.map((invite) => (invite.id === next.id ? next : invite)),
          }
        : current,
    );

  const onCreated = (next: Created) => {
    // Still loading, the list may answer from before this invite: it is merged in then.
    made.current = [next.invite, ...made.current];
    setLoad((current) =>
      current.status === "ready"
        ? { status: "ready", invites: withMade([next.invite], current.invites) }
        : current,
    );
    // The new invite is live: a filter that would hide it goes back to all.
    setFilter((current) => (current === "all" || current === "active" ? current : "all"));
    setCreated(next);
  };

  const revoke = (invite: WorkspaceInvite) =>
    void track(
      `invite-${invite.id}`,
      admin.revokeInvite(invite.id).then(
        (next) => {
          // Its Revoke button goes away; the row keeps the focus (the confirmation's returnFocus).
          replace(next);
          toast({ title: "Invite revoked", tone: "success" });
        },
        (error: Error) => adminFailure("Couldn't revoke the invite", error),
      ),
    );

  const invites = load.status === "ready" ? filterInvites(load.invites, filter, now) : [];

  return (
    <SettingsPage
      title="Invites"
      description="Private links that expire or stop after a set number of people join."
    >
      <SettingsGroup title="New invite">
        <NewInvite onCreated={onCreated} />
      </SettingsGroup>
      <SettingsGroup title="Invites" description="Newest first. Links are shown only when created.">
        <div ref={container} tabIndex={-1} className="admin-focus-root admin-invites">
          {load.status === "loading" ? <PaneListSkeleton rows={3} /> : null}
          {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
          {load.status === "ready" && load.invites.length > 0 ? (
            <div className="admin-invite-filter">
              <Tabs
                label="Show invites"
                items={FILTERS}
                value={filter}
                onValueChange={(value) => {
                  const picked = FILTERS.find((each) => each.value === value);

                  if (picked !== undefined) setFilter(picked.value);
                }}
              />
            </div>
          ) : null}
          {load.status === "ready" && invites.length === 0 ? (
            <p className="text-muted">{emptyLine(load.invites.length === 0 ? "all" : filter)}</p>
          ) : null}
          {invites.length > 0 ? (
            <section className="admin-audit-wrap" aria-label="Invites">
              <CardTable caption="Invites, newest first" columns={COLUMNS}>
                {invites.map((invite) => (
                  <InviteRow
                    key={invite.id}
                    invite={invite}
                    now={now}
                    busy={busy(`invite-${invite.id}`)}
                    onRevoke={(invite) => {
                      revoking.current = invite.id;
                      setAsking(invite);
                    }}
                  />
                ))}
              </CardTable>
            </section>
          ) : null}
        </div>
      </SettingsGroup>
      <SettingsGroup
        title="Shared join link"
        description="Everyone in the workspace can see and share the join link. It never expires; making a new one turns the old one off."
      >
        <Link to="/admin/workspace" className="settings-link admin-invite-shared">
          <Icon name="home" size={14} />
          The join link and vanity invite are on the Workspace page
        </Link>
      </SettingsGroup>
      <InviteLinkDialog created={created} onClose={() => setCreated(null)} />
      <Confirm
        ask={
          asking === null
            ? null
            : {
                title: "Revoke this invite?",
                message: revokeConfirmation(asking),
                label: "Revoke invite",
                danger: true,
                run: () => revoke(asking),
              }
        }
        onCancel={() => setAsking(null)}
        returnFocus={() => revokeFocus(container.current, revoking.current)}
      />
    </SettingsPage>
  );
}
