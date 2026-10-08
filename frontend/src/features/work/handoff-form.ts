/**
 * The handoff form's rules (`WorkHandoff#validate`, `work_threads#create_handoff`): what the form
 * checks before sending, and how the server's refusals read under their fields.
 */
import type { CreateWorkHandoff } from "../../gen/CreateWorkHandoff.ts";
import { ActionError } from "../../sync/run.ts";

/** The summary's limit, as the classic form's `maxlength` and the model have it. */
export const SUMMARY_MAX = 2000;

/** How many links or open questions a handoff carries. */
export const PACKAGE_MAX = 10;

/** How long each link or open question may be. */
export const ENTRY_MAX = 500;

/** The form's fields by wire name; `base` is the form as a whole. */
export type HandoffField = "receiverAgentId" | "summary" | "links" | "openQuestions" | "base";

export type HandoffProblems = Readonly<Partial<Record<HandoffField, string>>>;

export interface HandoffDraft {
  /** The agent's id as the select holds it; "" for none chosen. */
  readonly receiverAgentId: string;
  readonly summary: string;
  /** One per line. */
  readonly links: string;
  readonly openQuestions: string;
}

export const EMPTY_DRAFT: HandoffDraft = {
  receiverAgentId: "",
  summary: "",
  links: "",
  openQuestions: "",
};

/** One entry per line, trimmed, blanks and repeats dropped, as the server stores them. */
export function lines(text: string): string[] {
  return [
    ...new Set(
      text
        .split(/[\r\n]+/)
        .map((line) => line.trim())
        .filter((line) => line !== ""),
    ),
  ];
}

function isHttpUrl(value: string): boolean {
  return /^https?:\/\//i.test(value);
}

function charCount(value: string): number {
  return [...value].length;
}

/** What the form can tell before asking the server, worded as the server words it. */
export function localProblems(draft: HandoffDraft): HandoffProblems {
  const found: Partial<Record<HandoffField, string>> = {};
  const links = lines(draft.links);
  const questions = lines(draft.openQuestions);

  if (draft.receiverAgentId === "") {
    found.receiverAgentId = "Choose the agent to hand this work to.";
  }

  if (draft.summary.trim() === "") {
    found.summary = "Summary can't be blank.";
  } else if (charCount(draft.summary) > SUMMARY_MAX) {
    found.summary = `Summary is too long (maximum is ${SUMMARY_MAX} characters).`;
  }

  if (links.length > PACKAGE_MAX) {
    found.links = `Links are limited to ${PACKAGE_MAX} per handoff.`;
  } else if (links.some((link) => charCount(link) > ENTRY_MAX)) {
    found.links = `Links must be at most ${ENTRY_MAX} characters each.`;
  } else if (links.some((link) => !isHttpUrl(link))) {
    found.links = "Links must be http(s) URLs.";
  }

  if (questions.length > PACKAGE_MAX) {
    found.openQuestions = `Open questions are limited to ${PACKAGE_MAX} per handoff.`;
  } else if (questions.some((question) => charCount(question) > ENTRY_MAX)) {
    found.openQuestions = `Open questions must be at most ${ENTRY_MAX} characters each.`;
  }

  return found;
}

const FIELD_LABEL: Readonly<Record<Exclude<HandoffField, "base">, string>> = {
  receiverAgentId: "Receiver",
  summary: "Summary",
  links: "Links",
  openQuestions: "Open questions",
};

function isField(field: string): field is Exclude<HandoffField, "base"> {
  return field in FIELD_LABEL;
}

/** "can't be blank" under Summary reads "Summary can't be blank."; whole sentences stay. */
function sentence(field: Exclude<HandoffField, "base">, message: string): string {
  const text = /^[A-Z]/.test(message) ? message : `${FIELD_LABEL[field]} ${message}`;

  return /[.!?]$/.test(text) ? text : `${text}.`;
}

/**
 * A refused handoff as the form shows it: each `Validation` field's messages under that field
 * (the receiver's are whole sentences already), and anything else (an untracked thread's `base`,
 * a 403, a lost connection) as the form's alert, with the server's own words.
 */
export function serverProblems(error: Error): HandoffProblems {
  const found: Partial<Record<HandoffField, string>> = {};
  const fields = error instanceof ActionError ? error.fields : {};

  for (const [field, messages] of Object.entries(fields)) {
    if (messages.length === 0) {
      continue;
    }

    if (isField(field)) {
      found[field] = messages.map((message) => sentence(field, message)).join(" ");
    } else {
      found.base = [found.base, ...messages].filter((each) => each !== undefined).join(" ");
    }
  }

  if (Object.keys(found).length === 0) {
    found.base = error.message === "" ? "The work couldn't be handed off." : error.message;
  }

  return found;
}

/** The request body from a draft that passed `localProblems`. */
export function handoffBody(draft: HandoffDraft): CreateWorkHandoff {
  return {
    receiverAgentId: Number(draft.receiverAgentId),
    summary: draft.summary,
    links: lines(draft.links),
    openQuestions: lines(draft.openQuestions),
  };
}
