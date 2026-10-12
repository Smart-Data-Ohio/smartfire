/**
 * Slash-command routing, as the classic composer does it (crates/api_types composer.rs):
 * `//text` posts `/text` as an ordinary message, `/play` uses normal message submission, a known
 * `/command` runs on the server, and an unknown `/word` is posted as typed.
 */
import type { CommandOption } from "../../gen/CommandOption.ts";
import type { CommandSchema } from "../../gen/CommandSchema.ts";
import type { CommandValues } from "../../gen/CommandValues.ts";
import type { OptionValue } from "../../gen/OptionValue.ts";
import type { SlashCommand } from "../../gen/SlashCommand.ts";
import type { Subcommand } from "../../gen/Subcommand.ts";

export type SlashRoute =
  | { readonly kind: "message"; readonly markdown: string }
  | { readonly kind: "command"; readonly command: SlashCommand; readonly text: string };

const COMMAND = /^\/([a-z0-9_-]+)(?=\s|$)/i;

/** The command name a message starts with (`/remind in 5m` -> `remind`), or `null`. */
export function slashName(text: string): string | null {
  if (text.startsWith("//")) {
    return null;
  }

  return COMMAND.exec(text)?.[1]?.toLowerCase() ?? null;
}

/** Whether sending this text needs the command list to decide where it goes. */
export function looksLikeCommand(text: string): boolean {
  const name = slashName(text.trim());

  return name !== null && name !== "play";
}

/** Where a message goes: posted as Markdown, or run as a command. */
export function routeSlash(text: string, commands: readonly SlashCommand[]): SlashRoute {
  const trimmed = text.trim();

  if (trimmed.startsWith("//")) {
    return { kind: "message", markdown: text.trimEnd().replace(/^(\s*)\//, "$1") };
  }

  const name = slashName(trimmed);

  const command =
    name === null || name === "play" ? undefined : commands.find((entry) => entry.name === name);

  return command === undefined
    ? { kind: "message", markdown: text.trimEnd() }
    : { kind: "command", command, text: trimmed };
}

/**
 * The commands matching what's typed after `/`: prefix matches first, then other substring
 * matches, each in the server's order.
 */
export function filterCommands(
  commands: readonly SlashCommand[],
  query: string,
): readonly SlashCommand[] {
  const needle = query.toLowerCase();
  const prefix: SlashCommand[] = [];
  const inside: SlashCommand[] = [];

  for (const command of commands) {
    if (command.name.startsWith(needle)) {
      prefix.push(command);
    } else if (needle !== "" && command.name.includes(needle)) {
      inside.push(command);
    }
  }

  return [...prefix, ...inside];
}

/** The usage line: `/remind <when> <text>`, or a typed command's `/deploy <run|stop>`. */
export function commandUsage(command: SlashCommand): string {
  const hint =
    command.argHint === "" && command.optionSchema !== null
      ? schemaUsage(command.optionSchema)
      : command.argHint;

  return hint === "" ? `/${command.name}` : `/${command.name} ${hint}`;
}

// --- Typed options (agent commands with an option schema) ---

/** A pick list's entry, or what a field has picked: the label shown and the value sent. */
export interface PickedValue {
  readonly label: string;
  readonly value: OptionValue;
}

/** One option's field: what's typed, and for a pick list (choices, yes/no, a person, a channel) the pick. */
export interface OptionInput {
  readonly text: string;
  readonly picked: PickedValue | null;
}

/** A 422's field errors mapped onto the form, as sentences. */
export interface OptionProblems {
  /** By option name. */
  readonly options: Readonly<Record<string, string>>;
  readonly subcommand: string | null;
  /** What no field owns: an unreadable schema, an option the form doesn't show. */
  readonly general: readonly string[];
}

/** What the client finds wrong with an entry, by field. */
export interface EntryErrors {
  readonly options: Readonly<Record<string, string>>;
  readonly subcommand: string | null;
}

/** A schema-backed command being filled in, Discord-style: a subcommand, then option fields. */
export interface CommandEntry {
  readonly command: SlashCommand;
  readonly schema: CommandSchema;
  /** The chosen subcommand: `null` until one is picked, and always for an options-only schema. */
  readonly subcommand: string | null;
  /** What's typed in the subcommand field. */
  readonly subcommandText: string;
  /** The option fields on screen, in schema order: the required ones and any optional ones added. */
  readonly shown: readonly string[];
  readonly inputs: Readonly<Record<string, OptionInput>>;
  /** Fields the author has left: their errors show. */
  readonly touched: readonly string[];
  /** A send was refused for errors: every error shows. */
  readonly attempted: boolean;
  /** The server's 422, each until the field it names is edited. */
  readonly server: OptionProblems;
}

export const NO_PROBLEMS: OptionProblems = { options: {}, subcommand: null, general: [] };

/** The subcommands of a schema that has them, else `null`. */
export function subcommandsOf(schema: CommandSchema): readonly Subcommand[] | null {
  return "subcommands" in schema ? schema.subcommands : null;
}

/** The options the entry asks for now; `null` while a subcommand still has to be picked. */
export function entryOptions(entry: CommandEntry): readonly CommandOption[] | null {
  if ("options" in entry.schema) {
    return entry.schema.options;
  }

  return entry.schema.subcommands.find((sub) => sub.name === entry.subcommand)?.options ?? null;
}

/** Required options take fields at once; with none required, every option does. */
function initiallyShown(options: readonly CommandOption[]): readonly string[] {
  const required = options.flatMap((option) => (option.required ? [option.name] : []));

  return required.length > 0 ? required : options.map((option) => option.name);
}

/** A fresh entry for a command with a schema. */
export function startEntry(command: SlashCommand, schema: CommandSchema): CommandEntry {
  const entry: CommandEntry = {
    command,
    schema,
    subcommand: null,
    subcommandText: "",
    shown: [],
    inputs: {},
    touched: [],
    attempted: false,
    server: NO_PROBLEMS,
  };

  const options = entryOptions(entry);

  return options === null ? entry : { ...entry, shown: initiallyShown(options) };
}

/** Types in the subcommand field: an exact name picks it; anything else un-picks. */
export function typeSubcommand(entry: CommandEntry, text: string): CommandEntry {
  const name = text.trim().toLowerCase();
  const match = subcommandsOf(entry.schema)?.find((sub) => sub.name === name)?.name ?? null;

  return { ...chooseSubcommand(entry, match), subcommandText: text };
}

/** Picks a subcommand (or none); a different one starts its options afresh. */
export function chooseSubcommand(entry: CommandEntry, name: string | null): CommandEntry {
  if (name === entry.subcommand) {
    return entry;
  }

  const next: CommandEntry = {
    ...entry,
    subcommand: name,
    subcommandText: name ?? entry.subcommandText,
    inputs: {},
    touched: [],
    attempted: false,
    server: { ...entry.server, options: {}, subcommand: null },
  };

  const options = entryOptions(next);

  return { ...next, shown: options === null ? [] : initiallyShown(options) };
}

/** Adds an optional option's field, keeping schema order. */
export function showOption(entry: CommandEntry, name: string): CommandEntry {
  const options = entryOptions(entry) ?? [];

  if (entry.shown.includes(name) || !options.some((option) => option.name === name)) {
    return entry;
  }

  const shown = options.flatMap((option) =>
    option.name === name || entry.shown.includes(option.name) ? [option.name] : [],
  );

  return { ...entry, shown };
}

/** Removes an optional option's field and what it held. Required fields stay. */
export function hideOption(entry: CommandEntry, name: string): CommandEntry {
  const option = (entryOptions(entry) ?? []).find((candidate) => candidate.name === name);

  if (option === undefined || option.required) {
    return entry;
  }

  const { [name]: _input, ...inputs } = entry.inputs;
  const { [name]: _problem, ...problems } = entry.server.options;

  return {
    ...entry,
    shown: entry.shown.filter((shown) => shown !== name),
    inputs,
    touched: entry.touched.filter((touched) => touched !== name),
    server: { ...entry.server, options: problems },
  };
}

/** Sets a field; the server's error for it goes, since it was about the old value. */
export function editOption(entry: CommandEntry, name: string, input: OptionInput): CommandEntry {
  const { [name]: _problem, ...problems } = entry.server.options;

  return {
    ...entry,
    inputs: { ...entry.inputs, [name]: input },
    server: { ...entry.server, options: problems },
  };
}

/** Marks a field as left, so its error shows. */
export function touchField(entry: CommandEntry, name: string): CommandEntry {
  return entry.touched.includes(name) ? entry : { ...entry, touched: [...entry.touched, name] };
}

/** The server refused it: its errors go on the fields, and every client error shows too. */
export function withProblems(entry: CommandEntry, problems: OptionProblems): CommandEntry {
  return { ...entry, attempted: true, server: problems };
}

const INTEGER = /^[+-]?\d+$/;

const DECIMAL = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?$/i;

const BOOLEAN_CHOICES: readonly PickedValue[] = [
  { label: "True", value: true },
  { label: "False", value: false },
];

/** A pick list's entries, for choices and yes/no; `null` for free entry, a person or a channel. */
export function optionChoices(option: CommandOption): readonly PickedValue[] | null {
  switch (option.type) {
    case "string":
    case "integer":
    case "number":
      return option.choices?.map((choice) => ({ label: choice.name, value: choice.value })) ?? null;
    case "boolean":
      return BOOLEAN_CHOICES;
    case "user":
    case "room":
      return null;
  }
}

/** Whether the field is a pick list rather than free entry. */
export function picksValue(option: CommandOption): boolean {
  return optionChoices(option) !== null || option.type === "user" || option.type === "room";
}

const BOOLEAN_WORDS = new Map([
  ["true", true],
  ["yes", true],
  ["false", false],
  ["no", false],
]);

/** A typed word that names a choice (by label or value, any case), as if it were picked. */
function typedChoice(option: CommandOption, text: string): PickedValue | null {
  const needle = text.trim().toLowerCase();

  if (needle === "") {
    return null;
  }

  if (option.type === "boolean") {
    const value = BOOLEAN_WORDS.get(needle);

    return BOOLEAN_CHOICES.find((choice) => choice.value === value) ?? null;
  }

  return (
    optionChoices(option)?.find(
      (choice) =>
        choice.label.toLowerCase() === needle || String(choice.value).toLowerCase() === needle,
    ) ?? null
  );
}

function between(min: number | undefined, max: number | undefined, unit: string): string {
  if (min !== undefined && max !== undefined) {
    return `Must be ${min} to ${max}${unit}.`;
  }

  return min !== undefined ? `Must be at least ${min}${unit}.` : `Must be at most ${max}${unit}.`;
}

interface OptionRead {
  readonly value: OptionValue | null;
  readonly error: string | null;
}

const EMPTY: OptionRead = { value: null, error: null };

function refused(error: string): OptionRead {
  return { value: null, error };
}

/** A field's value as the server takes it, or why it isn't one (mirrors slash_schema's checks). */
function readOption(option: CommandOption, input: OptionInput | undefined): OptionRead {
  const text = input?.text.trim() ?? "";
  const picked = input?.picked ?? typedChoice(option, text);

  if (picked === null && text === "") {
    return option.required ? refused("Required.") : EMPTY;
  }

  switch (option.type) {
    case "string": {
      if (option.choices !== undefined) {
        return picked !== null && option.choices.some((choice) => choice.value === picked.value)
          ? { value: picked.value, error: null }
          : refused("Pick one of the choices.");
      }

      const length = [...text].length;

      if (
        (option.min_length !== undefined && length < option.min_length) ||
        (option.max_length !== undefined && length > option.max_length)
      ) {
        return refused(between(option.min_length, option.max_length, " characters"));
      }

      return { value: text, error: null };
    }

    case "integer":
    case "number": {
      if (option.choices !== undefined) {
        return picked !== null && option.choices.some((choice) => choice.value === picked.value)
          ? { value: picked.value, error: null }
          : refused("Pick one of the choices.");
      }

      const whole = option.type === "integer";

      if (!(whole ? INTEGER : DECIMAL).test(text)) {
        return refused(whole ? "Enter a whole number." : "Enter a number.");
      }

      const value = Number(text);

      if (whole ? !Number.isSafeInteger(value) : !Number.isFinite(value)) {
        return refused("Enter a smaller number.");
      }

      if (
        (option.min !== undefined && value < option.min) ||
        (option.max !== undefined && value > option.max)
      ) {
        return refused(between(option.min, option.max, ""));
      }

      return { value, error: null };
    }

    case "boolean":
      return picked === null
        ? refused("Pick True or False.")
        : { value: picked.value, error: null };
    case "user":
      return picked === null ? refused("Pick a person.") : { value: picked.value, error: null };
    case "room":
      return picked === null ? refused("Pick a channel.") : { value: picked.value, error: null };
  }
}

/** Everything the client finds wrong: a missing subcommand, and each option's problem. */
export function entryErrors(entry: CommandEntry): EntryErrors {
  const options = entryOptions(entry);

  if (options === null) {
    return { options: {}, subcommand: "Pick a subcommand." };
  }

  const errors: Record<string, string> = {};

  for (const option of options) {
    const { error } = readOption(option, entry.inputs[option.name]);

    if (error !== null) {
      errors[option.name] = error;
    }
  }

  return { options: errors, subcommand: null };
}

/** Whether Send may go: a subcommand where one is needed, and every option valid. */
export function entryValid(entry: CommandEntry): boolean {
  const errors = entryErrors(entry);

  return errors.subcommand === null && Object.keys(errors.options).length === 0;
}

/** The error a field shows now: the server's, else the client's once the field was left. */
export function shownError(entry: CommandEntry, name: string, errors: EntryErrors): string | null {
  const server = entry.server.options[name];

  if (server !== undefined) {
    return server;
  }

  return entry.attempted || entry.touched.includes(name) ? (errors.options[name] ?? null) : null;
}

/** The typed values the API takes as `options` (`CommandValues`). */
export function commandValues(entry: CommandEntry): CommandValues {
  const values: Record<string, OptionValue> = {};

  for (const option of entryOptions(entry) ?? []) {
    const { value } = readOption(option, entry.inputs[option.name]);

    if (value !== null) {
      values[option.name] = value;
    }
  }

  return { subcommand: entry.subcommand, values };
}

/** A token for the command line: bare, or JSON-quoted when it holds spaces or quotes. */
function token(value: string): string {
  return value === "" || /[\s"]/.test(value) ? JSON.stringify(value) : value;
}

/** The line itself: `/deploy run service:api replicas:2`, from what each field holds. */
function lineFor(entry: CommandEntry, read: (option: CommandOption) => string): string {
  const words = [`/${entry.command.name}`];
  const subcommand = entry.subcommand ?? entry.subcommandText.trim();

  if (subcommand !== "") {
    words.push(subcommand);
  }

  for (const option of entryOptions(entry) ?? []) {
    const value = read(option);

    if (value !== "") {
      words.push(`${option.name}:${token(value)}`);
    }
  }

  return words.join(" ");
}

/** The invocation's `text`: the command line of the values sent (agents get it as `arguments`). */
export function commandText(entry: CommandEntry): string {
  return lineFor(entry, (option) => {
    const { value } = readOption(option, entry.inputs[option.name]);

    return value === null ? "" : String(value);
  });
}

/** The entry as plain text (Escape): picks as their values, the rest as typed. */
export function entryText(entry: CommandEntry): string {
  return lineFor(entry, (option) => {
    const input = entry.inputs[option.name];

    return input?.picked === null || input?.picked === undefined
      ? (input?.text.trim() ?? "")
      : String(input.picked.value);
  });
}

const ARGUMENT = /([a-z0-9_-]+):("(?:[^"\\]|\\.)*"|\S*)|(\S+)/gi;

/** A quoted token's text; a bare one as is. */
function unquote(value: string): string {
  if (!value.startsWith('"')) {
    return value;
  }

  try {
    return String(JSON.parse(value));
  } catch {
    return value.slice(1, -1);
  }
}

/**
 * An entry from a typed line's arguments (`run service:api replicas:2`), the form `entryText`
 * writes: a leading bare word picks the subcommand, `name:value` fills that option.
 */
export function parseEntry(
  command: SlashCommand,
  schema: CommandSchema,
  args: string,
): CommandEntry {
  let entry = startEntry(command, schema);

  for (const match of args.matchAll(ARGUMENT)) {
    const [, name, raw, bare] = match;

    if (bare !== undefined) {
      if (subcommandsOf(schema) !== null && entry.subcommand === null) {
        entry = typeSubcommand(entry, bare);
      }

      continue;
    }

    const key = (name ?? "").toLowerCase();
    const option = (entryOptions(entry) ?? []).find((candidate) => candidate.name === key);

    if (option === undefined) {
      continue;
    }

    const text = unquote(raw ?? "");

    entry = editOption(showOption(entry, key), key, {
      text,
      picked: typedChoice(option, text),
    });
  }

  return entry;
}

const VALUE_FIELD = /^options\.values\.([a-z0-9_-]+)$/;

/** `is out of range` -> `Is out of range.` */
function sentence(messages: readonly string[]): string {
  return messages
    .map((message) => `${message.charAt(0).toUpperCase()}${message.slice(1)}.`)
    .join(" ");
}

/**
 * A 422 `Validation` envelope's fields mapped onto the form: `options.values.<name>` onto that
 * option, `options.subcommand` onto the subcommand, anything else (an unreadable schema's
 * `options`) to the form as a whole. An option the form doesn't show goes there too, named.
 */
export function serverProblems(
  entry: CommandEntry,
  fields: Readonly<Record<string, readonly string[]>>,
): OptionProblems {
  const shown = new Set(entry.shown);
  const options: Record<string, string> = {};
  const general: string[] = [];
  let subcommand: string | null = null;

  for (const [field, messages] of Object.entries(fields)) {
    const name = VALUE_FIELD.exec(field)?.[1];

    if (name !== undefined && shown.has(name)) {
      options[name] = sentence(messages);
    } else if (field === "options.subcommand") {
      subcommand = sentence(messages);
    } else {
      general.push(name === undefined ? sentence(messages) : `${name}: ${sentence(messages)}`);
    }
  }

  return { options, subcommand, general };
}

const TYPE_LABELS: Readonly<Record<CommandOption["type"], string>> = {
  string: "Text",
  integer: "Whole number",
  number: "Number",
  boolean: "True or false",
  user: "Person",
  room: "Channel",
};

/** What a field takes, for its hint: `Whole number, 1 to 10`, `Text, up to 40 characters`. */
export function optionKind(option: CommandOption): string {
  const label = TYPE_LABELS[option.type];

  if (option.type === "string" && option.choices === undefined) {
    const { min_length: min, max_length: max } = option;

    if (min !== undefined && max !== undefined) return `${label}, ${min} to ${max} characters`;

    if (min !== undefined) return `${label}, at least ${min} characters`;

    if (max !== undefined) return `${label}, up to ${max} characters`;

    return label;
  }

  if ((option.type === "integer" || option.type === "number") && option.choices === undefined) {
    const { min, max } = option;

    if (min !== undefined && max !== undefined) return `${label}, ${min} to ${max}`;

    if (min !== undefined) return `${label}, at least ${min}`;

    if (max !== undefined) return `${label}, at most ${max}`;
  }

  return optionChoices(option) === null || option.type === "boolean" ? label : "Choice";
}

/** The usage of a schema-backed command: `<run|stop>` or `<service> <replicas> [environment]`. */
export function schemaUsage(schema: CommandSchema): string {
  const subcommands = subcommandsOf(schema);

  if (subcommands !== null) {
    return `<${subcommands.map((sub) => sub.name).join("|")}>`;
  }

  return "options" in schema
    ? schema.options
        .map((option) => (option.required ? `<${option.name}>` : `[${option.name}]`))
        .join(" ")
    : "";
}
