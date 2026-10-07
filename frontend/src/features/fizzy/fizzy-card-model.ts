import type { CreatedFizzyCard } from "../../gen/CreatedFizzyCard.ts";
import type { CreateFizzyCard } from "../../gen/CreateFizzyCard.ts";
import type { FizzyMessageCardForm } from "../../gen/FizzyMessageCardForm.ts";
import { ActionError } from "../../sync/run.ts";

/** The source message: its room, its thread for a reply (`null` on the room's timeline), its id. */
export interface FizzyMessageScope {
  readonly roomId: number;
  readonly threadId: number | null;
  readonly messageId: number;
}

/** The classic form's limits (`maxlength` on its title and description). */
export const TITLE_MAX_LENGTH = 500;

export const DESCRIPTION_MAX_LENGTH = 50_000;

/** The classic validation message, and its per-field messages (the API's `fields`). */
export const VALIDATION_SUMMARY = "Choose a board and enter a title.";

const FIELD_MESSAGE = { boardId: "Choose a board.", title: "Enter a title." } as const;

export type FizzyField = keyof typeof FIELD_MESSAGE;

export type FieldErrors = Partial<Record<FizzyField, string>>;

export interface FizzyDraft {
  readonly boardId: string;
  readonly title: string;
  readonly description: string;
}

/** What creating the card needs, as reading the form tells the dialog. */
export type ReadForm = (scope: FizzyMessageScope) => Promise<FizzyMessageCardForm>;

export type CreateCard = (
  scope: FizzyMessageScope,
  body: CreateFizzyCard,
) => Promise<CreatedFizzyCard>;

/** The form, and why it's disconnected when a rejected token just disconnected it. */
export interface LoadedForm {
  readonly form: FizzyMessageCardForm;
  readonly notice: string | null;
}

/** One key per source message, room or thread. */
export function scopeKey(scope: FizzyMessageScope): string {
  return `${scope.roomId}/${scope.threadId ?? "-"}/${scope.messageId}`;
}

/** The classic form's defaults: no board chosen, the server's title and description. */
export function initialDraft(form: FizzyMessageCardForm): FizzyDraft {
  return { boardId: "", title: form.title, description: form.description };
}

/** The fields the browser requires before anything is sent (classic's `required`). */
export function localErrors(draft: FizzyDraft): FieldErrors {
  const errors: FieldErrors = {};

  if (draft.boardId.trim() === "") errors.boardId = FIELD_MESSAGE.boardId;

  if (draft.title.trim() === "") errors.title = FIELD_MESSAGE.title;

  return errors;
}

export function hasErrors(errors: FieldErrors): boolean {
  return errors.boardId !== undefined || errors.title !== undefined;
}

/** The field to focus after a failed check: the first one with an error, in form order. */
export function firstInvalid(errors: FieldErrors): FizzyField | null {
  if (errors.boardId !== undefined) return "boardId";

  return errors.title === undefined ? null : "title";
}

/** A `Validation` error's `fields`, one message per field the form shows. */
export function fieldErrors(fields: Readonly<Record<string, readonly string[]>>): FieldErrors {
  const board = fields.boardId?.[0];
  const title = fields.title?.[0];

  const errors: FieldErrors = {};

  if (board !== undefined) errors.boardId = board;

  if (title !== undefined) errors.title = title;

  return errors;
}

export function createBody(draft: FizzyDraft): CreateFizzyCard {
  return { boardId: draft.boardId, title: draft.title, description: draft.description };
}

/** The tags that mean the viewer has no usable Fizzy connection (any more). */
function disconnects(tag: string): boolean {
  return tag === "FizzyNotConnected" || tag === "FizzyTokenRejected";
}

/** What the dialog does with a failed create. It never sends the card again on its own. */
export type CreateFailure =
  | { readonly kind: "validation"; readonly errors: FieldErrors; readonly summary: string }
  | { readonly kind: "disconnected"; readonly message: string }
  | {
      readonly kind: "created";
      readonly message: string;
      readonly card: { readonly number: string; readonly url: string };
    }
  | { readonly kind: "problem"; readonly message: string };

export function classifyFailure(failure: Error): CreateFailure {
  if (!(failure instanceof ActionError)) {
    return { kind: "problem", message: failure.message };
  }

  if (failure.tag === "Validation") {
    return { kind: "validation", errors: fieldErrors(failure.fields), summary: failure.message };
  }

  if (disconnects(failure.tag)) {
    return { kind: "disconnected", message: failure.message };
  }

  if (failure.tag === "FizzyReplyFailed" && failure.createdCard !== null) {
    return { kind: "created", message: failure.message, card: failure.createdCard };
  }

  return { kind: "problem", message: failure.message };
}

/**
 * Reads the form. A token Fizzy rejects disconnects the account as the form is read; reading it
 * again (a read is safe to repeat) then brings the source to show beside the reason.
 */
export async function readForm(read: ReadForm, scope: FizzyMessageScope): Promise<LoadedForm> {
  try {
    return { form: await read(scope), notice: null };
  } catch (failure) {
    if (!(failure instanceof ActionError && disconnects(failure.tag))) throw failure;

    const form = await read(scope);

    return { form: { ...form, connected: false, boards: [] }, notice: failure.message };
  }
}
