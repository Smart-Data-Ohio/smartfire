import { ActionError } from "../../sync/run.ts";

/**
 * Whether a write that needed a fresh confirmation was left behind for a Google one: it carried a
 * credential, so it isn't kept for the return. Its form keeps what isn't secret for then.
 */
export function leftToConfirm(error: Error): boolean {
  return error instanceof ActionError && error.tag === "SudoRequired";
}

/**
 * Whether a write stopped at its confirmation: the person closed it, or left to confirm with
 * Google. Nothing went to the server, and the dialog already said what happened.
 */
export function stoppedAtConfirmation(error: Error): boolean {
  return (
    error instanceof ActionError &&
    (error.tag === "SudoRequired" || error.tag === "ConfirmationCancelled")
  );
}
