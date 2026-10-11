/**
 * JSON values as they come off the wire, and the small readers the mock parses request bodies
 * and client frames with. The mock is plain TypeScript (no Effect Schema), so its boundary
 * parsing is done by hand here, once.
 */

/** Any JSON value. */
export type Json = null | boolean | number | string | readonly Json[] | JsonRecord;

/** A JSON object. */
export interface JsonRecord {
  readonly [key: string]: Json;
}

/** Whether `value` is a JSON string (`String` of anything else is a different value). */
export function isString(value: Json | undefined): value is string {
  return value !== undefined && String(value) === value;
}

/** Whether `value` is a JSON number (`Number` of anything else is a different value). */
export function isNumber(value: Json | undefined): value is number {
  return value !== undefined && Number(value) === value;
}

export function isBoolean(value: Json | undefined): value is boolean {
  return value === true || value === false;
}

export function isRecord(value: Json | undefined): value is JsonRecord {
  return (
    value !== undefined &&
    value !== null &&
    !Array.isArray(value) &&
    !isString(value) &&
    !isNumber(value) &&
    !isBoolean(value)
  );
}

/** A field of a JSON object, or `undefined` when `value` isn't an object or lacks it. */
export function field(value: Json | undefined, key: string): Json | undefined {
  return isRecord(value) ? value[key] : undefined;
}

/** An integer field, or `null` when absent or not an integer. */
export function intField(value: Json | undefined, key: string): number | null {
  const found = field(value, key);

  return isNumber(found) && Number.isInteger(found) ? found : null;
}

export function stringField(value: Json | undefined, key: string): string | null {
  const found = field(value, key);

  return isString(found) ? found : null;
}

export function booleanField(value: Json | undefined, key: string): boolean | null {
  const found = field(value, key);

  return isBoolean(found) ? found : null;
}

/** A string array field; `null` when absent or when any element isn't a string. */
export function stringArrayField(value: Json | undefined, key: string): string[] | null {
  const found = field(value, key);

  if (!Array.isArray(found)) return null;

  const strings: string[] = [];

  for (const item of found) {
    if (!isString(item)) return null;
    strings.push(item);
  }

  return strings;
}

/** `JSON.parse` that answers `undefined` for text that isn't JSON. */
export function parseJson(text: string): Json | undefined {
  try {
    const parsed: Json = JSON.parse(text);

    return parsed;
  } catch {
    return undefined;
  }
}

/**
 * A `multipart/form-data` body as the mock reads it: each text field as a string, each file as
 * `{ filename, contentType, byteSize }`. The one form the SPA posts (first run's avatar) is
 * otherwise JSON in a text field.
 */
export function formJson(form: FormData): JsonRecord {
  const record: Record<string, Json> = {};

  form.forEach((value, key) => {
    record[key] =
      value instanceof File
        ? { filename: value.name, contentType: value.type, byteSize: value.size }
        : value;
  });

  return record;
}

/** Whether a `Content-Type` names a multipart form. */
export function isMultipart(contentType: string | null | undefined): boolean {
  return contentType?.toLowerCase().startsWith("multipart/form-data") ?? false;
}
