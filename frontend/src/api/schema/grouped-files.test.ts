import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { messageFixture } from "../testing.ts";
import { CreateMessage, MessageDTO } from "./message.ts";

describe("grouped file wire contracts", () => {
  it("preserves the ordered attachment list", () => {
    const attachment = {
      filename: "notes.txt",
      contentType: "text/plain",
      byteSize: 5,
      width: null,
      height: null,
      preview: "file",
      url: "/file",
      downloadUrl: "/file?disposition=attachment",
      thumbnailUrl: null,
    };

    const wire = {
      ...messageFixture(1, 12),
      attachments: [attachment, { ...attachment, filename: "second.txt" }],
    };

    expect(Schema.encodeSync(MessageDTO)(Schema.decodeUnknownSync(MessageDTO)(wire))).toEqual(wire);
  });

  it("preserves grouped signed ids with optional text", () => {
    const wire = {
      clientMessageId: "group",
      markdownSource: "",
      replyToMessageId: null,
      replyNotifyAuthor: null,
      attachmentSignedId: null,
      attachmentSignedIds: ["first", "second"],
    };

    expect(Schema.encodeSync(CreateMessage)(Schema.decodeUnknownSync(CreateMessage)(wire))).toEqual(
      wire,
    );
  });
});
