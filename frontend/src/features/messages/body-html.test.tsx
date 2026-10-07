import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
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
});
