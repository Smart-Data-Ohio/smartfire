/**
 * The room's chat beside theater mode (Discord's "Show chat"): while a share is expanded, the
 * call's room keeps its own timeline and composer in a column beside the stage (below it on
 * narrower screens and phones), so people can read and post without leaving the share. The same
 * mounted timeline and composer move, so drafts, typing, pickers and live messages carry on. Open
 * by default; showing or hiding it lasts for the tab's session, across calls.
 */
import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";
import { useStore } from "../../store/store.ts";
import { usePhoneLayout } from "../panes/use-right-pane.ts";
import { type CallState, useCall } from "./call-store.ts";
import { callActiveIn, useCallViewCovers } from "./call-view-cover.ts";

export const THEATER_CHAT_STORAGE_KEY = "smartfire.theaterChat";

function readTheaterChat(): boolean {
  try {
    return sessionStorage.getItem(THEATER_CHAT_STORAGE_KEY) !== "hidden";
  } catch {
    return true;
  }
}

export const theaterChatStore = createStore<{ readonly open: boolean }>(() => ({
  open: readTheaterChat(),
}));

export function setTheaterChat(open: boolean): void {
  theaterChatStore.setState({ open });

  try {
    sessionStorage.setItem(THEATER_CHAT_STORAGE_KEY, open ? "shown" : "hidden");
  } catch {
    // Without storage the choice lasts until reload.
  }
}

export function useTheaterChatOpen(): boolean {
  return useZustand(theaterChatStore, (state) => state.open);
}

/** Whether `roomId`'s call has a share expanded into theater mode (one still in the call). */
export function theaterIn(state: CallState, roomId: number): boolean {
  const videoId = state.expandedVideoId;

  return (
    videoId !== null &&
    callActiveIn(state, roomId) &&
    state.snapshot.participants.some((participant) => participant.screenId === videoId)
  );
}

/**
 * Whether `roomId`'s chat shows beside its theater mode: the call view is up (desktop: open; a
 * phone: covering the room), a share is expanded, the room has a conversation (a board hasn't)
 * and the viewer hasn't hidden the chat.
 */
export function useTheaterChat(roomId: number): boolean {
  const phone = usePhoneLayout();
  const covers = useCallViewCovers(roomId);
  const theater = useCall((state) => theaterIn(state, roomId));
  // Desktop: the controller's `viewOpen`. A phone: the URL's `call=1` (call-view-cover.ts).
  const viewOpen = useCall((state) => state.viewOpen);
  const conversation = useStore((state) => state.rooms[roomId]?.detail?.room.kind !== "board");
  const open = useTheaterChatOpen();

  return open && conversation && theater && (phone ? covers : viewOpen);
}
