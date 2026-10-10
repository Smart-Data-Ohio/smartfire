import { describe, expect, it } from "vitest";
import { plainText } from "../features/messages/commands.ts";
import { defaultThreadName } from "../features/threads/new-thread-pane.tsx";
import { messageFixture } from "../features/threads/test-fixtures.ts";
import { htmlPlainText, messagePlainText } from "./spoiler-text.ts";

/** A spoiler span as crates/richtext renders it. */
const spoiler = (words: string) => `<span class="spoiler" data-spoiler="">${words}</span>`;

describe("htmlPlainText", () => {
  it("replaces a spoiler span and keeps the words around it", () => {
    const html = `<p>see ${spoiler("secret words")} now</p><p><code>||secret||</code></p>`;

    expect(htmlPlainText(html)).toBe("see spoiler now||secret||");
  });

  it("keeps a link's label but never its URL or title", () => {
    expect(
      htmlPlainText(
        `<p><a href="https://example.com/alice-dies" title="Alice dies">${spoiler("x")}</a> after</p>`,
      ),
    ).toBe("spoiler after");
  });

  it("replaces a spoiler inside a spoiler once", () => {
    expect(htmlPlainText(`<p>${spoiler(`outer ${spoiler("SECRET")} tail`)}</p>`)).toBe("spoiler");
  });
});

describe("messagePlainText", () => {
  it("uses the Markdown when it can't hold a spoiler", () => {
    expect(messagePlainText("**ship** it", "<p><strong>ship</strong> it</p>")).toBe("**ship** it");
  });

  it("uses the rendered HTML when the Markdown has `||`, so it agrees with the renderer", () => {
    expect(
      messagePlainText(
        "||outer ||SECRET|| tail|| after",
        `<p>${spoiler("outer SECRET tail")} after</p>`,
      ),
    ).toBe("spoiler after");

    expect(messagePlainText("\\`||SECRET||\\`", `<p>\`${spoiler("SECRET")}\`</p>`)).toBe(
      "`spoiler`",
    );

    expect(messagePlainText("use `a || b`", "<p>use <code>a || b</code></p>")).toBe("use a || b");
  });

  it("never falls back to Markdown that holds `||`, even with no HTML", () => {
    expect(messagePlainText("||outer ||SECRET|| tail||", "")).toBe("");
  });
});

describe("plainText", () => {
  it("redacts spoilers through the rendered HTML", () => {
    expect(
      plainText(
        messageFixture(1, {
          markdownSource: "see ||secret words|| now",
          bodyHtml: `<p>see ${spoiler("secret words")} now</p>`,
        }),
      ),
    ).toBe("see spoiler now");

    expect(
      plainText(
        messageFixture(2, {
          markdownSource: "||outer ||SECRET|| tail||",
          bodyHtml: `<p>${spoiler("outer SECRET tail")}</p>`,
        }),
      ),
    ).toBe("spoiler");

    expect(
      plainText(
        messageFixture(3, {
          markdownSource: null,
          bodyHtml: `<p>see ${spoiler("secret words")} now</p>`,
        }),
      ),
    ).toBe("see spoiler now");
  });
});

describe("defaultThreadName", () => {
  it("does not use spoiler contents as the thread name", () => {
    expect(
      defaultThreadName("||the killer|| arrives", `<p>${spoiler("the killer")} arrives</p>`),
    ).toBe("spoiler arrives");

    expect(
      defaultThreadName("\\`||SECRET||\\` arrives", `<p>\`${spoiler("SECRET")}\` arrives</p>`),
    ).toBe("`spoiler` arrives");

    expect(defaultThreadName(null, `<p>${spoiler("the killer")} arrives</p>`)).toBe(
      "spoiler arrives",
    );
  });
});
