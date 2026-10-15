import { describe, expect, it } from "vitest";
import { toggleWrap } from "./markdown-keys.ts";
import {
  editMentions,
  formatMentions,
  hydrateMentions,
  type MentionText,
  mentionMarkdown,
} from "./mentions.ts";

const nameOf = (id: number) =>
  new Map([
    [14, "Jon Dixon"],
    [123, "Twin"],
    [456, "Twin"],
  ]).get(id);

describe("composer mentions", () => {
  it("keeps twins distinct and leaves unpicked @names literal", () => {
    const draft = hydrateMentions("<@123> and <@456>, typed @Twin", nameOf);

    expect(draft.text).toBe("@Twin and @Twin, typed @Twin");
    expect(mentionMarkdown(draft)).toBe("<@123> and <@456>, typed @Twin");
  });

  it.each(["Hello @Jon Dxon", "Hello @Jon", "Hello ", "Hello @Joan Dixon"])(
    "degrades an edited mention to plain text: %s",
    (text) => {
      const draft = editMentions(hydrateMentions("Hello <@14>", nameOf), text);

      expect(draft.mentions).toEqual([]);
      expect(mentionMarkdown(draft)).toBe(text);
    },
  );

  it("shifts a mention when text is inserted before it and preserves text added after it", () => {
    const draft = editMentions(hydrateMentions("<@14>", nameOf), "Hello @Jon Dixon");
    const edited = editMentions(draft, "Hello @Jon Dixon, thanks!");

    expect(edited.mentions).toEqual([{ start: 6, text: "@Jon Dixon", token: "<@14>" }]);
    expect(mentionMarkdown(edited)).toBe("Hello <@14>, thanks!");
  });

  it("round-trips an edited message with multiple mentions", () => {
    const draft = hydrateMentions("Hello <@14>, ask <@456>", nameOf);

    expect(mentionMarkdown(draft)).toBe("Hello <@14>, ask <@456>");
    expect(mentionMarkdown(editMentions(draft, `${draft.text} tomorrow`))).toBe(
      "Hello <@14>, ask <@456> tomorrow",
    );
  });

  it("keeps the second twin's identity when the first mention is deleted", () => {
    const draft = hydrateMentions("<@123> <@456>", nameOf);

    expect(mentionMarkdown(editMentions(draft, "@Twin", { start: 0, end: 6 }))).toBe("<@456>");
  });

  it.each(["@Twinx", "x@Twin", "@Twin_", "é@Twin", "@Twin字"])(
    "degrades word-adjacent typing: %s",
    (text) => {
      expect(mentionMarkdown(editMentions(hydrateMentions("<@123>", nameOf), text))).toBe(text);
    },
  );

  it.each(["@Twin's", "@Twin,", "(@Twin"])("preserves punctuation: %s", (text) => {
    expect(mentionMarkdown(editMentions(hydrateMentions("<@123>", nameOf), text))).toBe(
      text.replace("@Twin", "<@123>"),
    );
  });

  it("preserves parentheses typed around a mention", () => {
    const draft = editMentions(hydrateMentions("<@123>", nameOf), "(@Twin");
    expect(mentionMarkdown(editMentions(draft, "(@Twin)"))).toBe("(<@123>)");
  });

  it("keeps names containing Markdown special characters intact", () => {
    const draft = hydrateMentions("Hi <@14>!", () => "A*_B[team]");
    expect(draft.text).toBe("Hi @A*_B[team]!");
    expect(mentionMarkdown(editMentions(draft, `${draft.text} Thanks`))).toBe("Hi <@14>! Thanks");
  });

  it.each(["**", "*", "_", "__", "~~"])(
    "keeps identities through %s wrap, editing and unwrap",
    (marker) => {
      const current = hydrateMentions("Hi <@14>, <@456>!", nameOf);
      const edit = toggleWrap({ value: current.text, start: 3, end: 13 }, marker);
      const wrapped = formatMentions(current, edit);
      expect(mentionMarkdown(wrapped)).toBe(`Hi ${marker}<@14>${marker}, <@456>!`);
      const appended = editMentions(wrapped, `${wrapped.text} Done`);
      expect(mentionMarkdown(appended)).toBe(`Hi ${marker}<@14>${marker}, <@456>! Done`);

      const unwrapped = formatMentions(
        appended,
        toggleWrap({ ...edit, value: appended.text }, marker),
      );

      expect(mentionMarkdown(unwrapped)).toBe("Hi <@14>, <@456>! Done");
    },
  );

  it("keeps an unresolvable id as its original token", () => {
    const draft = hydrateMentions("Hello <@999> and <@14>", nameOf);

    expect(draft.text).toBe("Hello <@999> and @Jon Dixon");
    expect(mentionMarkdown(draft)).toBe("Hello <@999> and <@14>");
  });

  it("leaves escaped tokens and code spans and blocks unchanged", () => {
    const source = "\\<@14> `<@14>` ``<@14>``\n```\n<@14>\n```\n~~~\n<@14>\n~~~\n<@14>";
    const draft = hydrateMentions(source, nameOf);

    expect(draft.text).toBe(`${source.slice(0, -5)}@Jon Dixon`);
    expect(draft.mentions).toHaveLength(1);
    expect(mentionMarkdown(draft)).toBe(source);
  });

  it("does not serialize stale ranges that no longer match their displayed text", () => {
    const draft: MentionText = {
      text: "@Jon",
      mentions: [{ start: 0, text: "@Jon Dixon", token: "<@14>" }],
    };

    expect(mentionMarkdown(draft)).toBe("@Jon");
  });
});
