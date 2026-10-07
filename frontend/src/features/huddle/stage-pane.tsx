/**
 * The stage, in the right pane (the classic stage drawer): who's live and Go live or Stop
 * stream, the viewer's own place (raise or lower a hand), and the Hosts, Speakers and Listeners
 * roster with each manager's actions. Raised hands chime for managers, once a minute per person,
 * with an announcement. The roster stays live over the room's `stage.updated` events.
 */
import { type ReactNode, useEffect, useRef, useState } from "react";
import type { StreamQuality } from "../../gen/StreamQuality.ts";
import { store, useStore } from "../../store/store.ts";
import { huddles } from "../../sync/huddles.ts";
import { ActionError } from "../../sync/run.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Menu, MenuItem } from "../../ui/menu.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneFrame, RoomName } from "../panes/pane-frame.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { callController } from "./call-controller.ts";
import { useCall } from "./call-store.ts";
import { useCallParticipants } from "./presence.ts";
import { playHandChime, soundsMuted } from "./sounds.ts";
import {
  canManageStage,
  DEFAULT_STREAM_QUALITY,
  handsToAnnounce,
  STREAM_QUALITIES,
  type StageAction,
  type StageEntry,
  type StageViewer,
  stageActions,
  stageGroups,
} from "./stage.ts";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "ready" }
  | { readonly status: "error"; readonly message: string };

function failed(error: Error): string {
  return error.message === "" ? "Something went wrong." : error.message;
}

function request(roomId: number, entry: StageEntry, action: StageAction): Promise<void> {
  const membershipId = entry.member.membershipId;

  switch (action.kind) {
    case "role":
      return huddles.changeRole(roomId, membershipId, action.role);
    case "lower":
      return huddles.lowerHand(roomId, membershipId);
    case "moderate":
      return huddles.moderate(roomId, membershipId, action.action);
  }
}

function perform(roomId: number, entry: StageEntry, action: StageAction): Promise<void> {
  return request(roomId, entry, action).catch((error: Error) => {
    toast({ title: failed(error), tone: "danger" });
  });
}

function useViewer(roomId: number): StageViewer {
  const viewerId = useStore((state) => state.me?.user.id ?? null);
  const administrator = useStore((state) => state.me?.user.role === "administrator");

  const member = useStore(
    (state) => state.stages[roomId]?.members.find((entry) => entry.userId === viewerId) ?? null,
  );

  return {
    membershipId: member?.membershipId ?? null,
    role: member?.role ?? null,
    administrator,
  };
}

function MemberRow({
  roomId,
  entry,
  viewer,
  hostCount,
  inCall,
}: {
  readonly roomId: number;
  readonly entry: StageEntry;
  readonly viewer: StageViewer;
  readonly hostCount: number;
  readonly inCall: boolean;
}) {
  const { member } = entry;
  const actions = stageActions(entry, viewer, hostCount, inCall);
  const invite = actions.find((action) => action.kind === "role" && action.primary);
  const self = member.membershipId === viewer.membershipId;
  const roleWord = { host: "Host", speaker: "Speaker", listener: "Listener" }[member.role];

  return (
    <li className="stage-member" data-hand={member.handRaisedAt === null ? undefined : true}>
      <UserAvatar userId={member.userId} size={28} decorative />
      <span className="stage-member-text">
        <span className="stage-member-name">
          {entry.name}
          {self ? <span className="stage-member-you">(you)</span> : null}
        </span>
        <span className="stage-member-role">
          {roleWord}
          {member.serverMuted ? <span className="stage-badge">Muted</span> : null}
          {entry.queue === null ? null : (
            <span className="stage-badge" data-tone="hand">
              <Icon name="hand" size={12} /> Hand raised · #{entry.queue} in queue
            </span>
          )}
          {inCall ? (
            <span className="stage-in-call" title="In the call">
              <Icon name="headphones" size={12} />
            </span>
          ) : null}
        </span>
      </span>
      {invite === undefined ? null : (
        <Button variant="primary" size="sm" onClick={() => void perform(roomId, entry, invite)}>
          {invite.label}
        </Button>
      )}
      {actions.length === 0 ? null : (
        <Menu
          placement="bottom-end"
          label={`Actions for ${entry.name}`}
          trigger={(props) => (
            <IconButton {...props} icon="more" size="sm" label={`Actions for ${entry.name}`} />
          )}
        >
          {actions.map((action) => (
            <MenuItem
              key={action.label}
              tone={action.kind === "moderate" && action.action !== "unmute" ? "danger" : undefined}
              disabled={action.kind === "role" && action.disabled !== null}
              onSelect={() => void perform(roomId, entry, action)}
            >
              {action.label}
              {action.kind === "role" && action.disabled !== null ? (
                <span className="stage-action-hint">{action.disabled}</span>
              ) : null}
            </MenuItem>
          ))}
        </Menu>
      )}
    </li>
  );
}

function Group({
  label,
  entries,
  empty,
  children,
}: {
  readonly label: string;
  readonly entries: readonly StageEntry[];
  readonly empty: string | null;
  readonly children: (entry: StageEntry) => ReactNode;
}) {
  return (
    <section className="stage-group" aria-label={label}>
      <h3 className="stage-group-heading">
        {label} · {entries.length}
      </h3>
      {entries.length === 0 && empty !== null ? (
        <p className="stage-group-empty">{empty}</p>
      ) : (
        <ul className="stage-list">{entries.map(children)}</ul>
      )}
    </section>
  );
}

/** Live now and Stop stream, or Go live; then the viewer's own place on the stage. */
function StageControls({
  roomId,
  viewer,
}: {
  readonly roomId: number;
  readonly viewer: StageViewer;
}) {
  const live = useStore((state) => state.stages[roomId]?.live ?? null);

  const presenter = useStore((state) =>
    live === null ? "" : (state.users[live.userId]?.name ?? UNKNOWN_NAME),
  );

  const hand = useStore(
    (state) =>
      state.stages[roomId]?.members.find((entry) => entry.membershipId === viewer.membershipId)
        ?.handRaisedAt ?? null,
  );

  const connected = useCall((state) => state.roomId === roomId && state.phase === "connected");

  const [quality, setQuality] = useState<StreamQuality>(DEFAULT_STREAM_QUALITY);
  const [busy, setBusy] = useState(false);
  const presenting = live !== null && live.membershipId === viewer.membershipId;

  const run = (work: () => Promise<void>) => {
    setBusy(true);

    void work()
      .catch((error: Error) => toast({ title: failed(error), tone: "danger" }))
      .finally(() => setBusy(false));
  };

  let place: string;

  if (viewer.role === "listener") {
    place =
      hand === null
        ? "You are in the audience."
        : "Your hand is raised. A host can invite you to speak.";
  } else if (viewer.role === "speaker") {
    place = "You are speaking. Join the stage to be heard.";
  } else {
    place = "You are hosting this stage.";
  }

  return (
    <div className="stage-controls">
      {live === null ? null : (
        <div className="stage-live">
          <span className="call-live-badge">
            <Icon name="radio" size={12} /> Live
          </span>
          <span className="stage-live-name">{presenter}</span>
          {/* The presenter, hosts and administrators may end a stream. */}
          {presenting || canManageStage(viewer) ? (
            <Button
              variant="danger"
              size="sm"
              disabled={busy}
              onClick={() =>
                run(async () => {
                  await huddles.stopStream(roomId, live.id);
                  await callController.streamStopped(roomId);
                })
              }
            >
              Stop stream
            </Button>
          ) : null}
        </div>
      )}
      {live === null && (viewer.role === "host" || viewer.role === "speaker") ? (
        <div className="stage-go-live">
          <label className="stage-quality">
            <span>Stream quality</span>
            <select
              className="huddle-select"
              value={quality}
              onChange={(event) => {
                const next = STREAM_QUALITIES.find((value) => value === event.currentTarget.value);

                if (next !== undefined) {
                  setQuality(next);
                }
              }}
            >
              {STREAM_QUALITIES.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
          {/* The capture starts inside this click, before any request (Safari insists). */}
          <Button
            variant="primary"
            size="sm"
            icon="radio"
            disabled={!connected || busy}
            title={connected ? undefined : "Join the stage to go live"}
            onClick={() => run(() => callController.goLive(roomId, quality))}
          >
            Go live
          </Button>
        </div>
      ) : null}
      {viewer.role === null ? null : (
        <div className="stage-place">
          <p className="stage-note">{place}</p>
          {viewer.role === "listener" ? (
            <Button
              variant="secondary"
              size="sm"
              icon="hand"
              disabled={busy}
              onClick={() =>
                run(() =>
                  hand === null ? huddles.raiseHand(roomId) : huddles.lowerHand(roomId, null),
                )
              }
            >
              {hand === null ? "Raise hand" : "Lower hand"}
            </Button>
          ) : null}
        </div>
      )}
    </div>
  );
}

/** A manager hears and reads about newly raised hands, once a minute per person. */
function useHandAnnouncements(roomId: number, manage: boolean): string {
  const [announcement, setAnnouncement] = useState("");
  const raised = useRef<ReadonlySet<number> | null>(null);
  const chimedAt = useRef(new Map<number, number>());

  const hands = useStore((state) =>
    (state.stages[roomId]?.members ?? [])
      .filter((member) => member.handRaisedAt !== null)
      .map((member) => member.membershipId)
      .join(","),
  );

  useEffect(() => {
    const ids = new Set(
      hands
        .split(",")
        .filter((part) => part !== "")
        .map(Number),
    );

    const before = raised.current;

    raised.current = ids;

    // The first roster is the baseline, not news.
    if (before === null || !manage) {
      return;
    }

    const now = Date.now();
    const due = handsToAnnounce(before, ids, chimedAt.current, now);

    if (due.length === 0) {
      return;
    }

    for (const id of due) {
      chimedAt.current.set(id, now);
    }

    setAnnouncement(
      ids.size === 1
        ? "A listener raised their hand."
        : `${ids.size} listeners have their hands raised.`,
    );

    if (!soundsMuted(store.getState().me, new Date())) {
      playHandChime();
    }
  }, [hands, manage]);

  return announcement;
}

export function StagePane({ roomId }: { readonly roomId: number }) {
  const kind = useStore((state) => state.sidebar.rows[roomId]?.room.kind ?? null);
  const stage = useStore((state) => state.stages[roomId] ?? null);
  const users = useStore((state) => state.users);
  const viewer = useViewer(roomId);
  const participants = useCallParticipants(roomId);
  const [load, setLoad] = useState<Load>({ status: stage === null ? "loading" : "ready" });
  const [attempt, setAttempt] = useState(0);
  const manage = canManageStage(viewer);
  const announcement = useHandAnnouncements(roomId, manage);

  // biome-ignore lint/correctness/useExhaustiveDependencies: attempt is the Retry trigger, not an input
  useEffect(() => {
    if (kind !== "stage") {
      return;
    }

    let current = true;

    void huddles
      .stage(roomId)
      .then(() => {
        if (current) {
          setLoad({ status: "ready" });
        }
      })
      .catch((error: Error) => {
        if (current) {
          setLoad({
            status: "error",
            message:
              error instanceof ActionError && error.tag === "NotFound"
                ? "This stage isn’t available."
                : failed(error),
          });
        }
      });

    return () => {
      current = false;
    };
  }, [roomId, kind, attempt]);

  if (kind !== "stage") {
    return (
      <PaneFrame title="Stage" subtitle={<RoomName roomId={roomId} />}>
        <PaneEmpty
          icon="radio"
          title="Not a stage"
          text="Stages are rooms for talks and town halls."
        />
      </PaneFrame>
    );
  }

  if (stage === null) {
    return (
      <PaneFrame title="Stage" subtitle={<RoomName roomId={roomId} />}>
        {load.status === "error" ? (
          <PaneError
            message={load.message}
            onRetry={() => {
              setLoad({ status: "loading" });
              setAttempt((value) => value + 1);
            }}
          />
        ) : (
          <PaneListSkeleton rows={5} square={28} />
        )}
      </PaneFrame>
    );
  }

  const groups = stageGroups(stage, (userId) => {
    const user = users[userId];

    return {
      name: user?.name ?? UNKNOWN_NAME,
      administrator: user?.role === "administrator",
    };
  });

  const calling = new Set(participants.map((participant) => participant.userId));

  const row = (entry: StageEntry) => (
    <MemberRow
      key={entry.member.membershipId}
      roomId={roomId}
      entry={entry}
      viewer={viewer}
      hostCount={groups.hosts.length}
      inCall={calling.has(entry.member.userId)}
    />
  );

  return (
    <PaneFrame title="Stage" subtitle={<RoomName roomId={roomId} />}>
      <div className="stage-pane">
        <StageControls roomId={roomId} viewer={viewer} />
        <p className="visually-hidden" aria-live="polite">
          {announcement}
        </p>
        <Group label="Hosts" entries={groups.hosts} empty={null}>
          {row}
        </Group>
        <Group label="Speakers" entries={groups.speakers} empty="Nobody is speaking yet.">
          {row}
        </Group>
        <Group label="Listeners" entries={groups.listeners} empty="Nobody is listening yet.">
          {row}
        </Group>
      </div>
    </PaneFrame>
  );
}
