import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { inlineMentions } from "../../lib/body-html.ts";
import { BodyHtml } from "./body-html.tsx";

/** A body with a mention as the server renders it: a block `<div>` with an avatar link and name button. */
const HTML =
  '<p>Ship it, <action-text-attachment content-type="application/vnd.campfire.mention" sgid="s1">' +
  '<div class="mention mention--user-7" sgid="s1" data-user-id="7">\n' +
  '  <a title="Maya Okafor" class="btn avatar" href="/users/7">M</a>\n' +
  '  <button name="button" type="button" class="profile-card-name">Maya Okafor</button>\n' +
  "</div>\n</action-text-attachment> &amp; friends</p>";

/** The markup a body rendered before it went through `BodyHtml`, built the way it was. */
function inlined(className: string, stale?: true): string {
  const { container } = render(
    <div
      className={className}
      data-stale={stale}
      // biome-ignore lint/security/noDangerouslySetInnerHtml: the same server HTML, for comparison
      dangerouslySetInnerHTML={{ __html: inlineMentions(HTML) }}
    />,
  );

  return container.innerHTML;
}

describe("BodyHtml", () => {
  it("renders a pinned message's body as the pins pane did", () => {
    const expected = inlined("pin-card-body message-body");

    const { container } = render(<BodyHtml html={HTML} className="pin-card-body message-body" />);

    expect(container.innerHTML).toBe(expected);

    expect(container.querySelector("p .mention .profile-card-name")?.textContent).toBe(
      "Maya Okafor",
    );

    expect(container.querySelector("div.mention, a, button")).toBeNull();
  });

  it("renders the composer preview, stale or not, as the preview panel did", () => {
    for (const stale of [true, undefined] as const) {
      const expected = inlined("message-body composer-preview-body", stale);

      const { container } = render(
        <BodyHtml html={HTML} className="message-body composer-preview-body" data-stale={stale} />,
      );

      expect(container.innerHTML).toBe(expected);
    }
  });

  it("colours a fenced block after mount and keeps it through an unrelated re-render", async () => {
    const html = '<pre><code class="language-ts">const value: string = "hello";</code></pre>';
    const { container, rerender } = render(<BodyHtml html={html} className="message-body" />);

    await vi.waitFor(() => expect(container.querySelector(".code-token--keyword")).not.toBeNull());

    const coloured = container.querySelector("code");

    rerender(<BodyHtml html={html} className="message-body" data-stale />);

    expect(container.querySelector("code")).toBe(coloured);
    expect(container.querySelector(".code-token--string")?.textContent).toBe('"hello"');
    expect(container.querySelectorAll("button.code-copy")).toHaveLength(1);
  });

  it("redoes the blocks when the body changes (an edit)", async () => {
    const before = '<pre><code class="language-rust">fn a() {}</code></pre>';
    const after = '<pre><code class="language-rust">fn b() {}</code></pre>';
    const { container, rerender } = render(<BodyHtml html={before} className="message-body" />);

    await vi.waitFor(() => expect(container.querySelector(".code-token")).not.toBeNull());
    rerender(<BodyHtml html={after} className="message-body" />);
    await vi.waitFor(() => expect(container.querySelector(".code-token")).not.toBeNull());

    expect(container.querySelector("code")?.textContent).toBe("fn b() {}");
    expect(container.querySelectorAll("button.code-copy")).toHaveLength(1);
  });
});
