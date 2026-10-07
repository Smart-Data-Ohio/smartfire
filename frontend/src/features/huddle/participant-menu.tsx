/**
 * What you can do about someone in the call: their volume for you (0–200 %, remembered per
 * person), muting them for you only, and, for administrators and stage hosts in a voice or
 * stage room, muting them for everyone or removing them from the call (administrators only
 * for other administrators).
 */
import { type ReactElement, useState } from "react";
import { useStore } from "../../store/store.ts";
import { huddles } from "../../sync/huddles.ts";
import { Button } from "../../ui/button.tsx";
import { Popover, type PopoverTriggerProps } from "../../ui/popover.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { callController } from "./call-controller.ts";
import { useCall } from "./call-store.ts";
import { loadParticipantVolume } from "./engine/preferences.ts";

/** Whether the viewer may moderate this room's call: an administrator, or a stage's host. */
export function useCanModerate(roomId: number | null): boolean {
  return useStore((state) => {
    if (roomId === null) {
      return false;
    }

    const row = state.sidebar.rows[roomId];
    const kind = row?.room.kind;

    if (kind !== "voice" && kind !== "stage") {
      return false;
    }

    if (state.me?.user.role === "administrator") {
      return true;
    }

    // The live roster beats the sidebar's copy of the membership once the stage has loaded.
    const viewerId = state.me?.user.id;
    const member = state.stages[roomId]?.members.find((entry) => entry.userId === viewerId);

    return (member?.role ?? row?.membership.stageRole) === "host";
  });
}

interface ParticipantMenuProps {
  readonly userId: number;
  readonly trigger: (props: PopoverTriggerProps) => ReactElement;
}

function VolumeControls({ userId }: { readonly userId: number }) {
  const capped = useCall((state) => state.snapshot.boostCapped);
  const [volume, setVolume] = useState(() => loadParticipantVolume(userId));
  const muted = useCall((state) => state.localMutes.includes(userId));
  const max = capped ? 100 : 200;

  return (
    <>
      <label className="huddle-volume">
        <span className="huddle-volume-label">
          Volume <output>{Math.min(volume, max)}%</output>
        </span>
        <input
          type="range"
          min={0}
          max={max}
          step={5}
          value={Math.min(volume, max)}
          onChange={(event) => {
            const next = Number(event.currentTarget.value);

            setVolume(next);
            callController.setParticipantVolume(userId, next);
          }}
        />
      </label>
      {capped ? (
        <p className="huddle-volume-hint">
          Boosting above 100% needs the default speaker in this browser.
        </p>
      ) : null}
      <Toggle
        checked={muted}
        onCheckedChange={() => callController.toggleParticipantMute(userId)}
        label="Mute for me"
        description="Only you stop hearing them"
      />
    </>
  );
}

function ModerationControls({
  userId,
  close,
}: {
  readonly userId: number;
  readonly close: () => void;
}) {
  const roomId = useCall((state) => state.roomId);

  const participant = useStore((state) =>
    roomId === null
      ? undefined
      : state.huddles[roomId]?.participants.find((entry) => entry.userId === userId),
  );

  if (roomId === null || participant === undefined) {
    return null;
  }

  const moderate = (action: "mute" | "unmute" | "disconnect") => {
    close();
    huddles.moderate(roomId, participant.membershipId, action).catch((error: Error) => {
      toast({ title: "Couldn’t do that", description: error.message, tone: "danger" });
    });
  };

  return (
    <div className="huddle-moderation">
      <Button
        variant="secondary"
        size="sm"
        icon={participant.serverMuted ? "mic" : "mic-off"}
        onClick={() => moderate(participant.serverMuted ? "unmute" : "mute")}
      >
        {participant.serverMuted ? "Allow to speak" : "Mute for everyone"}
      </Button>
      <Button variant="danger" size="sm" icon="user-x" onClick={() => moderate("disconnect")}>
        Remove from call
      </Button>
    </div>
  );
}

export function ParticipantMenu({ userId, trigger }: ParticipantMenuProps) {
  const name = useStore((state) => state.users[userId]?.name ?? UNKNOWN_NAME);
  const viewerId = useStore((state) => state.me?.user.id ?? null);
  const roomId = useCall((state) => state.roomId);
  const canModerate = useCanModerate(roomId);

  // Only administrators moderate administrators (the server refuses a host otherwise).
  const outranked = useStore(
    (state) =>
      state.users[userId]?.role === "administrator" && state.me?.user.role !== "administrator",
  );

  const self = userId === viewerId;

  return (
    <Popover label={`${name} in the call`} placement="top" trigger={trigger}>
      {(close) => (
        <div className="huddle-participant-menu">
          <p className="huddle-participant-menu-name">{name}</p>
          {self ? (
            <p className="huddle-volume-hint">This is you.</p>
          ) : (
            <VolumeControls userId={userId} />
          )}
          {canModerate && !self && !outranked ? (
            <ModerationControls userId={userId} close={close} />
          ) : null}
        </div>
      )}
    </Popover>
  );
}
