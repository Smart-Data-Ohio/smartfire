/**
 * Slash-command routing, as the classic composer does it (crates/api_types composer.rs):
 * `//text` posts `/text` as an ordinary message, `/play` uses normal message submission, a known
 * `/command` runs on the server, and an unknown `/word` is posted as typed.
 */
import type { SlashCommand } from "../../gen/SlashCommand.ts";

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

/** The usage line: `/remind <when> <text>`. */
export function commandUsage(command: SlashCommand): string {
  return command.argHint === "" ? `/${command.name}` : `/${command.name} ${command.argHint}`;
}
