/** S4 errors follow the Rust API; the earlier mocks keep their own envelopes. */
import type { ApiError } from "../../src/gen/ApiError.ts";
import { HttpError } from "../http.ts";

const VALIDATION = "Validation" satisfies ApiError["_tag"];

function humanize(attribute: string): string {
  const words = attribute
    .replace(/_/g, " ")
    .replace(/[A-Z]/g, (letter) => ` ${letter.toLowerCase()}`);

  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** Model errors keep the attribute's name in the sentence and rename only the wire field. */
export function invalid(
  errors: readonly (readonly [string, string])[],
  rename: Readonly<Record<string, string>> = {},
): HttpError {
  const fields: Record<string, string[]> = {};

  for (const [attribute, message] of errors) {
    const field =
      rename[attribute] ??
      attribute.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase());

    fields[field] = [...(fields[field] ?? []), message];
  }

  return new HttpError(422, {
    _tag: VALIDATION,
    message: errors.map(([attribute, message]) => `${humanize(attribute)} ${message}`).join(", "),
    fields,
  });
}

/** A named field with a bare Rails validation message. */
export const validation = (field: string, message: string) => invalid([[field, message]]);

/** Classic refusals carry the whole sentence on base or receiverAgentId. */
export const sentence = (field: string, message: string) =>
  new HttpError(422, { _tag: VALIDATION, message, fields: { [field]: [message] } });

/** A body that serde cannot decode has no field errors. */
export const invalidBody = (detail: string) =>
  new HttpError(422, {
    _tag: VALIDATION,
    message: `The request body isn't valid: ${detail}`,
    fields: {},
  });
