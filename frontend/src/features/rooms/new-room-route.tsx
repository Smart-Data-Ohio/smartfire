import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";
import { openNewRoom } from "./new-room-store.ts";
import type { ManagedKind } from "./room-forms.ts";

/**
 * `/app/rooms/new/<kind>` (the classic `rooms/<kind>/new` pages, and the installed app's "New
 * room" shortcut): opens the create-a-room dialog on that kind over the home screen.
 */
export function NewRoomRoute({ kind }: { readonly kind: ManagedKind }) {
  const navigate = useNavigate();

  useEffect(() => {
    openNewRoom(kind);
    void navigate({ to: "/", replace: true });
  }, [kind, navigate]);

  return null;
}
