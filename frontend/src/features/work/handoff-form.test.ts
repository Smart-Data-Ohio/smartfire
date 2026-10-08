import { describe, expect, it } from "vitest";
import { ActionError } from "../../sync/run.ts";
import { EMPTY_DRAFT, handoffBody, lines, localProblems, serverProblems } from "./handoff-form.ts";
import { parseWorkSearch } from "./work-search.ts";

describe("the handoff form's rules", () => {
  it("takes one entry per line, trimmed, without blanks or repeats", () => {
    expect(lines(" a \r\n\nb\na\n  \n")).toEqual(["a", "b"]);
  });

  it("checks what the server would refuse, in its words", () => {
    const draft = {
      receiverAgentId: "4",
      summary: "x".repeat(2001),
      links: Array.from({ length: 11 }, (_, index) => `https://example.com/${index}`).join("\n"),
      openQuestions: `${"q".repeat(501)}`,
    };

    expect(localProblems(draft)).toEqual({
      summary: "Summary is too long (maximum is 2000 characters).",
      links: "Links are limited to 10 per handoff.",
      openQuestions: "Open questions must be at most 500 characters each.",
    });
    expect(localProblems({ ...EMPTY_DRAFT, summary: "   " })).toEqual({
      receiverAgentId: "Choose the agent to hand this work to.",
      summary: "Summary can't be blank.",
    });
    expect(
      localProblems({ ...draft, summary: "Done", links: "example.com", openQuestions: "" }),
    ).toEqual({ links: "Links must be http(s) URLs." });
  });

  it("reads the server's refusals as sentences under their fields", () => {
    const error = new ActionError(
      "Validation",
      "Summary can't be blank, Links must be http(s) URLs",
      {
        summary: ["can't be blank"],
        links: ["must be http(s) URLs"],
        receiverAgentId: ["Receiver is already the owner of this work"],
        packageEntries: ["are odd"],
      },
    );

    expect(serverProblems(error)).toEqual({
      summary: "Summary can't be blank.",
      links: "Links must be http(s) URLs.",
      receiverAgentId: "Receiver is already the owner of this work.",
      base: "are odd",
    });
    expect(
      serverProblems(new ActionError("Forbidden", "You cannot manage work in this thread")),
    ).toEqual({ base: "You cannot manage work in this thread" });
  });

  it("sends the agent's id as a number and the lists as lines", () => {
    expect(
      handoffBody({
        receiverAgentId: "41",
        summary: "Go",
        links: "https://a.test\n",
        openQuestions: "",
      }),
    ).toEqual({ receiverAgentId: 41, summary: "Go", links: ["https://a.test"], openQuestions: [] });
  });
});

describe("the Work page's search", () => {
  it("keeps a known filter and drops open, the default, and anything unknown", () => {
    expect(parseWorkSearch({ state: "agents" })).toEqual({ state: "agents" });
    expect(parseWorkSearch({ state: "open" })).toEqual({});
    expect(parseWorkSearch({ state: "closed" })).toEqual({});
    expect(parseWorkSearch({})).toEqual({});
  });
});
