import { describe, expect, it } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { MessageDTO } from "../../store/model.ts";
import { displayedAuthor, forwardedAuthor } from "./webhook-attribution.ts";

const ci = { name: "Deployments", username: "CI", avatarUrl: null };

const FORWARDED = "2026-10-06T00:01:00.000Z";

function message(id: number, change: Partial<MessageDTO>): MessageDTO {
  return { ...messageFixture(id, 1), ...change };
}

// CI posts 40, Priya (7) forwards it as 41, Bob (8) forwards hers as 42: each copy keeps CI's snapshot.
const posted = message(40, { creatorId: 9000, webhook: ci });

const priyas = message(41, {
  creatorId: 7,
  webhook: ci,
  forwardedAt: FORWARDED,
  forwardedFromMessageId: 40,
});

const bobs = message(42, {
  creatorId: 8,
  webhook: ci,
  forwardedAt: FORWARDED,
  forwardedFromMessageId: 41,
});

describe("displayedAuthor", () => {
  it("credits the webhook only for the message it posted", () => {
    expect(displayedAuthor(posted)).toEqual({ kind: "webhook", webhook: ci, name: "CI" });
  });

  it("credits the forwarder for a copy at any depth", () => {
    expect(displayedAuthor(priyas)).toEqual({ kind: "person", userId: 7 });
    expect(displayedAuthor(bobs)).toEqual({ kind: "person", userId: 8 });
  });

  it("credits a person's own message to them", () => {
    expect(displayedAuthor(message(43, { creatorId: 7 }))).toEqual({ kind: "person", userId: 7 });
  });
});

describe("forwardedAuthor", () => {
  it("credits a loaded source's own author", () => {
    expect(forwardedAuthor(priyas, posted)).toEqual({ kind: "webhook", webhook: ci, name: "CI" });
    expect(forwardedAuthor(bobs, priyas)).toEqual({ kind: "person", userId: 7 });
  });

  it("falls back to the webhook the copy carries, else nobody", () => {
    expect(forwardedAuthor(bobs, undefined)).toEqual({ kind: "webhook", webhook: ci, name: "CI" });
    expect(forwardedAuthor(message(44, { forwardedAt: FORWARDED }), undefined)).toBeNull();
  });
});
