import { useMatchRoute, useNavigate } from "@tanstack/react-router";
import { lazy, Suspense, useEffect, useState } from "react";
import { useStore } from "../../store/store.ts";

const RoomSettingsDialog = lazy(() => import("./room-settings-dialog.tsx"));

/** Fetches the settings dialog's chunk ahead of a click (the header title's hover). */
export function preloadRoomSettings(): void {
  void import("./room-settings-dialog.tsx");
}

/**
 * Shows the room's settings dialog while the URL is `/app/r/:id/settings`, over the conversation;
 * closing it goes back to the room. A direct message has no settings page here (its header has
 * rename and add people), so its URL opens the conversation instead.
 */
export function RoomSettingsHost({ roomId }: { readonly roomId: number }) {
  const navigate = useNavigate();
  const matchRoute = useMatchRoute();
  const open = matchRoute({ to: "/r/$roomId/settings" }) !== false;

  const kind = useStore(
    (state) =>
      state.rooms[roomId]?.detail?.room.kind ?? state.sidebar.rows[roomId]?.room.kind ?? null,
  );

  const [seen, setSeen] = useState(open);
  const direct = kind === "direct";

  if (open && !seen && !direct) {
    setSeen(true);
  }

  useEffect(() => {
    if (open && direct) {
      void navigate({ to: "/r/$roomId", params: { roomId }, replace: true });
    }
  }, [open, direct, navigate, roomId]);

  if (!seen || direct) {
    return null;
  }

  return (
    <Suspense fallback={null}>
      <RoomSettingsDialog
        roomId={roomId}
        open={open}
        onOpenChange={(next) => {
          if (!next) void navigate({ to: "/r/$roomId", params: { roomId } });
        }}
      />
    </Suspense>
  );
}
