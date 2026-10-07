import { useNavigate } from "@tanstack/react-router";
import { useEffect, useId, useState } from "react";
import type { RoomForm } from "../../gen/RoomForm.ts";
import { useStore } from "../../store/store.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { Tabs } from "../../ui/tabs.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import { MemberList } from "./member-list.tsx";
import {
  hasMemberList,
  isDirty,
  type ManagedKind,
  type RoomDraft,
  roomLabel,
  updateBody,
} from "./room-forms.ts";
import { RoomIconButton } from "./room-icon-button.tsx";
import "./rooms.css";

interface RoomSettingsDialogProps {
  readonly roomId: number;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly form: RoomForm };

type Tab = "general" | "members";

const NOUN = {
  open: "channel",
  closed: "channel",
  voice: "voice channel",
  stage: "stage",
  board: "board",
} as const satisfies Record<ManagedKind, string>;

function capitalised(text: string): string {
  return `${text.charAt(0).toUpperCase()}${text.slice(1)}`;
}

function managed(form: RoomForm): ManagedKind | null {
  return form.type === "direct" ? null : form.type;
}

function SettingsSkeleton() {
  return (
    <div className="room-settings-skeleton" aria-hidden="true">
      <span className="room-identity">
        <Skeleton width={44} height={44} radius="md" />
        <Skeleton width="70%" height={12} />
      </span>
      <Skeleton width="45%" height={10} />
      <Skeleton width="85%" height={10} />
    </div>
  );
}

/** The settings while they load, or why they didn't. */
function Pending({ load }: { readonly load: Load }) {
  return load.status === "error" ? (
    <p className="picker-note picker-error" role="alert">
      Couldn't load the settings: {load.message}
    </p>
  ) : (
    <SettingsSkeleton />
  );
}

/**
 * A room's settings (`/app/r/:id/settings`, the classic `rooms/<kind>/:id/edit` pages): its name
 * and icon, whether a text channel is private, who's in it (with stage roles), and Delete. The
 * creator and administrators can change them; everyone else reads them. Nothing is written until
 * "Save changes"; then the room's header, sidebar row and member list update in place.
 */
export default function RoomSettingsDialog({
  roomId,
  open,
  onOpenChange,
}: RoomSettingsDialogProps) {
  const navigate = useNavigate();
  const formId = useId();
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? 0);
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [kind, setKind] = useState<ManagedKind>("open");
  const [draft, setDraft] = useState<RoomDraft>({ name: "", iconName: null, userIds: [] });
  const [tab, setTab] = useState<Tab>("general");
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [iconError, setIconError] = useState<string | undefined>(undefined);
  const [problem, setProblem] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  // Each opening reads the room afresh: someone else may have changed it since.
  useEffect(() => {
    if (!open) {
      return;
    }

    let live = true;

    setLoad({ status: "loading" });
    setTab("general");
    setProblem(null);
    setIconError(undefined);
    actions.rooms.editForm(roomId).then(
      (form) => {
        if (!live) return;

        setLoad({ status: "ready", form });
        setKind(managed(form) ?? "open");
        setDraft({ name: form.name ?? "", iconName: form.iconName, userIds: form.userIds });
      },
      (failure: Error) => {
        if (live) setLoad({ status: "error", message: failure.message });
      },
    );

    return () => {
      live = false;
    };
  }, [roomId, open]);

  const form = load.status === "ready" ? load.form : null;
  const readOnly = form === null || !form.canSubmit;
  const dirty = form !== null && isDirty(form, kind, draft);
  const noun = NOUN[kind];
  const members = hasMemberList(kind);

  const canConvert =
    form !== null &&
    !readOnly &&
    (kind === "open" || kind === "closed") &&
    form.conversionTypes.includes(kind === "open" ? "closed" : "open");

  const agentIds = new Set(
    form?.users.filter((user) => user.agent !== null).map((user) => user.id),
  );

  const edit = (patch: Partial<RoomDraft>) => {
    setDraft((current) => ({ ...current, ...patch }));
    setProblem(null);

    if (patch.iconName !== undefined) setIconError(undefined);
  };

  const fail = (title: string) => (failure: ActionError) => {
    setBusy(false);
    setDeleting(false);

    const icon = failure.fields.iconName?.[0];

    if (failure.tag === "Validation" && icon !== undefined) {
      setTab("general");
      setIconError(`That icon ${icon}.`);
      setAttempt((count) => count + 1);
    } else if (failure.tag === "Validation") {
      setProblem(failure.message);
    } else {
      toast({ title, description: failure.message, tone: "danger" });
    }
  };

  const save = () => {
    if (form === null || readOnly || busy || !dirty) {
      return;
    }

    setBusy(true);
    actions.rooms.update(roomId, updateBody(form, kind, draft)).then((result) => {
      setBusy(false);
      onOpenChange(false);

      if (result.detail === null) {
        // They took themselves out: the room is gone from their sidebar.
        toast({ title: `You left ${roomLabel(result.room.kind, result.room.name ?? "the room")}` });
        void navigate({ to: "/" });
      } else {
        toast({ title: "Changes saved", tone: "success" });
      }
    }, fail("Couldn't save the changes"));
  };

  const remove = () => {
    if (form === null) return;

    setDeleting(true);
    actions.rooms.remove(roomId).then(
      () => {
        setDeleting(false);
        setConfirming(false);
        onOpenChange(false);
        toast({ title: `Deleted ${roomLabel(kind, form.name ?? form.displayName)}` });
        void navigate({ to: "/" });
      },
      fail(`Couldn't delete the ${noun}`),
    );
  };

  const title = `${capitalised(noun)} settings`;

  const description =
    form === null ? undefined : (
      <span className="room-form-description">{roomLabel(kind, form.displayName)}</span>
    );

  const footer = readOnly ? (
    <Button variant="secondary" onClick={() => onOpenChange(false)}>
      Close
    </Button>
  ) : (
    <>
      {dirty ? (
        <span className="room-form-dirty enter-fade" aria-live="polite">
          Unsaved changes
        </span>
      ) : null}
      <Button variant="secondary" onClick={() => onOpenChange(false)}>
        Cancel
      </Button>
      <Button
        type="submit"
        form={formId}
        variant="primary"
        disabled={!dirty}
        loading={busy}
        loadingLabel="Saving…"
      >
        Save changes
      </Button>
    </>
  );

  const general =
    form === null ? null : (
      <div className="room-settings-general">
        <div className="room-identity">
          <RoomIconButton
            kind={kind}
            iconName={draft.iconName}
            disabled={readOnly}
            invalid={iconError !== undefined}
            onChange={(iconName) => edit({ iconName })}
          />
          <TextField
            label="Name"
            className="room-name-input"
            value={draft.name}
            placeholder={form.displayName}
            autoComplete="off"
            spellCheck={false}
            maxLength={100}
            disabled={readOnly}
            data-autofocus={readOnly ? undefined : true}
            error={iconError}
            attempt={attempt}
            onChange={(event) => edit({ name: event.target.value })}
          />
        </div>
        {canConvert ? (
          <div className="room-privacy">
            <Toggle
              checked={kind === "closed"}
              onCheckedChange={(next) => {
                setKind(next ? "closed" : "open");
                setProblem(null);
              }}
              label="Private channel"
              description={
                kind === "closed"
                  ? "Only its members can see it. Manage who's in it under Members."
                  : "Everyone in the workspace is in it."
              }
            />
          </div>
        ) : null}
        {readOnly ? (
          <p className="room-readonly-note">
            Only the person who made this {noun} and administrators can change it.
          </p>
        ) : null}
        {form.canDelete ? (
          <section className="room-danger" aria-label="Delete">
            <div className="room-danger-text">
              <span className="room-danger-title">Delete this {noun}</span>
              <span className="room-danger-hint">
                Its messages, threads and files go for everyone. This can't be undone.
              </span>
            </div>
            <Button variant="danger" size="sm" icon="trash" onClick={() => setConfirming(true)}>
              Delete…
            </Button>
          </section>
        ) : null}
      </div>
    );

  const body =
    form === null ? (
      <Pending load={load} />
    ) : (
      <form
        id={formId}
        className="room-form"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          save();
        }}
      >
        {members ? (
          <Tabs
            items={[
              { value: "general", label: "General", icon: "settings" },
              { value: "members", label: `Members · ${draft.userIds.length}`, icon: "users" },
            ]}
            value={tab}
            onValueChange={(value) => setTab(value === "members" ? "members" : "general")}
            label="Settings sections"
          >
            <div key={tab} className="room-tab-panel enter-fade">
              {tab === "general" ? (
                general
              ) : (
                <MemberList
                  candidateIds={form.candidateIds}
                  memberIds={draft.userIds}
                  savedIds={form.userIds}
                  onChange={(userIds) => edit({ userIds })}
                  viewerId={viewerId}
                  stageRoles={form.stageRoles}
                  agentIds={agentIds}
                  readOnly={readOnly}
                />
              )}
            </div>
          </Tabs>
        ) : (
          general
        )}
        {problem === null ? null : (
          <p className="picker-note picker-error" role="alert">
            {problem}
          </p>
        )}
      </form>
    );

  return (
    <>
      <Dialog
        open={open}
        onOpenChange={onOpenChange}
        title={title}
        description={description}
        footer={footer}
      >
        {body}
      </Dialog>
      <Dialog
        open={confirming}
        onOpenChange={(next) => (deleting ? undefined : setConfirming(next))}
        role="alertdialog"
        size="sm"
        title={`Delete ${form === null ? noun : roomLabel(kind, form.displayName)}?`}
        description={`Everyone loses the ${noun} and everything in it. This can't be undone.`}
        footer={
          <>
            <Button variant="secondary" onClick={() => setConfirming(false)} data-autofocus>
              Keep it
            </Button>
            <Button variant="danger" loading={deleting} loadingLabel="Deleting…" onClick={remove}>
              Delete {noun}
            </Button>
          </>
        }
      />
    </>
  );
}
