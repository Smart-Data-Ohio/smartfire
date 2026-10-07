import { createContext, use } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import { ActionError } from "../../sync/run.ts";
import { toast } from "../../ui/toast-store.ts";

/** The loaded workspace and the way a section hands back the server's answer to a write. */
export interface AdminState {
  readonly workspace: Workspace;
  readonly replace: (next: Workspace) => void;
}

export const AdminContext = createContext<AdminState | null>(null);

/** The workspace, for a section inside the admin view. */
export function useAdmin(): AdminState {
  const state = use(AdminContext);

  if (state === null) {
    throw new Error("useAdmin needs the admin view");
  }

  return state;
}

/** Where the classic app asks for the password again; it comes back to this page afterwards. */
export const SUDO_PAGE = "/sudo/new";

/** Whether a write needs the password confirmed first. */
export function needsSudo(error: Error): boolean {
  return error instanceof ActionError && error.tag === "SudoRequired";
}

/**
 * Tells the person a write failed, with the server's reason. A lapsed password confirmation goes
 * to the classic confirmation page instead, as the classic form would; it comes back here.
 */
export function adminFailure(title: string, error: Error): void {
  if (needsSudo(error)) {
    window.location.assign(SUDO_PAGE);

    return;
  }

  toast({ title, description: error.message, tone: "danger" });
}

/** A section only administrators may open, for anyone else who lands on its URL. */
export function AdministratorsOnly() {
  return (
    <p className="settings-callout text-muted" role="status">
      Only administrators can see this page.
    </p>
  );
}
