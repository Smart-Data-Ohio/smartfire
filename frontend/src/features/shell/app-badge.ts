import { useEffect } from "react";
import { useStore } from "../../store/store.ts";
import { sidebarTotals } from "../sidebar/sections.ts";

/** Matches the classic sidebar's app icon: one badge per visible unread room. */
export function useAppBadge(): void {
  const count = useStore((state) =>
    state.sidebar.status === "ready" ? sidebarTotals(state.sidebar).unreadRooms : null,
  );

  useEffect(() => {
    if (count === null || !("setAppBadge" in navigator)) {
      return;
    }

    const update = async () => {
      try {
        if (count === 0 && "clearAppBadge" in navigator) {
          await navigator.clearAppBadge();
        } else {
          await navigator.setAppBadge(count);
        }
      } catch {
        // Badge support or permission can disappear while an installed app is running.
      }
    };

    void update();
  }, [count]);
}
