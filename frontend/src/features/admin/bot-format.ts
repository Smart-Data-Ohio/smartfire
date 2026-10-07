/** The bot screens' words, as the classic chat bot pages put them, and the rules behind them. */
import type { Bot } from "../../gen/Bot.ts";
import type { CredentialState } from "../../gen/CredentialState.ts";
import type { UpdateBotAgent } from "../../gen/UpdateBotAgent.ts";

/** What the classic pages ask before each irreversible change. */
export const CONFIRM = {
  remove:
    "Are you sure you want to permanently remove this bot from the account? This can’t be undone.",
  key: "Are you sure you want to change the bot key? All usage of this bot must be updated.",
  secret: "Reset the signing secret? The receiving service must be updated with the new secret.",
  suspend: "Suspend this agent and cancel its pending approvals? This cannot be undone from here.",
  disconnect: "Disconnect GitHub? This agent will no longer be able to request PR write actions.",
} as const;

/** The edit form's fields, as typed. */
export interface BotForm {
  readonly name: string;
  readonly iconName: string;
  readonly webhookUrl: string;
  readonly provider: string;
  readonly runtime: string;
  readonly description: string;
  readonly dailyMessageCap: string;
  readonly dailyBoardPostCap: string;
  readonly dailyExternalActionCap: string;
}

/** A budget as its input shows it: blank is unlimited. */
function capText(cap: number | null): string {
  return cap === null ? "" : `${cap}`;
}

/** The edit form filled from `bot`. */
export function botForm(bot: Bot): BotForm {
  const agent = bot.agent;

  return {
    name: bot.name,
    iconName: bot.iconName ?? "",
    webhookUrl: bot.webhookUrl ?? "",
    provider: agent?.provider ?? "",
    runtime: agent?.runtime ?? "",
    description: agent?.description ?? "",
    dailyMessageCap: capText(agent?.dailyMessageCap ?? null),
    dailyBoardPostCap: capText(agent?.dailyBoardPostCap ?? null),
    dailyExternalActionCap: capText(agent?.dailyExternalActionCap ?? null),
  };
}

/**
 * The edit as the classic form posts it: every field, as typed. The webhook URL goes only from an
 * administrator (anyone else's form doesn't show it), and the agent's fields only for a bot that
 * has an agent.
 */
export function botChange(form: BotForm, bot: Bot) {
  const agent: UpdateBotAgent | null =
    bot.agent === null
      ? null
      : {
          provider: form.provider,
          runtime: form.runtime,
          description: form.description,
          dailyMessageCap: form.dailyMessageCap.trim(),
          dailyBoardPostCap: form.dailyBoardPostCap.trim(),
          dailyExternalActionCap: form.dailyExternalActionCap.trim(),
        };

  return {
    name: form.name,
    iconName: form.iconName,
    webhookUrl: bot.canAdminister ? form.webhookUrl : null,
    agent,
  };
}

/** A form value as an optional field: blank is unset. */
export function optional(value: string): string | null {
  const trimmed = value.trim();

  return trimmed === "" ? null : trimmed;
}

/** The words for where a credential stands. */
export const CREDENTIAL_STATE: Readonly<Record<CredentialState, string>> = {
  active: "Active",
  expired: "Expired",
  revoked: "Revoked",
};

/** How to use a new credential's secret, as the classic page shows it. */
export const CREDENTIAL_USAGE = "Authorization: Bearer <secret>";

/** The notice after the kill switch, when the server sends none. */
export const SUSPENDED = "Agent suspended";
