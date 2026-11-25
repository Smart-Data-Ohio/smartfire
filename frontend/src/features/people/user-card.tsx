import { Link, useLocation, useNavigate } from "@tanstack/react-router";
import {
  type KeyboardEvent,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { CustomStatus } from "../../gen/CustomStatus.ts";
import type { PersonProfile } from "../../gen/PersonProfile.ts";
import type { Presence } from "../../gen/Presence.ts";
import type { Placement } from "../../lib/anchor.ts";
import { showPopover } from "../../lib/popover.ts";
import { duringAppFocus } from "../../lib/reader-focus.ts";
import { usePresence } from "../../motion/presence.ts";
import { useStore } from "../../store/store.ts";
import { peoplePages } from "../../sync/admin.ts";
import { directs } from "../../sync/directs.ts";
import { Button } from "../../ui/button.tsx";
import { originFor, useFloating } from "../../ui/floating.ts";
import { toast } from "../../ui/toast-store.ts";
import { useWorkingPresence } from "../agents/working.ts";
import { YOU_TAG } from "../directs/note-to-self.ts";
import { useViewerId } from "../messages/use-message.ts";
import { AgentBadge } from "./agent-badge.tsx";
import { agentKindLabel, agentTone, identityOf, toneLabel } from "./agent-identity.ts";
import { UNKNOWN_NAME, useIgnored, usePresenceStatus, useUser } from "./people.ts";
import { presenceOf } from "./people-format.ts";
import { ProfileHero } from "./profile-banner.tsx";
import { UserAvatar } from "./user-avatar.tsx";
import {
  type CardVia,
  cardEnter,
  cardLeave,
  closeUserCard,
  useUserCard,
} from "./user-card-store.ts";
import "../../ui/floating.css";
import "../../ui/popover.css";
import "./user-card.css";

/** The words beside a person's presence dot. */
const PRESENCE_WORDS = {
  online: "Active",
  idle: "Away",
  dnd: "Do not disturb",
  offline: "Offline",
} as const satisfies Record<Presence, string>;

/** How long a fetched profile (an administrator's view of the email, the status line) stays fresh. */
const PROFILE_TTL_MS = 60_000;

const profiles = new Map<number, { readonly at: number; readonly profile: PersonProfile }>();

function freshProfile(userId: number): PersonProfile | null {
  const hit = profiles.get(userId);

  return hit !== undefined && Date.now() - hit.at < PROFILE_TTL_MS ? hit.profile : null;
}

/** Forgets the fetched profiles (tests). */
export function clearUserCardProfiles(): void {
  profiles.clear();
}

/**
 * Their profile as the people page reads it, fetched quietly once the card is up: the card draws
 * from the store at once and this only adds what the store lacks (the email an administrator
 * sees, a status line not yet in presence, or the whole person when the store has never seen
 * them). A failure leaves the card as it is.
 */
function useCardProfile(userId: number): PersonProfile | null {
  const [loaded, setLoaded] = useState<PersonProfile | null>(() => freshProfile(userId));

  useEffect(() => {
    const fresh = freshProfile(userId);

    if (fresh !== null) {
      setLoaded(fresh);

      return;
    }

    const controller = new AbortController();

    peoplePages.profile(userId, controller.signal).then(
      (profile) => {
        profiles.set(userId, { at: Date.now(), profile });

        if (!controller.signal.aborted) {
          setLoaded(profile);
        }
      },
      () => undefined,
    );

    return () => controller.abort();
  }, [userId]);

  return loaded;
}

/** A custom status's emoji and words, or `null` when it has none or it has run out. */
export function customStatusLine(status: CustomStatus | null, now: number): string | null {
  if (status === null || (status.expiresAt !== null && Date.parse(status.expiresAt) <= now)) {
    return null;
  }

  const line = [status.emoji, status.text]
    .flatMap((part) => {
      const trimmed = part?.trim() ?? "";

      return trimmed === "" ? [] : [trimmed];
    })
    .join(" ");

  return line === "" ? null : line;
}

/** A bot's or agent's line: its kind and what it's doing, as the room header shows it. */
function AgentLine({ userId }: { readonly userId: number }) {
  const identity = identityOf(useUser(userId));
  const working = useWorkingPresence(userId);

  if (identity.kind !== "agent") {
    return <p className="user-card-status text-muted">Bot</p>;
  }

  const tone = agentTone(identity) ?? "idle";

  return (
    <p className="user-card-status user-card-agent" data-tone={tone}>
      {agentKindLabel(identity.agentKind)} · {working ?? toneLabel(tone, identity.status)}
    </p>
  );
}

/** A person's presence dot and word, and their status line. */
function PresenceLine({
  userId,
  customStatus,
  profile,
}: {
  readonly userId: number;
  readonly customStatus: CustomStatus | null;
  readonly profile: PersonProfile | null;
}) {
  const live = usePresenceStatus(userId);
  const liveText = useStore((state) => state.presence[userId]?.statusText ?? null);
  const loaded = profile?.status ?? null;

  const presence =
    live === undefined && loaded === null ? null : presenceOf(live, loaded?.presence ?? "offline");

  const text = liveText ?? loaded?.statusText ?? customStatusLine(customStatus, Date.now());

  if (presence === null && text === null) {
    return null;
  }

  return (
    <div className="user-card-presence">
      {presence === null ? null : (
        <p className="user-card-status" data-presence={presence}>
          <span className="user-card-dot" aria-hidden="true" />
          {PRESENCE_WORDS[presence]}
        </p>
      )}
      {text === null ? null : <p className="user-card-status-text">{text}</p>}
    </div>
  );
}

interface CardProps {
  readonly userId: number;
  readonly titleId: string;
}

/** Where "View profile" goes: your settings, an agent's profile, or their person page. */
function ProfileLink({ userId, own }: { readonly userId: number; readonly own: boolean }) {
  const identity = identityOf(useUser(userId));

  if (own) {
    return (
      <Link
        to="/settings"
        className="button"
        data-variant="secondary"
        data-size="sm"
        onClick={closeUserCard}
      >
        Edit profile
      </Link>
    );
  }

  return identity.kind === "agent" ? (
    <Link
      to="/agents/$agentId"
      params={{ agentId: identity.agentId }}
      className="button"
      data-variant="secondary"
      data-size="sm"
      preload={false}
      onClick={closeUserCard}
    >
      View profile
    </Link>
  ) : (
    <Link
      to="/people/$userId"
      params={{ userId }}
      className="button"
      data-variant="secondary"
      data-size="sm"
      preload={false}
      onClick={closeUserCard}
    >
      View profile
    </Link>
  );
}

/**
 * Someone's card, drawn from the store straight away: their banner and avatar, name (with the
 * account name when a nickname differs, and pronouns), badges, presence or agent status, the email
 * an administrator sees, their bio, and Message and View profile.
 */
function UserCard({ userId, titleId }: CardProps) {
  const navigate = useNavigate();
  const viewerId = useViewerId();
  const held = useUser(userId);
  const profile = useCardProfile(userId);
  const user = held ?? profile?.user;
  const ignoredNow = useIgnored(userId, profile?.ignoredByMe);
  const [messaging, setMessaging] = useState(false);
  const name = user?.name ?? UNKNOWN_NAME;
  const own = userId === viewerId;
  const ignored = !own && ignoredNow;
  const status = user?.status;
  const bot = user?.role === "bot";
  const bio = user?.bio?.trim() ?? "";

  const message = () => {
    setMessaging(true);
    // Nobody else opens your note-to-self, as the sidebar's "Message yourself" does.
    directs.create(own ? [] : [userId]).then(
      (row) => {
        closeUserCard();
        void navigate({ to: "/r/$roomId", params: { roomId: row.room.id } });
      },
      (error: Error) => {
        setMessaging(false);
        toast({
          title: own ? "Couldn't open your notes" : `Couldn't message ${name}`,
          description: error.message,
          tone: "danger",
        });
      },
    );
  };

  return (
    <div className="user-card" data-status={status}>
      <ProfileHero
        userId={userId}
        name={name}
        url={status === "deactivated" ? null : (user?.bannerUrl ?? null)}
        stillUrl={status === "deactivated" ? null : (user?.bannerStillUrl ?? null)}
        size="card"
      >
        <UserAvatar userId={userId} size={64} presence={!bot} live decorative />
      </ProfileHero>
      <div className="user-card-body">
        <div className="user-card-identity">
          <h2 id={titleId} className="user-card-name">
            {name}
            {own ? <span className="user-card-you"> {YOU_TAG}</span> : null}
          </h2>
          {user !== undefined && user.accountName !== user.name ? (
            <p className="user-card-account">{user.accountName}</p>
          ) : null}
          {user?.pronouns ? <p className="user-card-pronouns">{user.pronouns}</p> : null}
        </div>
        <div className="user-card-badges">
          <AgentBadge userId={userId} status />
          {user?.role === "administrator" ? (
            <span className="user-card-badge" data-tone="admin">
              Admin
            </span>
          ) : null}
          {ignored ? <span className="user-card-badge">Ignored</span> : null}
          {status === "banned" ? (
            <span className="user-card-badge" data-tone="danger">
              Banned
            </span>
          ) : null}
          {status === "deactivated" && !bot ? (
            <span className="user-card-badge">Deactivated</span>
          ) : null}
        </div>
        {status === "deactivated" ? (
          <p className="user-card-status text-muted">No longer on this account</p>
        ) : bot ? (
          <AgentLine userId={userId} />
        ) : user === undefined ? null : (
          <PresenceLine userId={userId} customStatus={user.customStatus} profile={profile} />
        )}
        {profile?.emailAddress == null ? null : (
          <a className="user-card-email" href={`mailto:${profile.emailAddress}`}>
            {profile.emailAddress}
          </a>
        )}
        {bio === "" || status === "deactivated" ? null : <p className="user-card-bio">{bio}</p>}
      </div>
      <div className="user-card-actions">
        {status === "active" ? (
          <Button
            variant="primary"
            size="sm"
            icon="message-circle"
            loading={messaging}
            onClick={message}
          >
            {own ? "Message yourself" : "Message"}
          </Button>
        ) : null}
        <ProfileLink userId={userId} own={own} />
      </div>
    </div>
  );
}

/** What the card last showed: kept through its exit so it doesn't go blank as it fades. */
interface Shown {
  readonly userId: number;
  readonly anchor: HTMLElement;
  readonly placement: Placement;
  readonly via: CardVia;
}

/**
 * The one user card, mounted once by the app shell and opened from any avatar or name
 * (`UserCardTrigger`, `UserCardHover`, mentions in message HTML). It sits in the top layer beside
 * its trigger (a manual popover with CSS anchor positioning), and closes on Esc, a press outside
 * it, a scroll under it, navigation, or its trigger going away. Opened by a click, it takes focus
 * and gives it back to the trigger on Esc.
 */
export function UserCardHost() {
  const card = useUserCard();
  const titleId = useId();
  const floatingId = useId();
  const visible = card.kind === "open" || card.kind === "closing";
  const [shown, setShown] = useState<Shown | null>(null);
  const pathname = useLocation({ select: (location) => location.pathname });
  const lastPath = useRef(pathname);

  if (
    visible &&
    (shown === null ||
      shown.userId !== card.userId ||
      shown.anchor !== card.anchor ||
      shown.placement !== card.placement ||
      shown.via !== card.via)
  ) {
    setShown({
      userId: card.userId,
      anchor: card.anchor,
      placement: card.placement,
      via: card.via,
    });
  }

  const presence = usePresence<HTMLDivElement>(visible);
  const anchorRef = useMemo(() => ({ current: shown?.anchor ?? null }), [shown?.anchor]);
  const placement = shown?.placement ?? "bottom-start";
  const via = shown?.via;
  const shownUserId = shown?.userId;

  useFloating(floatingId, anchorRef, presence.ref, placement, presence.mounted && shown !== null);

  useLayoutEffect(() => {
    const surface = presence.ref.current;

    if (visible && surface !== null) {
      showPopover(surface);
    }
  }, [visible, presence.ref]);

  // Opened by a click (or a key), focus goes in, onto Message or else View profile.
  // biome-ignore lint/correctness/useExhaustiveDependencies: a click on someone else's trigger swaps the card's contents, and focus moves into the new ones
  useLayoutEffect(() => {
    const surface = presence.ref.current;

    if (!visible || via !== "click" || surface === null) {
      return;
    }

    surface
      .querySelector<HTMLElement>(".user-card-actions :is(button, a[href])")
      ?.focus({ preventScroll: true });
  }, [visible, via, shownUserId, presence.ref]);

  useEffect(() => {
    if (lastPath.current !== pathname) {
      lastPath.current = pathname;
      closeUserCard();
    }
  }, [pathname]);

  useEffect(() => {
    const anchor = shown?.anchor;

    if (!visible || anchor === undefined) {
      return;
    }

    if (!anchor.isConnected) {
      closeUserCard();

      return;
    }

    const inCard = (target: EventTarget | null) =>
      target instanceof Node && (presence.ref.current?.contains(target) ?? false);

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target;

      // A press on the trigger is the trigger's own (a click toggles the card).
      if (!inCard(target) && !(target instanceof Node && anchor.contains(target))) {
        closeUserCard();
      }
    };

    // A hover-opened card has no focus to catch the key, so Esc anywhere closes it.
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        closeUserCard();
      }
    };

    // Whatever scrolled under the trigger would leave the card floating over the wrong thing.
    const onScroll = (event: Event) => {
      const target = event.target;
      const scroller = target === document ? document.documentElement : target;

      if (!inCard(target) && scroller instanceof Node && scroller.contains(anchor)) {
        closeUserCard();
      }
    };

    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("scroll", onScroll, true);

    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("scroll", onScroll, true);
    };
  }, [visible, shown?.anchor, presence.ref]);

  if (!presence.mounted || shown === null) {
    return null;
  }

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Escape") {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    closeUserCard();

    const anchor = shown.anchor;

    if (anchor.isConnected) {
      duringAppFocus(() => anchor.focus({ preventScroll: true }));
    }
  };

  return (
    <div
      ref={presence.ref}
      id={floatingId}
      role="dialog"
      aria-labelledby={titleId}
      tabIndex={-1}
      popover="manual"
      className="popover floating t-dropdown user-card-surface"
      data-state={presence.state}
      data-placement={placement}
      data-origin={originFor(placement)}
      onPointerEnter={cardEnter}
      onPointerLeave={cardLeave}
      onKeyDown={onKeyDown}
    >
      <UserCard key={shown.userId} userId={shown.userId} titleId={titleId} />
    </div>
  );
}
