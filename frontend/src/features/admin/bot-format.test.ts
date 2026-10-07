import { describe, expect, it } from "vitest";
import type { Bot } from "../../gen/Bot.ts";
import { botChange, botForm, optional } from "./bot-format.ts";

const bender: Bot = {
  id: 9,
  name: "Bender",
  avatarUrl: "/users/9/avatar",
  avatarAttached: false,
  iconName: null,
  icon: null,
  webhookUrl: "https://example.com/bender",
  canAdminister: true,
  agent: {
    id: 11,
    provider: "anthropic",
    runtime: null,
    description: null,
    dailyMessageCap: 50,
    dailyBoardPostCap: null,
    dailyExternalActionCap: 5,
    usage: "0/50 messages",
    suspended: false,
    ledgerUrl: "/agents/11/events",
    approvalsUrl: "/agents/11/approvals",
  },
  signingSecret: null,
  github: null,
};

describe("the bot form", () => {
  it("fills from the bot, a budget's blank meaning unlimited", () => {
    const form = botForm(bender);

    expect(form.iconName).toBe("");
    expect(form.dailyMessageCap).toBe("50");
    expect(form.dailyBoardPostCap).toBe("");
  });

  it("posts every field as typed, as the classic form does", () => {
    const change = botChange({ ...botForm(bender), dailyMessageCap: " lots " }, bender);

    expect(change.webhookUrl).toBe("https://example.com/bender");
    expect(change.agent?.dailyMessageCap).toBe("lots");
    expect(change.agent?.dailyBoardPostCap).toBe("");
  });

  it("leaves the webhook URL to administrators and the agent to bots that have one", () => {
    const owned = { ...bender, canAdminister: false, agent: null };
    const change = botChange(botForm(owned), owned);

    expect(change.webhookUrl).toBeNull();
    expect(change.agent).toBeNull();
  });

  it("reads a blank optional field as unset", () => {
    expect(optional("  ")).toBeNull();
    expect(optional(" robot ")).toBe("robot");
  });
});
