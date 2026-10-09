import { describe, expect, it } from "vitest";
import { ActionError } from "../../sync/run.ts";
import { checkHandoff, EMPTY_HANDOFF, handoffLines, serverHandoffErrors } from "./handoff-form.ts";

const ready = { ...EMPTY_HANDOFF, receiverAgentId: 9, summary: "  Over to you  " };

describe("the handoff form", () => {
  it("sends the trimmed summary and the non-blank lines", () => {
    expect(
      checkHandoff({
        ...ready,
        links: "https://a.example\n\n  http://b.example ",
        openQuestions: "Why?\n",
      }),
    ).toEqual({
      body: {
        receiverAgentId: 9,
        summary: "Over to you",
        links: ["https://a.example", "http://b.example"],
        openQuestions: ["Why?"],
      },
    });
    expect(handoffLines(" a \r\n\r\nb")).toEqual(["a", "b"]);
  });

  it("refuses what the server refuses, with its messages", () => {
    expect(checkHandoff(EMPTY_HANDOFF)).toEqual({
      errors: { receiver: "Choose the agent to hand off to", summary: "Summary can't be blank" },
    });
    expect(checkHandoff({ ...ready, summary: "x".repeat(2001) })).toEqual({
      errors: { summary: "Summary is too long (maximum is 2000 characters)" },
    });

    const many = Array.from({ length: 11 }, (_, index) => `https://x.example/${index}`).join("\n");

    expect(checkHandoff({ ...ready, links: many })).toEqual({
      errors: { links: "Links are limited to 10 per handoff" },
    });
    expect(checkHandoff({ ...ready, links: `https://x.example/${"a".repeat(500)}` })).toEqual({
      errors: { links: "Links must be at most 500 characters each" },
    });
    expect(checkHandoff({ ...ready, links: "javascript:alert(1)" })).toEqual({
      errors: { links: "Links must be http(s) URLs" },
    });
    expect(checkHandoff({ ...ready, openQuestions: "q\n".repeat(11) })).toEqual({
      errors: { openQuestions: "Open questions are limited to 10 per handoff" },
    });
    expect(checkHandoff({ ...ready, openQuestions: "q".repeat(501) })).toEqual({
      errors: { openQuestions: "Open questions must be at most 500 characters each" },
    });
  });

  it("puts the server's field errors on those fields, and a fieldless refusal on the alert", () => {
    expect(
      serverHandoffErrors(
        new ActionError("Validation", "Summary is too long (maximum is 2000 characters)", {
          summary: ["is too long (maximum is 2000 characters)"],
          receiverAgentId: ["Receiver must hold the manage_threads capability in this room"],
          links: ["must be http(s) URLs"],
          openQuestions: ["are limited to 10 per handoff"],
        }),
      ),
    ).toEqual({
      errors: {
        summary: "Summary is too long (maximum is 2000 characters).",
        receiver: "Receiver must hold the manage_threads capability in this room.",
        links: "Links must be http(s) URLs.",
        openQuestions: "Open questions are limited to 10 per handoff.",
      },
      alert: null,
    });

    expect(
      serverHandoffErrors(
        new ActionError("Validation", "This thread isn't tracked as work", {
          base: ["This thread isn't tracked as work"],
        }),
      ),
    ).toEqual({ errors: {}, alert: "This thread isn't tracked as work" });

    expect(
      serverHandoffErrors(new ActionError("Forbidden", "You cannot manage work in this thread")),
    ).toEqual({
      errors: {},
      alert: "You cannot manage work in this thread",
    });
  });
});
