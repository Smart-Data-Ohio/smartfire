import { useEffect, useId, useRef, useState } from "react";
import type { AnnouncementFollow } from "../../gen/AnnouncementFollow.ts";
import type { RoomKind } from "../../gen/RoomKind.ts";
import { useAnnouncementRoom } from "../../store/announcements.ts";
import { useStore } from "../../store/store.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import { RoomGlyph } from "./room-glyph.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | {
      readonly status: "ready";
      readonly following: readonly AnnouncementFollow[];
      readonly followers: readonly AnnouncementFollow[];
    };

/** One followed (or following) room: its glyph and name, and Unfollow for those who may. */
function FollowRow({
  roomId,
  removing,
  disabled,
  onRemove,
}: {
  readonly roomId: number;
  readonly removing: boolean;
  readonly disabled: boolean;
  readonly onRemove: (() => void) | null;
}) {
  const row = useStore((state) => state.sidebar.rows[roomId] ?? null);
  const name = row?.displayName ?? "Unknown channel";

  return (
    <li className="room-follow">
      <RoomGlyph
        kind={row?.room.kind ?? "open"}
        iconName={row?.room.iconName ?? null}
        announcement={row?.room.announcement === true}
        size={16}
        className="room-follow-icon"
      />
      <span className="room-follow-name">{name}</span>
      {onRemove === null ? null : (
        <Button
          size="sm"
          variant="secondary"
          aria-label={`Unfollow ${name}`}
          disabled={disabled}
          loading={removing}
          loadingLabel="Unfollowing…"
          onClick={onRemove}
        >
          Unfollow
        </Button>
      )}
    </li>
  );
}

/** The follows read in flight, or failed with a way to try again. */
function FollowsState({
  load,
  onRetry,
}: {
  readonly load: Exclude<Load, { readonly status: "ready" }>;
  readonly onRetry: () => void;
}) {
  if (load.status === "loading") {
    return (
      <div className="room-settings-skeleton" role="status" aria-label="Loading follows">
        <Skeleton width="50%" height={12} />
        <Skeleton width="35%" height={12} />
      </div>
    );
  }

  return (
    <div className="room-form-load-error">
      <p className="picker-note picker-error" role="alert">
        Couldn't load this channel's follows: {load.message}
      </p>
      <Button size="sm" onClick={onRetry}>
        Try again
      </Button>
    </div>
  );
}

interface AnnouncementSettingsProps {
  readonly roomId: number;
  readonly kind: RoomKind;
  /** The creator or an administrator: they may change the flag and this room's follows. */
  readonly manage: boolean;
}

/**
 * Announcement rooms, in a room's settings. Its creator and administrators can make it one: then
 * its authors (and administrators) can publish a message, which copies it into every room that
 * follows it. Below, the announcement rooms this room follows: anyone in it can read the list; its
 * managers can follow one more (any announcement room they are in) or unfollow. A board can be an
 * announcement room but can't follow one: it shows posts, not timeline messages. Each change
 * saves as it's made.
 */
export function AnnouncementSettings({ roomId, kind, manage }: AnnouncementSettingsProps) {
  const pickerId = useId();
  const mounted = useRef(true);
  // The switch shows the store's word only: sync, or this write's reply when nothing newer came.
  const announcement = useAnnouncementRoom(roomId);
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [flagging, setFlagging] = useState(false);
  const [picked, setPicked] = useState("");
  const [following, setFollowing] = useState(false);
  const [unfollowing, setUnfollowing] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const followable = kind !== "board";

  useEffect(() => {
    mounted.current = true;

    return () => {
      mounted.current = false;
    };
  }, []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: attempt is the Try again trigger
  useEffect(() => {
    let live = true;

    setLoad({ status: "loading" });
    actions.announcements.follows(roomId).then(
      (list) => {
        if (live) setLoad({ status: "ready", ...list });
      },
      (failure: ActionError) => {
        if (live) setLoad({ status: "error", message: failure.message });
      },
    );

    return () => {
      live = false;
    };
  }, [roomId, attempt]);

  const followed = load.status === "ready" ? load.following : [];

  const rows = useStore((state) => state.sidebar.rows);
  const nameOf = (id: number) => rows[id]?.displayName ?? "the channel";

  // Announcement rooms the viewer is in that this room doesn't follow yet. The server decides the
  // rest (no cycles, still an announcement room) and says why when it refuses.
  const options = Object.values(rows)
    .filter(
      (row) =>
        row.room.id !== roomId &&
        row.room.kind !== "direct" &&
        row.room.announcement === true &&
        !followed.some((follow) => follow.sourceRoomId === row.room.id),
    )
    .map((row) => ({ id: row.room.id, name: row.displayName }))
    .sort((left, right) => left.name.localeCompare(right.name));

  const choice = options.some((option) => `${option.id}` === picked)
    ? picked
    : `${options[0]?.id ?? ""}`;

  const flag = (next: boolean) => {
    if (flagging) return;

    setFlagging(true);
    setProblem(null);
    actions.announcements.set(roomId, next).then(
      () => {
        if (mounted.current) setFlagging(false);
      },
      (failure: ActionError) => {
        if (!mounted.current) return;

        setFlagging(false);
        setProblem(failure.message);
      },
    );
  };

  const follow = () => {
    const sourceRoomId = Number(choice);

    if (choice === "" || following || load.status !== "ready") return;

    const name = nameOf(sourceRoomId);

    setFollowing(true);
    setProblem(null);
    actions.announcements.follow(sourceRoomId, roomId).then(
      () => {
        toast({ title: `Following #${name}`, tone: "success" });

        // The reply lists the source room's follows; this room's list is read again.
        return actions.announcements.follows(roomId).then(
          (list) => {
            if (!mounted.current) return;

            setFollowing(false);
            setPicked("");
            setLoad({ status: "ready", ...list });
          },
          (failure: ActionError) => {
            if (!mounted.current) return;

            setFollowing(false);
            setLoad({ status: "error", message: failure.message });
          },
        );
      },
      (failure: ActionError) => {
        if (!mounted.current) return;

        setFollowing(false);
        setProblem(failure.message);
      },
    );
  };

  const unfollow = (sourceRoomId: number) => {
    if (unfollowing !== null) return;

    const name = nameOf(sourceRoomId);

    setUnfollowing(sourceRoomId);
    setProblem(null);
    actions.announcements.unfollow(sourceRoomId, roomId).then(
      () => {
        toast({ title: `Unfollowed #${name}` });

        if (!mounted.current) return;

        setUnfollowing(null);
        setLoad((current) =>
          current.status === "ready"
            ? {
                ...current,
                following: current.following.filter((each) => each.sourceRoomId !== sourceRoomId),
              }
            : current,
        );
      },
      (failure: ActionError) => {
        if (!mounted.current) return;

        setUnfollowing(null);
        setProblem(failure.message);
      },
    );
  };

  const busy = following || unfollowing !== null;

  // Until the follows load, every list shows their state: a board, with no Following list, shows
  // it under Followed by, so a failed read can be retried there too.
  const status =
    load.status === "ready" ? null : (
      <FollowsState load={load} onRetry={() => setAttempt((count) => count + 1)} />
    );

  const followedBy =
    manage && announcement && (load.status === "ready" ? load.followers.length > 0 : !followable);

  return (
    <div className="room-integrations" data-room-integration="">
      {manage ? (
        <div className="room-privacy">
          <Toggle
            checked={announcement}
            disabled={flagging}
            onCheckedChange={flag}
            label="Announcement channel"
            description={
              announcement
                ? "Authors and administrators can publish its messages to every channel that follows it."
                : "Let other channels follow this one and get a copy of each message published here."
            }
          />
        </div>
      ) : (
        <p className="room-readonly-note">
          {announcement
            ? "This is an announcement channel: other channels can follow it."
            : "Only the person who made this room and administrators can make it an announcement channel."}
        </p>
      )}
      {followable ? (
        <section className="room-subscribe" aria-label="Following">
          <h3 className="room-subscribe-title">Following</h3>
          <p className="room-integration-note">
            Messages published in these announcement channels are copied here.
          </p>
          {status !== null ? (
            status
          ) : followed.length === 0 ? (
            <p className="room-integration-note">Not following any announcement channels.</p>
          ) : (
            <ul className="room-follow-list">
              {followed.map((each) => (
                <FollowRow
                  key={each.sourceRoomId}
                  roomId={each.sourceRoomId}
                  removing={unfollowing === each.sourceRoomId}
                  disabled={busy}
                  onRemove={manage ? () => unfollow(each.sourceRoomId) : null}
                />
              ))}
            </ul>
          )}
          {manage && load.status === "ready" ? (
            options.length === 0 ? (
              <p className="room-integration-note">
                {followed.length === 0
                  ? "There are no announcement channels you're in to follow."
                  : "This channel follows every announcement channel you're in."}
              </p>
            ) : (
              <div className="room-follow-picker">
                <label className="visually-hidden" htmlFor={pickerId}>
                  Announcement channel to follow
                </label>
                <select
                  id={pickerId}
                  className="input room-follow-select"
                  value={choice}
                  disabled={busy}
                  onChange={(event) => {
                    setPicked(event.target.value);
                    setProblem(null);
                  }}
                >
                  {options.map((option) => (
                    <option key={option.id} value={`${option.id}`}>
                      {option.name}
                    </option>
                  ))}
                </select>
                <Button
                  variant="primary"
                  icon="megaphone"
                  disabled={busy}
                  loading={following}
                  loadingLabel="Following…"
                  onClick={follow}
                >
                  Follow
                </Button>
              </div>
            )
          ) : null}
        </section>
      ) : null}
      {followedBy ? (
        <section className="room-subscribe" aria-label="Followed by">
          <h3 className="room-subscribe-title">Followed by</h3>
          {load.status === "ready" ? (
            <ul className="room-follow-list">
              {load.followers.map((each) => (
                <FollowRow
                  key={each.targetRoomId}
                  roomId={each.targetRoomId}
                  removing={false}
                  disabled
                  onRemove={null}
                />
              ))}
            </ul>
          ) : (
            status
          )}
        </section>
      ) : null}
      {problem === null ? null : (
        <p className="picker-note picker-error" role="alert">
          {problem}
        </p>
      )}
    </div>
  );
}
