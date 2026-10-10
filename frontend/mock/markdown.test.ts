import { describe, expect, it } from "vitest";
import { mentionsUser, renderMarkdown } from "./markdown.ts";

describe("stable user mentions", () => {
  it("distinguishes duplicate names and preserves legacy name resolution", () => {
    const people = [
      { id: 1, name: "Twin" },
      { id: 2, name: "Twin" },
    ];

    const html = renderMarkdown("<@2> @[Twin]", people);

    expect(mentionsUser(html, 2)).toBe(true);
    expect(mentionsUser(html, 1)).toBe(false);
    expect(html).toContain("@[Twin]");
    expect(mentionsUser(renderMarkdown("@[Unique]", [{ id: 3, name: "Unique" }]), 3)).toBe(true);
  });

  it("renders the current escaped name after a rename, using the same source", () => {
    const source = "Hello <@1>";

    expect(renderMarkdown(source, [{ id: 1, name: "Before" }])).toContain(
      'class="profile-card-name">Before</button>',
    );

    const renamed = renderMarkdown(source, [{ id: 1, name: "After <script>" }]);

    expect(renamed).toContain('class="profile-card-name">After &lt;script&gt;</button>');
    expect(mentionsUser(renamed, 1)).toBe(true);
    expect(renamed).not.toContain("Before");
  });

  it("leaves unknown, deleted and inaccessible IDs literal without notification identities", () => {
    const source = "<@1> <@2> <@0> <@-1> <@01> <@9223372036854775808>";
    const html = renderMarkdown(source, []);

    expect(html).toBe(
      "<p>&lt;@1&gt; &lt;@2&gt; &lt;@0&gt; &lt;@-1&gt; &lt;@01&gt; &lt;@9223372036854775808&gt;</p>",
    );
    expect(mentionsUser(html, 1)).toBe(false);
    expect(mentionsUser(html, 2)).toBe(false);
  });

  it("keeps code, link labels and escaped mentions literal", () => {
    const html = renderMarkdown(
      "`<@1>` [<@1>](https://example.test) \\<@1> \\@[Current]\n\n```\n<@1>\n```",
      [{ id: 1, name: "Current" }],
    );

    expect(html).toContain("<code>&lt;@1&gt;</code>");
    expect(html).toContain(">&lt;@1&gt;</a>");
    expect(html).toContain("&lt;@1&gt; @[Current]");
    expect(mentionsUser(html, 1)).toBe(false);
  });
});
