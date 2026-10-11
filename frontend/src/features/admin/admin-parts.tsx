import { createContext, type ReactNode, use, useRef } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import {
  browserDeps,
  DEFAULT_UPLOAD_LIMIT_BYTES,
  UploadTask,
} from "../../lib/upload/direct-upload.ts";
import { store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { toast } from "../../ui/toast-store.ts";
import { stoppedAtConfirmation } from "../auth/confirmation.ts";
import { type RowParts, useKeepRowFocus } from "../destinations/row-focus.ts";

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

/**
 * Tells the person a write failed, with the server's reason. A write that stopped at its fresh
 * confirmation (closed, or left for Google) says nothing: the dialog already did.
 */
export function adminFailure(title: string, error: Error): void {
  if (stoppedAtConfirmation(error)) {
    return;
  }

  toast({ title, description: error.message, tone: "danger" });
}

/** Uploads `file` directly and answers its signed id, for a write that attaches it. */
export async function uploaded(file: File): Promise<string> {
  const task = new UploadTask(
    file,
    browserDeps(
      actions.messages.startUpload,
      store.getState().boot?.account.uploadLimitBytes ?? DEFAULT_UPLOAD_LIMIT_BYTES,
    ),
    () => undefined,
  );

  await task.start();

  const { phase, signedId, error } = task.snapshot;

  if (phase !== "done" || signedId === null) {
    throw new Error(error ?? "The upload didn't finish.");
  }

  return signedId;
}

/**
 * The unsaved edit kept under `key` across a Google confirmation's round trip, taken (and
 * forgotten) once; `null` when there's none.
 */
export function takeDraft(key: string): string | null {
  try {
    const draft = sessionStorage.getItem(key);

    sessionStorage.removeItem(key);

    return draft;
  } catch {
    return null;
  }
}

/**
 * Keeps `draft` under `key` for the page to restore after a Google confirmation's round trip: for
 * a write that carries a credential, so isn't kept itself. Never put the credential in `draft`.
 */
export function keepDraft(key: string, draft: string): void {
  try {
    sessionStorage.setItem(key, draft);
  } catch {
    // Without storage the edit is lost with the page, as on the classic form.
  }
}

/**
 * Copies `text`, saying so. Where there is no clipboard (an origin that isn't secure) or it
 * refuses, `shown` (the element showing the text) is selected for a manual copy instead.
 */
export function copy(text: string, what: string, shown?: HTMLElement | null): void {
  const manual = () => {
    selectContents(shown);
    toast({ title: "Couldn't copy — select it manually", tone: "danger" });
  };

  try {
    void navigator.clipboard
      .writeText(text)
      .then(() => toast({ title: `${what} copied`, tone: "success" }), manual);
  } catch {
    manual();
  }
}

/** Selects everything inside `element`, for the reader to copy themselves. */
function selectContents(element: HTMLElement | null | undefined): void {
  const selection = window.getSelection();

  if (!element || !selection) {
    return;
  }

  const range = document.createRange();
  range.selectNodeContents(element);
  selection.removeAllRanges();
  selection.addRange(range);
}

/** A confirmation, as the classic pages' `turbo_confirm` asks it. */
export function Confirm({
  ask,
  onCancel,
}: {
  readonly ask: {
    readonly title: string;
    readonly message: string;
    readonly label: string;
    readonly danger: boolean;
    readonly run: () => void;
  } | null;
  readonly onCancel: () => void;
}) {
  return (
    <Dialog
      open={ask !== null}
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
      role="alertdialog"
      size="sm"
      title={ask?.title ?? ""}
      description={ask?.message}
      footer={
        <>
          <Button variant="secondary" onClick={onCancel} data-autofocus>
            Cancel
          </Button>
          <Button
            variant={ask?.danger === true ? "danger" : "primary"}
            onClick={() => {
              onCancel();
              ask?.run();
            }}
          >
            {ask?.label ?? "OK"}
          </Button>
        </>
      }
    />
  );
}

/** A section only administrators may open, for anyone else who lands on its URL. */
export function AdministratorsOnly() {
  return (
    <p className="settings-callout text-muted" role="status">
      Only administrators can see this page.
    </p>
  );
}

/**
 * How the admin lists mark their rows for features/destinations' row focus: rows carry `data-row`
 * (and `tabIndex={-1}`), their controls `data-row-control`, inside an `.admin-focus-root`.
 */
const ADMIN_ROW_PARTS: RowParts = {
  list: ".admin-focus-root",
  row: "[data-row]",
  open: "[data-row-control]",
  leaving: "[data-motion='leave']",
  id: "data-row",
  control: "data-row-control",
};

/**
 * Keeps focus in an admin list as its rows change (features/destinations/row-focus.ts). A removed
 * row hands focus to the same control in the next row, else the previous, else the container; a
 * row that remounts or moves to the other list (a role change) keeps it on the control it was in;
 * a control that goes away (a revoked credential's Revoke) leaves it on its row. Put the returned
 * ref on the `.admin-focus-root` container around the rows.
 */
export function useRowFocus() {
  const container = useRef<HTMLDivElement | null>(null);

  useKeepRowFocus(container, ADMIN_ROW_PARTS);

  return container;
}

/**
 * A table phones show as a stack of cards (admin.css): a row is a card, its `title` cell first,
 * every other cell a "label value" line. A table whose display is changed loses its semantics in
 * Chromium and Safari, so each part restates its role, and the column headers stay for screen
 * readers (only visually hidden there): every value keeps its header.
 */
export function CardTable({
  columns,
  caption,
  children,
}: {
  readonly columns: readonly string[];
  readonly caption?: string;
  readonly children: ReactNode;
}) {
  return (
    // biome-ignore lint/a11y/noRedundantRoles: phones restyle the table as cards, which drops the implicit role
    <table className="admin-audit-table" data-cards role="table">
      {caption === undefined ? null : <caption className="visually-hidden">{caption}</caption>}
      {/* biome-ignore lint/a11y/noRedundantRoles: as the table's */}
      <thead role="rowgroup">
        {/* biome-ignore lint/a11y/noRedundantRoles: as the table's */}
        <tr role="row">
          {columns.map((column) => (
            // biome-ignore lint/a11y/noRedundantRoles: as the table's
            <th key={column} scope="col" role="columnheader">
              {column}
            </th>
          ))}
        </tr>
      </thead>
      {/* biome-ignore lint/a11y/noRedundantRoles: as the table's */}
      <tbody role="rowgroup">{children}</tbody>
    </table>
  );
}

/** A row of a CardTable: a card on phones. */
export function CardRow({ children }: { readonly children: ReactNode }) {
  // biome-ignore lint/a11y/noRedundantRoles: as CardTable's
  return <tr role="row">{children}</tr>;
}

/**
 * A cell of a CardTable, under `column`: on phones a line labelled with it, or with `title`, the
 * card's heading.
 */
export function CardCell({
  column,
  title = false,
  className,
  children,
}: {
  readonly column: string;
  readonly title?: boolean;
  readonly className?: string;
  readonly children: ReactNode;
}) {
  return (
    <td
      // biome-ignore lint/a11y/noRedundantRoles: as CardTable's
      role="cell"
      className={className}
      data-label={title ? undefined : column}
      data-title={title || undefined}
    >
      {children}
    </td>
  );
}
