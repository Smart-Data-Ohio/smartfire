import { describe, expect, it } from "vitest";
import { plainText } from "../features/messages/commands.ts";
import { defaultThreadName } from "../features/threads/new-thread-pane.tsx";
import { messageFixture } from "../features/threads/test-fixtures.ts";
import { htmlPlainText, messagePlainText, redactMarkdownSpoilers } from "./spoiler-text.ts";

/** A spoiler span as crates/richtext renders it. */
const spoiler = (words: string) => `<span class="spoiler" data-spoiler="">${words}</span>`;

/** Links, images and references around spoilers. crates/richtext/tests/spoilers.rs has the same table. */
const LINK_CASES: readonly (readonly [string, string])[] = [
  ['[||Alice dies||](https://example.com/alice-dies "Alice dies")', "spoiler"],
  ['see [the ||end||](https://example.com/a "t") now', "see spoiler now"],
  ['[||x||](https://example.com/a "a) b") after', "spoiler after"],
  ["[||x||](<https://example.com/alice dies>) after", "spoiler after"],
  [
    'read [||Alice dies||][ending] now\n\n[ending]: https://example.com/alice-dies "Alice dies"',
    "read spoiler now\n\nspoiler",
  ],
  [
    "[||Alice dies||] now\n\n[||Alice dies||]: https://example.com/alice-dies",
    "spoiler now\n\nspoiler",
  ],
  ['[||x||][r]\n\n[r]: https://example.com/a\n  "Alice dies"', "spoiler\n\nspoiler"],
  ['![||Alice dies||](https://example.com/alice.png "Alice dies") end', "spoiler end"],
  ["[![||x||](https://example.com/i.png)](https://example.com/alice-dies)", "spoiler"],
  ["||Alice dies|| <https://example.com/shown>", "spoiler <https://example.com/shown>"],
  ["<https://example.com/||alice||>", "<https://example.com/spoiler>"],
  ["||<https://example.com/alice-dies>||", "spoiler"],
  [
    "[a](https://example.com/b) ||x|| [c](https://example.com/d)",
    "[a](https://example.com/b) spoiler [c](https://example.com/d)",
  ],
  [
    "[a][r] and no spoiler\n\n[r]: https://example.com/shown",
    "[a][r] and no spoiler\n\n[r]: https://example.com/shown",
  ],
];

describe("redactMarkdownSpoilers", () => {
  it("replaces a closed spoiler and leaves unpaired markers", () => {
    expect(redactMarkdownSpoilers("see ||secret words|| now")).toBe("see spoiler now");
    expect(redactMarkdownSpoilers("||**secret**||")).toBe("spoiler");
    expect(redactMarkdownSpoilers("||@[David] is the killer||")).toBe("spoiler");
    expect(redactMarkdownSpoilers("||nope")).toBe("||nope");
    expect(redactMarkdownSpoilers("a ||| b")).toBe("a ||| b");
  });

  it("keeps the words of a block with no spoiler, as the renderer does across a blank line", () => {
    expect(redactMarkdownSpoilers("||top\n\nbottom||")).toBe("||top\n\nbottom||");
    expect(redactMarkdownSpoilers("plain\n\n||secret|| end")).toBe("plain\n\nspoiler end");
  });

  it("hides nested spoilers whole: the inner markers never reveal the words between them", () => {
    expect(redactMarkdownSpoilers("||outer ||SECRET|| tail||")).toBe("spoiler");
    expect(redactMarkdownSpoilers("say ||a **||SECRET||** c|| ok")).toBe("say spoiler ok");
  });

  it("does not let escapes or code spans move the pairing", () => {
    // The renderer makes `x` a spoiler between two escaped backticks.
    expect(redactMarkdownSpoilers("\\`||SECRET||\\`")).toBe("\\`spoiler\\`");
    // A code span's pipes would pair with the spoiler's opener in a stricter parser.
    expect(redactMarkdownSpoilers("`||` ||SECRET|| tail")).toBe("`spoiler tail");
    expect(redactMarkdownSpoilers("\\||a|| ||SECRET||")).toBe("\\spoiler");
    expect(redactMarkdownSpoilers("```\n||SECRET||\n```")).toBe("```\nspoiler\n```");
  });

  it("hides the whole link, URL and title, when its label holds a spoiler", () => {
    for (const [source, redacted] of LINK_CASES) {
      expect(redactMarkdownSpoilers(source), source).toBe(redacted);
    }
  });

  it("hides two spoilers and what sits between them", () => {
    expect(redactMarkdownSpoilers("||one|| and ||two||")).toBe("spoiler");
  });
});

describe("htmlPlainText", () => {
  it("replaces a spoiler span and keeps the words around it", () => {
    const html = `<p>see ${spoiler("secret words")} now</p><p><code>||secret||</code></p>`;

    expect(htmlPlainText(html)).toBe("see spoiler now||secret||");
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

  it("redacts the Markdown broadly when there is no HTML", () => {
    expect(messagePlainText("||outer ||SECRET|| tail||", "")).toBe("spoiler");
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
