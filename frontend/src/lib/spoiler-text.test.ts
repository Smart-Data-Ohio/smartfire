import { describe, expect, it } from "vitest";
import { plainText } from "../features/messages/commands.ts";
import { defaultThreadName } from "../features/threads/new-thread-pane.tsx";
import { messageFixture } from "../features/threads/test-fixtures.ts";
import { htmlPlainText, redactMarkdownSpoilers } from "./spoiler-text.ts";

describe("redactMarkdownSpoilers", () => {
  it("replaces each closed spoiler and leaves code, fences, and unclosed text", () => {
    expect(redactMarkdownSpoilers("see ||secret words|| now")).toBe("see spoiler now");
    expect(redactMarkdownSpoilers("||one|| and ||two||")).toBe("spoiler and spoiler");
    expect(redactMarkdownSpoilers("||**secret**||")).toBe("spoiler");
    expect(redactMarkdownSpoilers("||@[David] is the killer||")).toBe("spoiler");
    expect(redactMarkdownSpoilers("use `||secret||` here")).toBe("use `||secret||` here");
    expect(redactMarkdownSpoilers("```\n||secret||\n```")).toBe("```\n||secret||\n```");
    expect(redactMarkdownSpoilers("||nope")).toBe("||nope");
    expect(redactMarkdownSpoilers("||top\n\nbottom||")).toBe("||top\n\nbottom||");
  });
});

describe("htmlPlainText", () => {
  it("replaces a spoiler span and keeps the words around it", () => {
    const html =
      '<p>see <span class="spoiler" data-spoiler="">secret words</span> now</p>' +
      "<p><code>||secret||</code></p>";

    expect(htmlPlainText(html)).toBe("see spoiler now||secret||");
  });
});

describe("plainText", () => {
  it("redacts spoilers from Markdown and from rendered HTML", () => {
    expect(plainText(messageFixture(1, { markdownSource: "see ||secret words|| now" }))).toBe(
      "see spoiler now",
    );

    expect(
      plainText(
        messageFixture(2, {
          markdownSource: null,
          bodyHtml: '<p>see <span data-spoiler="">secret words</span> now</p>',
        }),
      ),
    ).toBe("see spoiler now");
  });
});

describe("defaultThreadName", () => {
  it("does not use spoiler contents as the thread name", () => {
    expect(defaultThreadName("||the killer|| arrives", "<p>ignored</p>")).toBe("spoiler arrives");
    expect(defaultThreadName(null, '<p><span class="spoiler">the killer</span> arrives</p>')).toBe(
      "spoiler arrives",
    );
  });
});
