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

/**
 * Whether a failed write may still have reached the server and been saved, so what the form holds
 * can't be trusted until an answer says what the server has. Only a refusal (a 422 `Validation`,
 * which writes nothing) or a stop at the confirmation (nothing was sent) is known to have saved
 * nothing. A dropped connection, a timeout or a server error may have come after the save.
 */
export function writeMayHaveLanded(error: Error): boolean {
  const refused = error instanceof ActionError && error.tag === "Validation";

  return !refused && !stoppedAtConfirmation(error);
}
