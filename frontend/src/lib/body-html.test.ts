import { describe, expect, it } from "vitest";
import { inlineMentions } from "./body-html.ts";

const MENTION =
  '<action-text-attachment content-type="application/vnd.campfire.mention" sgid="s1">' +
  '<div class="mention mention--user-4" sgid="s1" data-user-id="4">\n' +
  '  <a title="Maya Okafor" class="btn avatar" href="/users/4">M</a>\n' +
  '  <button name="button" type="button" class="profile-card-name">Maya Okafor</button>\n' +
  "</div>\n</action-text-attachment>";

describe("inlineMentions", () => {
  it("turns the mention wrapper into a span, attributes and contents kept", () => {
    const html = inlineMentions(`<p>Thanks ${MENTION} for this</p>`);

    expect(html).toContain('<span class="mention mention--user-4" sgid="s1" data-user-id="4">');
    expect(html).toContain('class="profile-card-name">Maya Okafor</button>\n</span>');
    expect(html).not.toContain("<div");
  });

  it("keeps the sentence in one paragraph once parsed", () => {
    const host = document.createElement("div");

    host.innerHTML = inlineMentions(`<p>Thanks ${MENTION} for this</p>`);

    expect(host.querySelectorAll("p")).toHaveLength(1);
    expect(host.querySelector("p")?.textContent).toContain("for this");
  });

  it("handles several mentions and leaves other HTML alone", () => {
    const html = `<p>${MENTION} and ${MENTION.replaceAll("4", "5")}</p><div class="other">x</div>`;
    const result = inlineMentions(html);

    expect(result.match(/<span class="mention/g)).toHaveLength(2);
    expect(result).toContain('<div class="other">x</div>');
    expect(inlineMentions("<p>plain</p>")).toBe("<p>plain</p>");
  });
});
