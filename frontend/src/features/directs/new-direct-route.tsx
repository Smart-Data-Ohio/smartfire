import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";
import { openOverlay } from "../switcher/overlay-store.ts";

/**
 * `/app/rooms/new/direct` (the classic `rooms/directs/new` page): opens the New message picker
 * over the home screen, as the "+" by Direct messages does.
 */
export function NewDirectRoute() {
  const navigate = useNavigate();

  useEffect(() => {
    openOverlay("new-direct");
    void navigate({ to: "/", replace: true });
  }, [navigate]);

  return null;
}
