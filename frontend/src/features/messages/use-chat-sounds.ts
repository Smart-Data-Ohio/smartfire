import { useEffect, useRef } from "react";
import { actions } from "../../sync/runtime.ts";

export function useChatSounds(roomId: number, viewingLatestPage: () => boolean): void {
  const viewingRef = useRef(viewingLatestPage);

  viewingRef.current = viewingLatestPage;
  useEffect(() => actions.chatSounds.listen(roomId, () => viewingRef.current()), [roomId]);
}
