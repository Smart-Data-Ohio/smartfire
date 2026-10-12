import { type FormEvent, useId, useState } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import { useStore } from "../../store/store.ts";
import { admin } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { useBusy } from "../settings/settings-parts.tsx";
import { adminFailure, useAdmin } from "./admin-parts.tsx";

export function AfkPolicyForm({ workspace }: { readonly workspace: Workspace }) {
  const { replace } = useAdmin();
  const rows = useStore((state) => state.sidebar.rows);
  const rooms = Object.values(rows).filter((row) => row.room.kind === "voice");
  const [roomId, setRoomId] = useState(workspace.afkPolicy.roomId);
  const [timeoutMinutes, setTimeoutMinutes] = useState(workspace.afkPolicy.timeoutMinutes);
  const { busy, track } = useBusy();
  const id = useId();

  const submit = (event: FormEvent) => {
    event.preventDefault();
    void track(
      "afk",
      admin.updateWorkspace({ afkPolicy: { roomId, timeoutMinutes } }).then(
        (next) => {
          replace(next);
          toast({ title: "AFK policy saved", tone: "success" });
        },
        (error: Error) => adminFailure("Couldn't save the AFK policy", error),
      ),
    );
  };

  return (
    <form className="settings-form" onSubmit={submit}>
      <div className="field">
        <label className="field-label" htmlFor={`${id}-room`}>
          AFK voice room
        </label>
        <select
          id={`${id}-room`}
          className="input settings-select"
          value={roomId ?? ""}
          onChange={(event) =>
            setRoomId(event.target.value === "" ? null : Number(event.target.value))
          }
        >
          <option value="">None</option>
          {roomId !== null && !rooms.some((row) => row.room.id === roomId) ? (
            <option value={roomId}>Room {roomId}</option>
          ) : null}
          {rooms.map((row) => (
            <option key={row.room.id} value={row.room.id}>
              {row.displayName}
            </option>
          ))}
        </select>
      </div>
      <div className="field">
        <label className="field-label" htmlFor={`${id}-timeout`}>
          Idle timeout
        </label>
        <select
          id={`${id}-timeout`}
          className="input settings-select"
          value={timeoutMinutes}
          disabled={roomId === null}
          onChange={(event) => setTimeoutMinutes(Number(event.target.value))}
        >
          {[1, 5, 15, 30, 60].map((minutes) => (
            <option key={minutes} value={minutes}>
              {minutes} {minutes === 1 ? "minute" : "minutes"}
            </option>
          ))}
        </select>
      </div>
      <p className="field-hint">
        Idle callers move only if they can access the AFK room. Muting does not make someone idle.
        Stage audiences stay put. The AFK room admits callers regardless of its user limit.
      </p>
      <Button type="submit" variant="primary" loading={busy("afk")} disabled={busy("afk")}>
        Save AFK policy
      </Button>
    </form>
  );
}
