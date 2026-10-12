import { describe, expect, it } from "vitest";
import type { CommandSchema } from "../../gen/CommandSchema.ts";
import type { SlashCommand } from "../../gen/SlashCommand.ts";
import {
  type CommandEntry,
  chooseSubcommand,
  commandText,
  commandUsage,
  commandValues,
  dropExtra,
  editOption,
  entryErrors,
  entryText,
  entryValid,
  filterCommands,
  hideOption,
  lineArguments,
  looksLikeCommand,
  optionKind,
  parseEntry,
  routeSlash,
  serverProblems,
  shownError,
  showOption,
  slashName,
  startEntry,
  touchField,
  typeSubcommand,
  withProblems,
} from "./slash.ts";

function command(name: string, argHint = ""): SlashCommand {
  return {
    name,
    description: `${name} description`,
    argHint,
    takesArguments: argHint !== "",
    agentName: null,
    optionSchema: null,
  };
}

const COMMANDS = [
  command("remind", "<when> <text>"),
  command("shrug"),
  command("me", "<text>"),
  command("status", "<text>"),
  command("play", "<sound>"),
];

describe("slashName", () => {
  it("reads the command at the start of the text", () => {
    expect(slashName("/remind in 5m stretch")).toBe("remind");
    expect(slashName("/SHRUG")).toBe("shrug");
    expect(slashName("//shrug")).toBeNull();
    expect(slashName("hello /shrug")).toBeNull();
    expect(slashName("/")).toBeNull();
    expect(slashName("/path/to/file")).toBeNull();
  });

  it("tells when the command list is needed", () => {
    expect(looksLikeCommand("  /me waves")).toBe(true);
    expect(looksLikeCommand("plain text")).toBe(false);
    expect(looksLikeCommand("  /PLAY bell")).toBe(false);
  });
});

describe("routeSlash", () => {
  it("posts /play through normal message submission, even when the command is registered", () => {
    expect(routeSlash("/play bell", COMMANDS)).toEqual({
      kind: "message",
      markdown: "/play bell",
    });
    expect(routeSlash("  /PLAY bell  ", COMMANDS)).toEqual({
      kind: "message",
      markdown: "  /PLAY bell",
    });
  });
  it("runs a known command with the whole trimmed line", () => {
    expect(routeSlash("/remind in 5m stretch \n", COMMANDS)).toEqual({
      kind: "command",
      command: COMMANDS[0],
      text: "/remind in 5m stretch",
    });
  });

  it("posts an unknown /word as an ordinary message", () => {
    expect(routeSlash("/nope hi", COMMANDS)).toEqual({ kind: "message", markdown: "/nope hi" });
  });

  it("posts //text as /text", () => {
    expect(routeSlash("//shrug me", COMMANDS)).toEqual({ kind: "message", markdown: "/shrug me" });
  });

  it("posts plain text untouched but trimmed at the end", () => {
    expect(routeSlash("hello  \n", COMMANDS)).toEqual({ kind: "message", markdown: "hello" });
  });
});

describe("filterCommands", () => {
  it("lists prefix matches before substring matches, in server order", () => {
    expect(filterCommands(COMMANDS, "s").map((entry) => entry.name)).toEqual(["shrug", "status"]);
    expect(filterCommands(COMMANDS, "m").map((entry) => entry.name)).toEqual(["me", "remind"]);
    expect(filterCommands(COMMANDS, "").map((entry) => entry.name)).toEqual([
      "remind",
      "shrug",
      "me",
      "status",
      "play",
    ]);
    expect(filterCommands(COMMANDS, "zz")).toEqual([]);
  });

  it("formats the usage line", () => {
    expect(commandUsage(COMMANDS[0] ?? command("x"))).toBe("/remind <when> <text>");
    expect(commandUsage(command("shrug"))).toBe("/shrug");
  });
});

const DEPLOY_SCHEMA: CommandSchema = {
  subcommands: [
    {
      name: "run",
      description: "Start a deploy",
      options: [
        {
          name: "service",
          description: "The service",
          type: "string",
          required: true,
          min_length: 2,
          max_length: 5,
        },
        {
          name: "environment",
          description: "Where",
          type: "string",
          required: true,
          choices: [
            { name: "Staging", value: "staging" },
            { name: "Production", value: "production" },
          ],
        },
        {
          name: "replicas",
          description: "Copies",
          type: "integer",
          required: false,
          min: 1,
          max: 10,
        },
      ],
    },
    { name: "stop", description: "Stop it", options: [] },
  ],
};

const DEPLOY: SlashCommand = {
  ...command("deploy"),
  takesArguments: true,
  agentName: "Ember",
  optionSchema: DEPLOY_SCHEMA,
};

const TYPES_SCHEMA: CommandSchema = {
  options: [
    { name: "ratio", description: "Ratio", type: "number", required: false, min: 0.5, max: 2 },
    { name: "flag", description: "Flag", type: "boolean", required: false },
    { name: "who", description: "Who", type: "user", required: false },
    { name: "where", description: "Where", type: "room", required: false },
  ],
};

function filled(values: Readonly<Record<string, string>>): CommandEntry {
  let entry = chooseSubcommand(startEntry(DEPLOY, DEPLOY_SCHEMA), "run");

  for (const [name, text] of Object.entries(values)) {
    entry = editOption(showOption(entry, name), name, { text, picked: null });
  }

  return entry;
}

describe("typed command entry", () => {
  it("waits for a subcommand, then shows the required options", () => {
    const entry = startEntry(DEPLOY, DEPLOY_SCHEMA);

    expect(entry.shown).toEqual([]);
    expect(entryErrors(entry).subcommand).toBe("Pick a subcommand.");
    expect(typeSubcommand(entry, "ru").subcommand).toBeNull();

    const run = typeSubcommand(entry, "RUN");

    expect(run.subcommand).toBe("run");
    expect(run.shown).toEqual(["service", "environment"]);
    expect(chooseSubcommand(run, "stop").shown).toEqual([]);
    expect(entryValid(chooseSubcommand(run, "stop"))).toBe(true);
  });

  it("shows every option when none is required, and adds and removes optional ones in order", () => {
    expect(startEntry(DEPLOY, TYPES_SCHEMA).shown).toEqual(["ratio", "flag", "who", "where"]);

    const entry = showOption(filled({}), "replicas");

    expect(entry.shown).toEqual(["service", "environment", "replicas"]);
    expect(hideOption(entry, "replicas").shown).toEqual(["service", "environment"]);
    expect(hideOption(entry, "service").shown).toEqual(entry.shown);
  });

  it("validates as the server does: required, type, range, length and choices", () => {
    expect(entryErrors(filled({})).options).toEqual({
      service: "Required.",
      environment: "Required.",
    });
    expect(
      entryErrors(filled({ service: "a", environment: "moon", replicas: "11" })).options,
    ).toEqual({
      service: "Must be 2 to 5 characters.",
      environment: "Pick one of the choices.",
      replicas: "Must be 1 to 10.",
    });
    expect(
      entryErrors(filled({ service: "api", environment: "x", replicas: "2.5" })).options,
    ).toMatchObject({ replicas: "Enter a whole number." });
    expect(
      entryErrors(filled({ service: "api", environment: "x", replicas: "1e3" })).options,
    ).toMatchObject({ replicas: "Enter a whole number." });
    // Lengths count characters, not UTF-16 units, as Rust's chars() does.
    expect(entryErrors(filled({ service: "😀😀", environment: "Staging" })).options).toEqual({});

    const types = startEntry(DEPLOY, TYPES_SCHEMA);

    const typed = (name: string, text: string) =>
      entryErrors(editOption(types, name, { text, picked: null })).options[name];

    expect(typed("ratio", "0.25")).toBe("Must be 0.5 to 2.");
    expect(typed("ratio", "nope")).toBe("Enter a number.");
    expect(typed("ratio", "1.5")).toBeUndefined();
    expect(typed("flag", "maybe")).toBe("Pick True or False.");
    expect(typed("flag", "yes")).toBeUndefined();
    expect(typed("who", "ada")).toBe("Pick a person.");
    expect(typed("where", "general")).toBe("Pick a channel.");
  });

  it("shows a field's error once it's left, or after a refused send", () => {
    const entry = filled({});
    const errors = entryErrors(entry);

    expect(shownError(entry, "service", errors)).toBeNull();
    expect(shownError(touchField(entry, "service"), "service", errors)).toBe("Required.");
    expect(shownError({ ...entry, attempted: true }, "environment", errors)).toBe("Required.");
  });

  it("serializes the values and the line exactly as the API takes them", () => {
    const entry = filled({ service: " api ", environment: "Staging", replicas: "3" });

    expect(entryValid(entry)).toBe(true);
    expect(commandValues(entry)).toEqual({
      subcommand: "run",
      values: { service: "api", environment: "staging", replicas: 3 },
    });
    expect(commandText(entry)).toBe("/deploy run service:api environment:staging replicas:3");

    const optionsOnly = editOption(
      editOption(startEntry(DEPLOY, TYPES_SCHEMA), "ratio", { text: "1.5", picked: null }),
      "who",
      { text: "Ada", picked: { label: "Ada", value: 12 } },
    );

    expect(commandValues(optionsOnly)).toEqual({
      subcommand: null,
      values: { ratio: 1.5, who: 12 },
    });
    expect(
      commandValues(editOption(optionsOnly, "flag", { text: "False", picked: null })).values,
    ).toMatchObject({ flag: false });
  });

  it("writes plain text that parses back into the same entry", () => {
    const entry = filled({ service: "a b", environment: "prod", replicas: "4" });
    const line = entryText(entry);

    expect(line).toBe('/deploy run service:"a b" environment:prod replicas:4');

    const parsed = parseEntry(DEPLOY, DEPLOY_SCHEMA, line.replace(/^\/deploy /, ""));

    expect(parsed.subcommand).toBe("run");
    expect(parsed.shown).toEqual(["service", "environment", "replicas"]);
    expect(parsed.inputs.service?.text).toBe("a b");
    expect(entryErrors(parsed).options).toEqual({ environment: "Pick one of the choices." });

    const complete = parseEntry(DEPLOY, DEPLOY_SCHEMA, "run service:api environment:Production");

    expect(entryValid(complete)).toBe(true);
    expect(commandValues(complete).values).toEqual({ service: "api", environment: "production" });
  });

  it("maps a 422's fields onto the options, the subcommand, and the form", () => {
    const entry = filled({ service: "api", environment: "staging" });

    const problems = serverProblems(entry, {
      "options.values.service": ["length is out of range"],
      "options.values.replicas": ["is unknown"],
      "options.subcommand": ["is unknown"],
      options: ["command option schema is unavailable"],
    });

    expect(problems).toEqual({
      options: { service: "Length is out of range." },
      subcommand: "Is unknown.",
      general: ["replicas: Is unknown.", "Command option schema is unavailable."],
    });

    const refused = withProblems(entry, problems);

    expect(refused.attempted).toBe(true);
    expect(shownError(refused, "service", entryErrors(refused))).toBe("Length is out of range.");

    // Editing the field drops the server's word on it.
    const edited = editOption(refused, "service", { text: "web", picked: null });

    expect(shownError(edited, "service", entryErrors(edited))).toBeNull();
  });

  it("describes options and a schema's usage", () => {
    expect(commandUsage(DEPLOY)).toBe("/deploy <run|stop>");
    expect(commandUsage({ ...DEPLOY, optionSchema: TYPES_SCHEMA })).toBe(
      "/deploy [ratio] [flag] [who] [where]",
    );

    const [service, environment, replicas] =
      "subcommands" in DEPLOY_SCHEMA ? (DEPLOY_SCHEMA.subcommands[0]?.options ?? []) : [];

    expect(service && optionKind(service)).toBe("Text, 2 to 5 characters");
    expect(environment && optionKind(environment)).toBe("Choice");
    expect(replicas && optionKind(replicas)).toBe("Whole number, 1 to 10");
  });
});

describe("typed lines, as the server reads their values", () => {
  it("keeps arguments that fit no option, and refuses to send them", () => {
    const misspelt = parseEntry(
      DEPLOY,
      DEPLOY_SCHEMA,
      "run service:api environment:staging replica:10 again",
    );

    expect(misspelt.extra).toEqual(["replica:10", "again"]);
    expect(entryErrors(misspelt).general).toEqual(["Not options here: replica:10, again."]);
    expect(entryValid(misspelt)).toBe(false);
    // Back to text they're still there, to fix by hand.
    expect(entryText(misspelt)).toBe(
      "/deploy run service:api environment:staging replica:10 again",
    );

    const removed = dropExtra(misspelt);

    expect(entryErrors(removed).general).toEqual([]);
    expect(entryValid(removed)).toBe(true);
    expect(entryErrors(parseEntry(DEPLOY, DEPLOY_SCHEMA, "run region:eu")).general).toEqual([
      "Not an option here: region:eu.",
    ]);
  });

  it("reads a person and a channel back from their ids, named when they're known", () => {
    const picked = editOption(
      editOption(startEntry(DEPLOY, TYPES_SCHEMA), "who", {
        text: "Maya Okafor",
        picked: { label: "Maya Okafor", value: 12 },
      }),
      "where",
      { text: "#ops", picked: { label: "#ops", value: 3 } },
    );

    const line = entryText(picked);

    expect(line).toBe("/deploy who:12 where:3");

    const names = (type: "user" | "room", id: number) =>
      type === "user" && id === 12 ? "Maya Okafor" : type === "room" && id === 3 ? "ops" : null;

    const parsed = parseEntry(DEPLOY, TYPES_SCHEMA, lineArguments(line), names);

    expect(entryValid(parsed)).toBe(true);
    expect(commandValues(parsed)).toEqual({ subcommand: null, values: { who: 12, where: 3 } });
    expect(parsed.inputs.who).toEqual({
      text: "Maya Okafor",
      picked: { label: "Maya Okafor", value: 12 },
    });
    expect(parsed.inputs.where?.text).toBe("#ops");

    // An id this tab hasn't loaded still goes: the server looks it up.
    const unnamed = parseEntry(DEPLOY, TYPES_SCHEMA, "who:40");

    expect(commandValues(unnamed).values).toEqual({ who: 40 });
    expect(unnamed.inputs.who?.text).toBe("40");
    expect(entryErrors(parseEntry(DEPLOY, TYPES_SCHEMA, "who:0 where:ops")).options).toEqual({
      who: "Pick a person.",
      where: "Pick a channel.",
    });
  });

  it("matches a choice by its exact value before any case-folded match", () => {
    const schema: CommandSchema = {
      options: [
        {
          name: "env",
          description: "Env",
          type: "string",
          required: true,
          choices: [
            { name: "Loud", value: "PROD" },
            { name: "Quiet", value: "prod" },
          ],
        },
      ],
    };

    const quiet = editOption(startEntry(DEPLOY, schema), "env", {
      text: "Quiet",
      picked: { label: "Quiet", value: "prod" },
    });

    const line = entryText(quiet);

    expect(line).toBe("/deploy env:prod");
    expect(commandValues(parseEntry(DEPLOY, schema, lineArguments(line))).values).toEqual({
      env: "prod",
    });
    expect(commandValues(parseEntry(DEPLOY, schema, "env:PROD")).values).toEqual({ env: "PROD" });
    expect(commandValues(parseEntry(DEPLOY, schema, "env:quiet")).values).toEqual({
      env: "prod",
    });
  });

  it("checks a choice's limits as well as its membership, as slash_schema does", () => {
    const schema: CommandSchema = {
      options: [
        {
          name: "code",
          description: "Code",
          type: "string",
          required: false,
          min_length: 2,
          max_length: 3,
          choices: [
            { name: "Short", value: "ab" },
            { name: "Long", value: "toolong" },
          ],
        },
        {
          name: "size",
          description: "Size",
          type: "integer",
          required: false,
          max: 10,
          choices: [
            { name: "Small", value: 5 },
            { name: "Huge", value: 50 },
          ],
        },
      ],
    };

    const entry = startEntry(DEPLOY, schema);

    const errorFor = (name: string, label: string, value: string | number) =>
      entryErrors(editOption(entry, name, { text: label, picked: { label, value } })).options[name];

    expect(errorFor("code", "Short", "ab")).toBeUndefined();
    expect(errorFor("code", "Long", "toolong")).toBe("Must be 2 to 3 characters.");
    expect(errorFor("size", "Small", 5)).toBeUndefined();
    expect(errorFor("size", "Huge", 50)).toBe("Must be at most 10.");
    // Outside both: both, as the server reports both.
    expect(entryErrors(editOption(entry, "code", { text: "x", picked: null })).options.code).toBe(
      "Must be 2 to 3 characters. Pick one of the choices.",
    );
  });

  it("puts a subcommand error on the alert when the form has no subcommand field", () => {
    const entry = startEntry(DEPLOY, TYPES_SCHEMA);

    expect(
      serverProblems(entry, {
        "options.subcommand": ["is required only for commands with subcommands"],
      }),
    ).toEqual({
      options: {},
      subcommand: null,
      general: ["subcommand: Is required only for commands with subcommands."],
    });
  });
});

describe("choice values through the text form", () => {
  const VERBATIM: CommandSchema = {
    options: [
      {
        name: "env",
        description: "Env",
        type: "string",
        required: false,
        choices: [
          { name: "Padded", value: " prod " },
          { name: "Plain", value: "prod" },
          { name: "Empty", value: "" },
          { name: "Spaced", value: "eu west: 2" },
        ],
      },
    ],
  };

  const pick = (label: string, value: string) =>
    editOption(startEntry(DEPLOY, VERBATIM), "env", { text: label, picked: { label, value } });

  it.each([
    ["Padded", " prod ", '/deploy env:" prod "'],
    ["Plain", "prod", "/deploy env:prod"],
    ["Empty", "", '/deploy env:""'],
    ["Spaced", "eu west: 2", '/deploy env:"eu west: 2"'],
  ])("keeps %s's value verbatim, there and back", (label, value, line) => {
    const picked = pick(label, value);

    expect(entryText(picked)).toBe(line);
    expect(commandText(picked)).toBe(line);
    expect(commandValues(picked).values).toEqual({ env: value });

    const parsed = parseEntry(DEPLOY, VERBATIM, lineArguments(line));

    expect(parsed.inputs.env).toEqual({ text: label, picked: { label, value } });
    expect(commandValues(parsed).values).toEqual({ env: value });
    expect(entryText(parsed)).toBe(line);
  });

  it("tells an empty value apart from no value", () => {
    const blank = editOption(startEntry(DEPLOY, VERBATIM), "env", { text: "", picked: null });

    expect(entryText(blank)).toBe("/deploy");
    expect(commandValues(blank).values).toEqual({});
    expect(commandValues(parseEntry(DEPLOY, VERBATIM, "env:")).values).toEqual({});
    expect(commandValues(parseEntry(DEPLOY, VERBATIM, 'env:""')).values).toEqual({ env: "" });
  });
});
