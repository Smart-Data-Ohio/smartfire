import { useEffect } from "react";
import { actions } from "../../sync/runtime.ts";

export function useChatSounds(roomId: number): void {
  useEffect(() => actions.chatSounds.listen(roomId), [roomId]);
}
