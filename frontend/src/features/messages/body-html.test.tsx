import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { inlineMentions } from "../../lib/body-html.ts";
import { BodyHtml } from "./body-html.tsx";

const HTML =
  '<p>Ship it, <action-text-attachment content-type="application/vnd.campfire.mention" sgid="x"><span class="mention" data-user-id="7">@Maya</span></action-text-attachment> &amp; friends</p>';

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
