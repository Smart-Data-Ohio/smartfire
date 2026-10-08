import { describe, expect, it } from "vitest";
import type { MessageDTO } from "../../gen/MessageDTO.ts";
import { messageAnchor } from "./message-destination.ts";

const message: Pick<MessageDTO, "id" | "roomId" | "threadId"> = {
  id: 4,
  roomId: 8,
  threadId: null,
};

describe("messageAnchor", () => {
  it("stays on a root message of the URL's room", () => {
    expect(messageAnchor(8, message)).toEqual({ kind: "here" });
  });

  it("opens another room's message there", () => {
    expect(messageAnchor(1, message)).toEqual({
      kind: "elsewhere",
      href: "/app/r/8/m/4",
    });
  });

  it("opens a reply on its thread", () => {
    expect(messageAnchor(8, { ...message, threadId: 9 })).toEqual({
      kind: "elsewhere",
      href: "/app/r/8/t/9?m=4",
    });
  });
});
