import { useNavigate } from "@tanstack/react-router";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { useStore as useZustand } from "zustand";
import type { DirectCandidate } from "../../gen/DirectCandidate.ts";
import type { RoomForm } from "../../gen/RoomForm.ts";
import { useStore } from "../../store/store.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import { PeoplePicker } from "../directs/people-picker.tsx";
import { KindCards } from "./kind-cards.tsx";
import { newRoomPreset } from "./new-room-store.ts";
import {
  availableChannels,
  type Channel,
  channelOf,
  createBody,
  hasMemberList,
  kindOf,
  type ManagedKind,
  privacyChoice,
} from "./room-forms.ts";
import { RoomIconButton } from "./room-icon-button.tsx";
import "./rooms.css";

interface NewRoomDialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

type Step = "details" | "members";

/** Nobody is turned away for numbers: a room holds the whole workspace. */
const NO_LIMIT = Number.MAX_SAFE_INTEGER;

const TITLE = {
  text: "Create a channel",
  voice: "Create a voice channel",
  stage: "Create a stage",
  board: "Create a board",
} as const satisfies Record<Channel, string>;

const ROOM_WORD = {
  text: "channel",
  voice: "voice channel",
  stage: "stage",
  board: "board",
} as const satisfies Record<Channel, string>;

const MEMBERS_HINT = {
  closed: "Only the people you add can see this channel or its history.",
  voice: "The people you add see this channel in their sidebar and can drop in.",
  stage: "You host. The people you add join as listeners; you can bring them up to speak later.",
  board: "The people you add can see the board, post, and pick up tasks.",
} as const;

/** The create form for each kind, fetched as the dialog meets it, so its defaults are the server's. */
function useNewForms(open: boolean, kind: ManagedKind) {
  const [forms, setForms] = useState<Partial<Record<ManagedKind, RoomForm>>>({});
  const [error, setError] = useState<string | null>(null);
  const [tries, setTries] = useState(0);
  const loaded = forms[kind] !== undefined;

  // biome-ignore lint/correctness/useExhaustiveDependencies: a retry (`tries`) loads the form again
  useEffect(() => {
    if (!open || loaded) {
      return;
    }

    // A new kind or a retry starts clean; the error shows again only if this load fails too.
    setError(null);

    let live = true;

    actions.rooms.newForm(kind).then(
      (form) => {
        if (!live) return;

        setForms((current) => ({ ...current, [kind]: form }));
        setError(null);
      },
      (failure: Error) => {
        if (live) setError(failure.message);
      },
    );

    return () => {
      live = false;
    };
  }, [open, kind, loaded, tries]);

  // The first form that arrives says which types the workspace lets the viewer create.
  const any = forms.open ?? forms.closed ?? forms.voice ?? forms.stage ?? forms.board ?? null;

  return {
    form: forms[kind] ?? null,
    allowed: any?.allowedTypes ?? null,
    error,
    retry: () => setTries((count) => count + 1),
  };
}

function candidatesOf(form: RoomForm | null, viewerId: number): readonly DirectCandidate[] | null {
  if (form === null) {
    return null;
  }

  const agents = new Set(form.users.filter((user) => user.agent !== null).map((user) => user.id));

  return form.candidateIds
    .filter((id) => id !== viewerId)
    .map((userId) => ({ userId, agent: agents.has(userId), starred: false }));
}

/**
 * Create a room (the sidebar's "+", its workspace menu, the PWA shortcut, classic "new room"
 * links): pick the kind as a card, name it and give it an icon, choose whether a text channel is
 * private, then (for every kind but a public channel) who's in it. Opens the room it made.
 */
export default function NewRoomDialog({ open, onOpenChange }: NewRoomDialogProps) {
  const navigate = useNavigate();
  const formId = useId();
  const preset = useZustand(newRoomPreset, (state) => state.kind);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? 0);
  const [wasOpen, setWasOpen] = useState(false);
  const [channel, setChannel] = useState<Channel>("text");
  const [isPrivate, setPrivate] = useState(false);
  const [name, setName] = useState("");
  const [iconName, setIconName] = useState<string | null>(null);
  const [members, setMembers] = useState<readonly number[]>([]);
  const [step, setStep] = useState<Step>("details");
  const [direction, setDirection] = useState<"forward" | "back">("forward");
  const [busy, setBusy] = useState(false);
  // One key per opening: retrying a create whose reply was lost gets the room it made, not another.
  const [clientRoomId, setClientRoomId] = useState(() => crypto.randomUUID());

  const [problem, setProblem] = useState<{ field: "iconName" | "other"; message: string } | null>(
    null,
  );

  const [attempt, setAttempt] = useState(0);
  const stepRef = useRef<HTMLDivElement | null>(null);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setChannel(channelOf(preset));
      setPrivate(preset === "closed");
      setName("");
      setIconName(null);
      setMembers([]);
      setStep("details");
      setDirection("forward");
      setBusy(false);
      setProblem(null);
      setClientRoomId(crypto.randomUUID());
    }
  }

  const choice = useNewForms(open, kindOf(channel, isPrivate));
  const allowed = choice.allowed;

  const available: readonly Channel[] =
    allowed === null ? ["text", "voice", "stage", "board"] : availableChannels(allowed);

  const privacy = allowed === null ? "both" : privacyChoice(allowed);
  const effectivePrivate = privacy === "both" ? isPrivate : privacy === "closed";
  const kind = kindOf(channel, effectivePrivate);
  const { form } = choice;
  const needsMembers = hasMemberList(kind);
  // Nothing is created from a form that hasn't loaded: its default name is the server's.
  const ready = form !== null;
  const placeholder = form?.name ?? "";
  const candidates = candidatesOf(form, viewerId);

  // A step that comes in takes focus at its first field, as the dialog does when it opens.
  useLayoutEffect(() => {
    if (step === "members") {
      stepRef.current?.querySelector<HTMLElement>("[data-autofocus]")?.focus();
    }
  }, [step]);

  const fail = (failure: ActionError) => {
    setBusy(false);

    const iconProblem = failure.fields.iconName?.[0];

    if (failure.tag === "Validation" && iconProblem !== undefined) {
      setProblem({ field: "iconName", message: `That icon ${iconProblem}.` });
      setAttempt((count) => count + 1);
      setDirection("back");
      setStep("details");

      return;
    }

    if (failure.tag === "Validation") {
      setProblem({ field: "other", message: failure.message });

      return;
    }

    toast({ title: "Couldn't create the room", description: failure.message, tone: "danger" });
  };

  const create = () => {
    if (busy || form === null) {
      return;
    }

    setBusy(true);
    setProblem(null);

    const draft = { name, iconName, userIds: [viewerId, ...members] };
    const body = createBody(kind, draft, form.name, clientRoomId);

    actions.rooms.create(body).then((result) => {
      setBusy(false);
      onOpenChange(false);
      void navigate({ to: "/r/$roomId", params: { roomId: result.room.id } });
    }, fail);
  };

  const next = () => {
    if (!ready) {
      return;
    }

    if (needsMembers) {
      setDirection("forward");
      setStep("members");
    } else {
      create();
    }
  };

  const back = () => {
    setDirection("back");
    setStep("details");
  };

  const roomWord = ROOM_WORD[channel];
  const shownName = name.trim() === "" ? placeholder : name.trim();

  const footer =
    step === "details" ? (
      <>
        <Button variant="secondary" onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button
          type="submit"
          form={formId}
          variant="primary"
          loading={busy}
          loadingLabel="Creating…"
          disabled={!ready}
          {...(needsMembers ? { trailingIcon: "chevron-right" as const } : {})}
        >
          {needsMembers ? "Next" : `Create ${roomWord}`}
        </Button>
      </>
    ) : (
      <>
        <Button variant="ghost" icon="chevron-left" className="room-form-back" onClick={back}>
          Back
        </Button>
        <Button
          variant="primary"
          loading={busy}
          loadingLabel="Creating…"
          disabled={!ready}
          onClick={create}
        >
          {members.length === 0 ? `Create with just you` : `Create ${roomWord}`}
        </Button>
      </>
    );

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={step === "details" ? TITLE[channel] : "Add people"}
      description={
        step === "members" && kind !== "open" ? (
          <span className="room-form-description">
            <strong>{shownName}</strong> · {MEMBERS_HINT[kind]}
          </span>
        ) : undefined
      }
      footer={footer}
    >
      <div
        key={step}
        ref={stepRef}
        className="room-step"
        data-direction={direction}
        data-step={step}
      >
        {step === "details" ? (
          <form
            id={formId}
            className="room-form"
            noValidate
            onSubmit={(event) => {
              event.preventDefault();
              next();
            }}
          >
            <KindCards
              value={channel}
              available={available}
              onValueChange={(next) => {
                setChannel(next);
                setProblem(null);
              }}
            />
            <div className="room-identity">
              <RoomIconButton
                kind={kind}
                iconName={iconName}
                invalid={problem?.field === "iconName"}
                onChange={(next) => {
                  setIconName(next);
                  setProblem(null);
                }}
              />
              <TextField
                label="Name"
                className="room-name-input"
                value={name}
                placeholder={placeholder}
                autoComplete="off"
                spellCheck={false}
                maxLength={100}
                data-autofocus
                error={problem?.field === "iconName" ? problem.message : undefined}
                attempt={attempt}
                onChange={(event) => {
                  setName(event.target.value);
                }}
              />
            </div>
            {channel === "text" && privacy === "both" ? (
              <div className="room-privacy">
                <Toggle
                  checked={isPrivate}
                  onCheckedChange={setPrivate}
                  label={<span className="room-privacy-label">Private channel</span>}
                  description="Only the people you add can see it. Otherwise everyone in the workspace is in."
                />
              </div>
            ) : null}
            {choice.error === null ? null : (
              <p className="picker-note picker-error room-form-load-error" role="alert">
                <span>Couldn't load the form: {choice.error}</span>
                <Button variant="link" size="sm" onClick={choice.retry}>
                  Try again
                </Button>
              </p>
            )}
            {problem?.field === "other" ? (
              <p className="picker-note picker-error" role="alert">
                {problem.message}
              </p>
            ) : null}
          </form>
        ) : (
          <div className="room-members-step">
            <PeoplePicker
              candidates={candidates}
              error={choice.error}
              selected={members}
              fixed={[viewerId]}
              limit={NO_LIMIT}
              onSelectedChange={(next) => {
                setMembers(next);
                setProblem(null);
              }}
              onSubmit={create}
              label="Add:"
              placeholder="Type a name"
            />
            {problem?.field === "other" ? (
              <p className="picker-note picker-error" role="alert">
                {problem.message}
              </p>
            ) : null}
          </div>
        )}
      </div>
    </Dialog>
  );
}
