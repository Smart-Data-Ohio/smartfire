import { describe, expect, it } from "vitest";
import type { SlashCommand } from "../../gen/SlashCommand.ts";
import { commandUsage, filterCommands, looksLikeCommand, routeSlash, slashName } from "./slash.ts";

function command(name: string, argHint = ""): SlashCommand {
  return {
    name,
    description: `${name} description`,
    argHint,
    takesArguments: argHint !== "",
    agentName: null,
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
