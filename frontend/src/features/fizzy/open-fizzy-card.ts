import { useNavigate } from "@tanstack/react-router";
import type { MessageDTO } from "../../store/model.ts";
import { overConversationState } from "./overlay-history.ts";

/**
 * The message menu's "Create Fizzy card": opens the dialog's URL over the conversation, the
 * room's for a timeline message and the thread's for a reply. Closing it steps back.
 */
export function useOpenFizzyCard(): (message: MessageDTO) => void {
  const navigate = useNavigate();

  return (message) => {
    if (message.threadId === null) {
      void navigate({
        to: "/r/$roomId/m/$sourceId/fizzy/new",
        params: { roomId: message.roomId, sourceId: message.id },
        state: overConversationState(),
      });

      return;
    }

    void navigate({
      to: "/r/$roomId/t/$threadId/m/$sourceId/fizzy/new",
      params: { roomId: message.roomId, threadId: message.threadId, sourceId: message.id },
      search: {},
      state: overConversationState(),
    });
  };
}
