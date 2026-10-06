import type { MessageDTO } from "../../store/model.ts";

/**
 * Under a message that started a thread: reply count, the last repliers' faces and "Last reply
 * 2 hours ago"; opens the thread in the right pane. (Threads slice.)
 */
export function ThreadIndicator(_props: { readonly message: MessageDTO }) {
  return null;
}
