import { Link, Navigate, useNavigate, useParams } from "@tanstack/react-router";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import type { PersonProfile } from "../../gen/PersonProfile.ts";
import { useStore } from "../../store/store.ts";
import { peoplePages } from "../../sync/admin.ts";
import { directs } from "../../sync/directs.ts";
import { ActionError } from "../../sync/run.ts";
import { Avatar } from "../../ui/avatar.tsx";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { toast } from "../../ui/toast-store.ts";
import { adminFailure, Confirm } from "../admin/admin-parts.tsx";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { svgDataUrl } from "../settings/settings-format.ts";
import { usePresenceStatus } from "./people.ts";
import {
  BAN_CONFIRMATION,
  botPage,
  landBan,
  newerUser,
  OWN_TRANSFER_HINT,
  PRESENCE_LABEL,
  presenceOf,
  TRANSFER_HINT,
  UNBAN_CONFIRMATION,
} from "./people-format.ts";
import { useHasAgentPage } from "./people-page.tsx";
import { UserAvatar } from "./user-avatar.tsx";
import "./people.css";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "missing" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly profile: PersonProfile };

/** A confirmation waiting on the person, as the classic ban button's `turbo_confirm` asks. */
interface Ask {
  readonly title: string;
  readonly message: string;
  readonly label: string;
  readonly danger: boolean;
  readonly run: () => void;
}

const TRANSFER_ID = "person-transfer";

/** The transfer group's Show QR code button, where its dialog hands focus back. */
function qrControl(): HTMLElement | null {
  return document.querySelector<HTMLElement>(`#${TRANSFER_ID} [data-qr]`);
}

/** The status badge: a live presence dot and word, and their status line. */
function StatusBadge({ profile }: { readonly profile: PersonProfile }) {
  const live = usePresenceStatus(profile.user.id);
  const status = profile.status;

  if (status === null) {
    return null;
  }

  const presence = presenceOf(live, status.presence);

  return (
    <p className="person-status" data-presence={presence}>
      <span className="person-status-dot" aria-hidden="true" />
      <span>{PRESENCE_LABEL[presence]}</span>
      {status.statusText === null ? null : (
        <span className="person-status-text text-muted">{status.statusText}</span>
      )}
    </p>
  );
}

/** An administrator's sign-in link for them (or your own): copy, QR code and share. */
function TransferGroup({
  url,
  qrSvg,
  own,
  name,
}: {
  readonly url: string;
  readonly qrSvg: string | null;
  readonly own: boolean;
  readonly name: string;
}) {
  const [qr, setQr] = useState(false);
  const fieldId = useId();
  const canShare = "share" in navigator;

  const copy = () => {
    navigator.clipboard.writeText(url).then(
      () => toast({ title: "Sign-in link copied", tone: "success" }),
      (error: Error) =>
        toast({ title: "Couldn't copy the link", description: error.message, tone: "danger" }),
    );
  };

  const share = () => {
    navigator
      .share({ title: own ? "Your sign-in link" : `${name}'s sign-in link`, url })
      .catch(() => undefined);
  };

  return (
    <section id={TRANSFER_ID} className="person-transfer" aria-label="Sign-in link">
      <label htmlFor={fieldId} className="person-transfer-label">
        {own ? OWN_TRANSFER_HINT : TRANSFER_HINT}
      </label>
      <input
        id={fieldId}
        className="input person-transfer-url"
        value={url}
        readOnly
        onFocus={(event) => event.currentTarget.select()}
      />
      <div className="person-transfer-actions">
        <Button variant="secondary" size="sm" icon="copy" onClick={copy}>
          Copy link
        </Button>
        {qrSvg === null ? null : (
          <Button variant="secondary" size="sm" data-qr onClick={() => setQr(true)}>
            Show QR code
          </Button>
        )}
        {canShare ? (
          <Button variant="secondary" size="sm" icon="arrow-up-right" onClick={share}>
            Share
          </Button>
        ) : null}
      </div>
      {qrSvg === null ? null : (
        <Dialog
          open={qr}
          onOpenChange={setQr}
          size="sm"
          returnFocus={qrControl}
          title="Scan to sign in"
          description={
            own
              ? "Point your phone's camera at the code. Don't show it to anyone else."
              : `Only show this to ${name}: it signs them in.`
          }
        >
          <img className="person-qr" src={svgDataUrl(qrSvg)} alt="QR code for the sign-in link" />
        </Dialog>
      )}
    </section>
  );
}

/** The loaded page: what the classic `users#show` shows this viewer, with its buttons. */
function Profile({
  profile,
  viewerId,
  onChange,
}: {
  readonly profile: PersonProfile;
  readonly viewerId: number | undefined;
  readonly onChange: (update: (current: PersonProfile) => PersonProfile) => void;
}) {
  const navigate = useNavigate();
  const [ask, setAsk] = useState<Ask | null>(null);
  const [savingDnd, setSavingDnd] = useState(false);
  const [messaging, setMessaging] = useState(false);
  const [banning, setBanning] = useState(false);
  const mounted = useRef<AbortController | null>(null);
  const held = useStore((state) => state.users[profile.user.id]);
  // Their user as the newest copy has it: a resync may have brought a later one than this page's.
  const user = newerUser(profile.user, held);
  const own = user.id === viewerId;
  const active = user.status === "active";
  const deactivated = user.status === "deactivated";
  const stored = useStore((state) => state.dndAllowances[profile.user.id]);
  // Their DND exception as the store has it, so a change that finished after you left and came
  // back still shows; the page's own copy only until the store has one.
  const dndAllowed = active && !own ? (stored ?? profile.dndAllowed) : null;

  // Leaving the page stops a ban's follow-up fetches.
  useEffect(() => {
    const controller = new AbortController();

    mounted.current = controller;

    return () => controller.abort();
  }, []);

  const setAllowance = (allowed: boolean) => {
    setSavingDnd(true);
    peoplePages
      .setDndAllowance(user.id, allowed)
      .then(
        () => undefined,
        (error: Error) =>
          toast({
            title: allowed ? "Couldn't allow them during DND" : "Couldn't mute them during DND",
            description: error.message,
            tone: "danger",
          }),
      )
      .finally(() => setSavingDnd(false));
  };

  const message = () => {
    setMessaging(true);
    directs.create([user.id]).then(
      (row) => void navigate({ to: "/r/$roomId", params: { roomId: row.room.id } }),
      (error: Error) => {
        setMessaging(false);
        toast({
          title: `Couldn't message ${user.name}`,
          description: error.message,
          tone: "danger",
        });
      },
    );
  };

  const changeBan = (banned: boolean) => {
    setBanning(true);
    peoplePages
      .setBanned(user.id, banned, mounted.current?.signal)
      .then(
        (next) => {
          // Only what a ban changes: a DND change may have landed after this reply was made.
          onChange((current) => landBan(current, next));
          toast({
            title: banned ? `${user.name} is banned` : `${user.name}'s ban was removed`,
            tone: "success",
          });
        },
        (error: Error) =>
          adminFailure(banned ? `Couldn't ban ${user.name}` : "Couldn't remove the ban", error),
      )
      .finally(() => setBanning(false));
  };

  const askBan = () =>
    setAsk(
      active
        ? {
            title: `Ban ${user.name}?`,
            message: BAN_CONFIRMATION,
            label: `Ban ${user.name}`,
            danger: true,
            run: () => changeBan(true),
          }
        : {
            title: "Remove the ban?",
            message: UNBAN_CONFIRMATION,
            label: "Remove ban",
            danger: false,
            run: () => changeBan(false),
          },
    );

  if (user.role === "bot") {
    return (
      <div className="person" data-status={user.status}>
        <Avatar name={user.name} userId={user.id} src={user.avatarUrl} size={96} />
        <span className="people-badge">Bot</span>
        {active ? (
          <div className="person-actions">
            <Button
              variant="primary"
              icon="message-circle"
              aria-label={`Message ${user.name}`}
              loading={messaging}
              onClick={message}
            >
              Message
            </Button>
            {profile.canManageBot ? (
              <Link
                to="/admin/bots/$botId/grants"
                params={{ botId: `${user.id}` }}
                className="button"
                data-variant="secondary"
              >
                Manage capability grants
              </Link>
            ) : null}
          </div>
        ) : (
          <p className="text-muted">{user.name} is no longer on this account</p>
        )}
      </div>
    );
  }

  if (deactivated) {
    return (
      <div className="person" data-status="deactivated">
        <UserAvatar userId={user.id} size={96} />
        <p className="text-muted">{user.name} is no longer on this account</p>
      </div>
    );
  }

  return (
    <div className="person" data-status={user.status}>
      <UserAvatar userId={user.id} size={96} />
      {user.status === "banned" ? <span className="people-badge">Banned</span> : null}
      {profile.emailAddress === null ? null : (
        <a className="person-email" href={`mailto:${profile.emailAddress}`}>
          {profile.emailAddress}
        </a>
      )}
      {user.bio === null || user.bio === "" ? null : <p className="person-bio">{user.bio}</p>}
      <StatusBadge profile={profile} />
      {active ? (
        <div className="person-actions">
          {dndAllowed === null ? null : (
            <Button
              variant="secondary"
              icon={dndAllowed ? "bell-off" : "bell-ring"}
              aria-pressed={dndAllowed}
              loading={savingDnd}
              onClick={() => setAllowance(!dndAllowed)}
            >
              {dndAllowed ? "Mute during DND" : "Allow during DND"}
            </Button>
          )}
          <Button
            variant="primary"
            icon="message-circle"
            aria-label={`Message ${user.name}`}
            loading={messaging}
            onClick={message}
          >
            Message
          </Button>
        </div>
      ) : null}
      {profile.transferUrl === null || !active ? null : (
        <TransferGroup
          url={profile.transferUrl}
          qrSvg={profile.transferQrSvg}
          own={own}
          name={user.name}
        />
      )}
      {profile.canBan ? (
        <Button
          variant={active ? "secondary" : "danger"}
          icon="ban"
          loading={banning}
          aria-busy={banning}
          onClick={askBan}
        >
          {active ? `Ban ${user.name}` : "Remove ban"}
        </Button>
      ) : null}
      <Confirm ask={ask} onCancel={() => setAsk(null)} />
    </div>
  );
}

/**
 * `/app/people/$userId`: someone's page, as the classic `users#show` shows it to this viewer:
 * their status, the DND exception, Message and, for administrators, their email, sign-in link
 * and the ban button. A bot with an agent record opens its agent profile.
 */
export function PersonPage({
  userId,
  viewerId,
}: {
  readonly userId: number;
  readonly viewerId: number | undefined;
}) {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const hasAgentPage = useHasAgentPage();
  const navigate = useNavigate();
  const left = useRef(false);
  const generation = useRef(0);
  const loading = useRef<AbortController | null>(null);

  const fetchProfile = useCallback(() => {
    const mine = ++generation.current;
    const controller = new AbortController();

    loading.current?.abort();
    loading.current = controller;
    peoplePages.profile(userId, controller.signal).then(
      (profile) => {
        if (mine === generation.current) setLoad({ status: "ready", profile });
      },
      (error: Error) => {
        if (mine !== generation.current) return;

        setLoad(
          error instanceof ActionError && error.tag === "NotFound"
            ? { status: "missing" }
            : { status: "error", message: error.message },
        );
      },
    );
  }, [userId]);

  useEffect(() => {
    fetchProfile();

    // Leaving (or Strict Mode's rehearsal) drops the reply of the load in flight and stops it
    // fetching again.
    return () => {
      generation.current++;
      loading.current?.abort();
    };
  }, [fetchProfile]);

  /** A change from the page outranks any load still in flight. */
  const change = (update: (current: PersonProfile) => PersonProfile) => {
    generation.current++;
    setLoad((current) =>
      current.status === "ready" ? { status: "ready", profile: update(current.profile) } : current,
    );
  };

  const bot =
    load.status === "ready" &&
    load.profile.user.role === "bot" &&
    load.profile.user.agent !== null &&
    hasAgentPage
      ? load.profile.user
      : null;

  useEffect(() => {
    if (bot === null || left.current) {
      return;
    }

    left.current = true;

    const target = botPage(bot, hasAgentPage);

    void navigate({ href: target.slice("/app".length), replace: true });
  }, [bot, hasAgentPage, navigate]);

  const heldUser = useStore((state) => state.users[userId]);
  const title = load.status === "ready" ? newerUser(load.profile.user, heldUser).name : "Person";
  const identity = load.status === "ready" ? newerUser(load.profile.user, heldUser) : null;
  const own = load.status === "ready" && load.profile.user.id === viewerId;

  return (
    <PageFrame
      title={title}
      meta={
        identity?.pronouns ? <span className="text-muted">{identity.pronouns}</span> : undefined
      }
      icon="users"
      back
      tools={
        own ? (
          <Link to="/settings" className="button" data-variant="secondary" data-size="sm">
            Edit my profile
          </Link>
        ) : undefined
      }
    >
      <div className="people-page">
        {identity !== null && identity.accountName !== identity.name ? (
          <p className="text-muted">{identity.accountName}</p>
        ) : null}
        {load.status === "loading" || bot !== null ? (
          <PaneListSkeleton rows={2} square={96} />
        ) : null}
        {load.status === "missing" ? (
          <PaneError message="There's nobody here by that link." />
        ) : null}
        {load.status === "error" ? (
          <PaneError
            message={load.message}
            onRetry={() => {
              setLoad({ status: "loading" });
              fetchProfile();
            }}
          />
        ) : null}
        {load.status === "ready" && bot === null ? (
          <Profile profile={load.profile} viewerId={viewerId} onChange={change} />
        ) : null}
      </div>
    </PageFrame>
  );
}

/** The route: the person in the URL, seen by whoever is signed in. */
export function PersonRoute() {
  const { userId } = useParams({ from: "/shell/people/$userId" });
  const viewerId = useStore((state) => state.me?.user.id);

  if (userId === viewerId) {
    return <Navigate to="/settings" replace />;
  }

  return <PersonPage key={userId} userId={userId} viewerId={viewerId} />;
}
