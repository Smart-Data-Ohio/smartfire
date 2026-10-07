import { createContext, use, useEffect, useRef, useState } from "react";
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

/** Where focus goes after a change moves or removes the row it was in: `control` in row `row`. */
export interface RowFocus {
  /** The row's `data-row`; `null` when there's no row left to go to. */
  readonly row: string | null;
  /** The control's `data-row-control` inside that row. */
  readonly control: string;
}

/** Where focus waits while a dialog or menu opened from a row closes: it isn't lost there yet. */
const OVERLAY = "dialog, [role='menu'], [popover]";

/** How often, and how long, focus is checked while a closing overlay still holds it. */
const WAIT_POLL_MS = 50;

const WAIT_LIMIT_MS = 2000;

/**
 * Keeps focus in a list when a change remounts or removes the focused row (features/destinations'
 * row focus does the same for its lists). Rows carry `data-row` (and `tabIndex={-1}`), their
 * controls `data-row-control`; `focusAfter` names the target with the change. Once the change
 * renders and focus has fallen to the page (after any closing dialog lets go of it), it goes to
 * that control, else its row, else the `container`. Focus someone moved elsewhere stays put.
 */
export function useFocusAfter() {
  const container = useRef<HTMLDivElement | null>(null);
  // A fresh object per request, so asking twice for the same row still runs the effect.
  const [request, setRequest] = useState<{ readonly target: RowFocus } | null>(null);

  useEffect(() => {
    const root = container.current;

    if (request === null || root === null) {
      return;
    }

    const { target } = request;

    // Whether there's nothing left to do: focus landed, or someone put it elsewhere.
    const land = (): boolean => {
      const active = document.activeElement;

      if (active !== null && active !== document.body) {
        return active.closest(OVERLAY) === null;
      }

      const row =
        target.row === null
          ? null
          : root.querySelector<HTMLElement>(`[data-row="${CSS.escape(target.row)}"]`);

      const control = row?.querySelector<HTMLElement>(
        `[data-row-control="${CSS.escape(target.control)}"]:not(:disabled)`,
      );

      (control ?? row ?? root).focus();

      return true;
    };

    if (land()) {
      return;
    }

    let waited = 0;

    const timer = window.setInterval(() => {
      waited += WAIT_POLL_MS;

      if (land() || waited >= WAIT_LIMIT_MS) {
        window.clearInterval(timer);
      }
    }, WAIT_POLL_MS);

    return () => window.clearInterval(timer);
  }, [request]);

  const focusAfter = (target: RowFocus) => setRequest({ target });

  return { container, focusAfter };
}
