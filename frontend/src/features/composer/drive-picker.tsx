import { type KeyboardEvent, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { DriveFile } from "../../gen/DriveFile.ts";
import type { DriveRecipient } from "../../gen/DriveRecipient.ts";
import type { DriveShare } from "../../gen/DriveShare.ts";
import { postClassicForm } from "../../lib/classic-form.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { BrandMark } from "../cards/brand-marks.tsx";
import {
  CONFIRMATION_MESSAGE,
  type DrivePick,
  driveDisconnected,
  driveSearchStatus,
  grantCapacity,
  grantEnabled,
  moveActive,
  pickerKeyAction,
  searchDelay,
  shareBlockedMessage,
  shareResultLabel,
  shareSummary,
} from "./drive-picker.ts";

interface DrivePickerProps {
  readonly roomId: number;
  /** Drive files already on this message, so a full message is refused before any grant. */
  readonly attachedFileIds: readonly string[];
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly onAttach: (file: DrivePick) => void;
}

function pickOf(file: DriveFile): DrivePick | null {
  if (file.id === null) {
    return null;
  }

  return { id: file.id, name: file.name ?? "Google Drive file", kind: file.kind, url: file.url };
}

/**
 * Search Drive from the composer, then attach the file or grant the room view access first.
 * Not connected uses the same `POST /google/connect` link as settings ("Enable Drive previews").
 */
export function DrivePicker({
  roomId,
  attachedFileIds,
  open,
  onOpenChange,
  onAttach,
}: DrivePickerProps) {
  const listId = useId();
  const [query, setQuery] = useState("");
  const [files, setFiles] = useState<readonly DrivePick[]>([]);
  const [active, setActive] = useState(-1);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<ActionError | null>(null);
  const [review, setReview] = useState<DrivePick | null>(null);
  const [recipients, setRecipients] = useState<readonly DriveRecipient[]>([]);
  const [chosen, setChosen] = useState<ReadonlySet<string>>(new Set());
  const [granting, setGranting] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [report, setReport] = useState<DriveShare["results"] | null>(null);
  const subsequent = useRef(false);
  const request = useRef(0);
  const searchRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (open) {
      searchRef.current?.focus();
    }
  }, [open]);

  useEffect(() => {
    if (!open) {
      subsequent.current = false;
      setQuery("");
      setFiles([]);
      setActive(-1);
      setError(null);

      return;
    }

    const current = ++request.current;
    const delay = searchDelay(subsequent.current);

    subsequent.current = true;
    setSearching(true);

    const handle = window.setTimeout(() => {
      void actions.drive.search(query).then(
        (list) => {
          if (current !== request.current) {
            return;
          }

          setSearching(false);
          setError(null);
          setFiles(
            list.files.flatMap((file) => {
              const pick = pickOf(file);

              return pick === null ? [] : [pick];
            }),
          );
          setActive(-1);
        },
        (failure: ActionError) => {
          if (current !== request.current) {
            return;
          }

          setSearching(false);
          setError(failure);
          setFiles([]);
          setActive(-1);
        },
      );
    }, delay);

    return () => window.clearTimeout(handle);
  }, [open, query]);

  const close = () => onOpenChange(false);

  useLayoutEffect(() => {
    if (active < 0) return;

    document.getElementById(`${listId}-${active}`)?.scrollIntoView({ block: "nearest" });
  }, [active, listId]);

  const choose = (file: DrivePick) => {
    setReview(file);
    setChosen(new Set());
    setRecipients([]);
    setNotice(null);
    setReport(null);
    void actions.drive.recipients(roomId).then(
      (list) => setRecipients(list.recipients),
      () => setRecipients([]),
    );
  };

  const attach = (file: DrivePick) => {
    onAttach(file);
    setReview(null);
    close();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    const action = pickerKeyAction(event.key);

    if (action === null) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    if (action === "close") {
      close();
    } else if (action === "next" || action === "previous") {
      setActive((index) => moveActive(index, action === "next" ? 1 : -1, files.length));
    } else if (active >= 0 && files[active] !== undefined) {
      choose(files[active]);
    }
  };

  const disconnected = error !== null && driveDisconnected(error);
  const status = searching ? "Searching Drive…" : driveSearchStatus(error, files.length);
  const attachOnlyKind = review?.kind === "folder" || review?.kind === "shortcut";
  const kindNotice = attachOnlyKind ? shareBlockedMessage(review.kind) : null;

  return (
    <>
      {open ? (
        <div className="drive-picker" role="dialog" aria-label="Find a Drive file">
          <div className="drive-picker-bar">
            <input
              className="drive-picker-search"
              placeholder="Search Drive files"
              aria-label="Search Drive files"
              role="combobox"
              aria-expanded={files.length > 0}
              aria-autocomplete="list"
              aria-controls={listId}
              aria-activedescendant={active >= 0 ? `${listId}-${active}` : undefined}
              ref={searchRef}
              autoComplete="off"
              autoCapitalize="off"
              spellCheck={false}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={onKeyDown}
            />
            <button
              type="button"
              className="drive-picker-dismiss"
              aria-label="Close Drive search"
              onClick={close}
            >
              Close
            </button>
          </div>
          {disconnected ? (
            <div className="drive-picker-disconnected">
              <p>Google Drive isn't connected.</p>
              <Button
                type="button"
                onClick={() => postClassicForm("/google/connect", [["features[]", "drive"]])}
              >
                Enable Drive previews
              </Button>
            </div>
          ) : (
            <div
              className="drive-picker-results"
              id={listId}
              role="listbox"
              aria-label="Drive files"
            >
              {files.map((file, index) => (
                <button
                  key={file.id}
                  type="button"
                  id={`${listId}-${index}`}
                  className="drive-option"
                  role="option"
                  aria-selected={index === active}
                  onMouseEnter={() => setActive(index)}
                  onClick={() => choose(file)}
                >
                  <BrandMark name="drive" size={18} />
                  <span className="drive-option-name">{file.name}</span>
                  <span className="drive-option-kind">{file.kind}</span>
                </button>
              ))}
            </div>
          )}
          {status === "" ? null : (
            <p className="drive-picker-status" role="status">
              {status}
            </p>
          )}
        </div>
      ) : null}
      <Dialog
        open={review !== null}
        onOpenChange={(next) => {
          if (!next) {
            setReview(null);
          }
        }}
        title="Share a Drive file"
        description="Access grants happen immediately and stay in place even if you do not send the message, or if a recipient later leaves this chat. Future members are not added automatically. Only view access is granted, and no email notifications are sent."
        size="sm"
        footer={
          <>
            <Button type="button" variant="ghost" onClick={() => setReview(null)}>
              Cancel
            </Button>
            <Button
              type="button"
              onClick={() => {
                if (review !== null) {
                  attach(review);
                }
              }}
            >
              Attach only
            </Button>
            {report === null ? (
              <Button
                type="button"
                variant="primary"
                disabled={!grantEnabled(chosen.size, review?.kind ?? "file") || granting}
                loading={granting}
                loadingLabel="Granting"
                onClick={() => {
                  if (review === null) return;

                  if (!grantCapacity(attachedFileIds, review.id)) {
                    setNotice(shareBlockedMessage("full"));

                    return;
                  }

                  const approved = recipients
                    .filter((member) => chosen.has(String(member.id)))
                    .map((member) => ({ id: String(member.id), email: member.email }));

                  setGranting(true);
                  setReport(null);
                  void actions.drive.share(roomId, review.id, approved, attachedFileIds).then(
                    (share) => {
                      setGranting(false);

                      if (share.outcome === "confirmation_required") {
                        const changed = new Set(share.changedIds.map(String));
                        const live = new Set(share.recipients.map((member) => String(member.id)));

                        setRecipients(share.recipients);
                        setChosen(
                          (current) =>
                            new Set([...current].filter((id) => live.has(id) && !changed.has(id))),
                        );
                        setNotice(CONFIRMATION_MESSAGE);

                        return;
                      }

                      if (share.outcome === "blocked") {
                        setNotice(shareBlockedMessage(share.blocked ?? "file"));

                        return;
                      }

                      if (share.outcome === "full") {
                        setNotice(shareBlockedMessage("full"));

                        return;
                      }

                      const failed = share.results.some((result) => result.status === "failed");

                      onAttach(review);

                      if (failed) {
                        setReport(share.results);
                        setNotice(shareSummary(share.results));

                        return;
                      }

                      setReview(null);
                      close();
                    },
                    (failure: ActionError) => {
                      setGranting(false);
                      setNotice(failure.message);
                    },
                  );
                }}
              >
                Grant view access and attach
              </Button>
            ) : (
              <Button
                type="button"
                variant="primary"
                onClick={() => {
                  setReview(null);
                  close();
                }}
              >
                Done
              </Button>
            )}
          </>
        }
      >
        {notice !== null || kindNotice !== null ? (
          <p className="drive-picker-notice" role="status">
            {notice ?? kindNotice}
          </p>
        ) : null}
        {report === null ? null : (
          <ul className="drive-share-results">
            {report.map((result) => (
              <li key={result.recipient.id}>
                {result.recipient.name}{" "}
                <span className="drive-recipient-email">
                  {result.recipient.email} · {shareResultLabel(result.status, result.reason)}
                </span>
              </li>
            ))}
          </ul>
        )}
        <ul className="drive-recipients">
          {recipients.map((member) => (
            <li key={member.id}>
              <Checkbox
                checked={chosen.has(String(member.id))}
                label={
                  <>
                    {member.name} <span className="drive-recipient-email">{member.email}</span>
                  </>
                }
                onCheckedChange={(checked) => {
                  setChosen((current) => {
                    const next = new Set(current);

                    if (checked) {
                      next.add(String(member.id));
                    } else {
                      next.delete(String(member.id));
                    }

                    return next;
                  });
                }}
              />
            </li>
          ))}
        </ul>
      </Dialog>
    </>
  );
}

/** The files pinned on the message that hasn't been sent yet. */
export function DrivePendingChips({
  files,
  onRemove,
}: {
  readonly files: readonly DrivePick[];
  readonly onRemove: (id: string) => void;
}) {
  if (files.length === 0) {
    return null;
  }

  return (
    <div className="composer-drive-files" aria-live="polite">
      {files.map((file) => (
        <span key={file.id} className="card drive-chip">
          <span className="drive-icon" aria-hidden="true">
            <BrandMark name="drive" size={18} />
          </span>
          <span className="drive-text">
            <span className="drive-title">{file.name}</span>
            <span className="card-subtle">Google Drive</span>
          </span>
          <button
            type="button"
            className="drive-remove"
            aria-label={`Remove ${file.name}`}
            onClick={() => onRemove(file.id)}
          >
            ×
          </button>
        </span>
      ))}
    </div>
  );
}
