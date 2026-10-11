/**
 * What the welcome card and the guide do: put the card away (or bring it back), saved with the
 * person's appearance preferences so every session follows; and join a suggested room.
 */
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { welcomeDismissal } from "./welcome-dismissal.ts";
import { welcomeStore } from "./welcome-store.ts";

/**
 * Hides the welcome card from Home, or (`false`) shows it there again: at once, before the save
 * answers, so a navigation right after sees the change. A refused save puts it back.
 */
export function setWelcomeDismissed(dismissed: boolean): void {
  welcomeDismissal.set(dismissed).catch((error: Error) => {
    toast({
      title: dismissed ? "Couldn't hide the welcome" : "Couldn't show the welcome",
      description: error.message,
      tone: "danger",
    });
  });
}

/** Joins an open room; the welcome is read again so the card says Open from then on. */
export async function joinSuggestedRoom(roomId: number): Promise<void> {
  await actions.joinOpenRoom(roomId);
  void welcomeStore.refresh();
}
