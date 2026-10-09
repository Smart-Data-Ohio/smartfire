/**
 * The handoff form's rules, as the server checks them (`work_handoffs`): a summary of at most
 * 2000 characters, up to 10 http(s) links and up to 10 open questions, each at most 500
 * characters. The messages are the server's, so a refusal reads the same either way. A
 * `Validation` reply's messages are shown on the field they name.
 */
import type { CreateWorkHandoff } from "../../gen/CreateWorkHandoff.ts";
import { ActionError } from "../../sync/run.ts";

export const SUMMARY_LIMIT = 2000;

/** At most this many links, and as many open questions. */
export const HANDOFF_ITEM_LIMIT = 10;

/** Each link or question is at most this long. */
export const HANDOFF_ITEM_LENGTH = 500;

/** What the person typed. */
export interface HandoffDraft {
  readonly receiverAgentId: number | null;
  readonly summary: string;
  /** One URL per line. */
  readonly links: string;
  /** One question per line. */
  readonly openQuestions: string;
}

/** The fields that can fail. */
export type HandoffField = "receiver" | "summary" | "links" | "openQuestions";

/** A message per field that fails. */
export type HandoffErrors = Readonly<Partial<Record<HandoffField, string>>>;

export const EMPTY_HANDOFF: HandoffDraft = {
  receiverAgentId: null,
  summary: "",
  links: "",
  openQuestions: "",
};

/** The non-blank lines, trimmed, as the server splits a textarea. */
export function handoffLines(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

const HTTP_URL = /^https?:\/\/\S+$/i;

function linksError(links: readonly string[]): string | undefined {
  if (links.length > HANDOFF_ITEM_LIMIT) {
    return `Links are limited to ${HANDOFF_ITEM_LIMIT} per handoff`;
  }

  if (links.some((link) => link.length > HANDOFF_ITEM_LENGTH)) {
    return `Links must be at most ${HANDOFF_ITEM_LENGTH} characters each`;
  }

  return links.every((link) => HTTP_URL.test(link)) ? undefined : "Links must be http(s) URLs";
}

function questionsError(questions: readonly string[]): string | undefined {
  if (questions.length > HANDOFF_ITEM_LIMIT) {
    return `Open questions are limited to ${HANDOFF_ITEM_LIMIT} per handoff`;
  }

  return questions.some((question) => question.length > HANDOFF_ITEM_LENGTH)
    ? `Open questions must be at most ${HANDOFF_ITEM_LENGTH} characters each`
    : undefined;
}

function summaryError(summary: string): string | undefined {
  if (summary === "") {
    return "Summary can't be blank";
  }

  return summary.length > SUMMARY_LIMIT
    ? `Summary is too long (maximum is ${SUMMARY_LIMIT} characters)`
    : undefined;
}

/**
 * Checks the draft: the request to send, or what's wrong with it. A missing receiver reads as
 * the server's "Receiver must be…" would, since the server answers the same for none.
 */
export function checkHandoff(
  draft: HandoffDraft,
): { readonly body: CreateWorkHandoff } | { readonly errors: HandoffErrors } {
  const summary = draft.summary.trim();
  const links = handoffLines(draft.links);
  const openQuestions = handoffLines(draft.openQuestions);

  const errors: Partial<Record<HandoffField, string>> = {};

  const problems = {
    summary: summaryError(summary),
    links: linksError(links),
    openQuestions: questionsError(openQuestions),
  };

  if (draft.receiverAgentId === null) {
    errors.receiver = "Choose the agent to hand off to";
  }

  for (const field of ["summary", "links", "openQuestions"] as const) {
    const problem = problems[field];

    if (problem !== undefined) {
      errors[field] = problem;
    }
  }

  if (draft.receiverAgentId === null || Object.keys(errors).length > 0) {
    return { errors };
  }

  return { body: { receiverAgentId: draft.receiverAgentId, summary, links, openQuestions } };
}

const FIELD_LABEL: Record<HandoffField, string> = {
  receiver: "Receiver",
  summary: "Summary",
  links: "Links",
  openQuestions: "Open questions",
};

/** Wire fields the form can show under its own control. A receiver policy error names the id. */
function formField(field: string): HandoffField | undefined {
  switch (field) {
    case "summary":
    case "links":
    case "openQuestions":
      return field;
    case "receiver":
    case "receiverAgent":
    case "receiverAgentId":
      return "receiver";
    default:
      return undefined;
  }
}

/** "can't be blank" under Summary reads "Summary can't be blank."; a whole sentence stays. */
function serverSentence(field: HandoffField, message: string): string {
  const text = /^[A-Z]/.test(message) ? message : `${FIELD_LABEL[field]} ${message}`;

  return /[.!?]$/.test(text) ? text : `${text}.`;
}

/** Field messages, plus a form-level alert when the refusal names no field. */
export interface ServerHandoffProblems {
  readonly errors: HandoffErrors;
  readonly alert: string | null;
}

/**
 * A refused handoff as the form shows it: field messages on their fields, anything else as
 * the alert.
 */
export function serverHandoffErrors(error: Error): ServerHandoffProblems {
  const fields = error instanceof ActionError ? error.fields : {};
  const errors: Partial<Record<HandoffField, string>> = {};
  const extra: string[] = [];

  for (const [field, messages] of Object.entries(fields)) {
    if (messages.length === 0) {
      continue;
    }

    const mapped = formField(field);

    if (mapped === undefined) {
      extra.push(...messages);
    } else {
      errors[mapped] = messages.map((message) => serverSentence(mapped, message)).join(" ");
    }
  }

  if (Object.keys(errors).length === 0 && extra.length === 0) {
    return {
      errors,
      alert: error.message === "" ? "The work couldn't be handed off." : error.message,
    };
  }

  return { errors, alert: extra.length === 0 ? null : extra.join(" ") };
}
